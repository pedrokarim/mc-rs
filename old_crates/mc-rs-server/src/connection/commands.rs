use super::*;

impl ConnectionHandler {
    // -----------------------------------------------------------------------
    // Phase 1.4: Chat & Commands
    // -----------------------------------------------------------------------

    pub(super) async fn handle_text(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let text = match Text::proto_decode(buf) {
            Ok(t) => t,
            Err(e) => {
                warn!("Bad Text packet from {addr}: {e}");
                return;
            }
        };

        let sender_name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("<{sender_name}> {}", text.message);

        // Plugin event: PlayerChat (cancellable)
        if let Some(conn) = self.connections.get(&addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerChat {
                player,
                message: text.message.clone(),
            };
            let snapshot = self.build_snapshot();
            let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
            if result == EventResult::Cancelled {
                return;
            }
        }

        let response = Text::raw(format!("<{sender_name}> {}", text.message));
        self.send_packet(addr, packets::id::TEXT, &response).await;
    }

    pub(super) async fn handle_command_request(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        // Rate limit: commands
        {
            let last = self
                .connections
                .get(&addr)
                .map(|c| c.last_command_tick)
                .unwrap_or(0);
            if !self.check_rate_limit(addr, last, MIN_COMMAND_INTERVAL) {
                return;
            }
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.last_command_tick = self.game_world.current_tick();
            }
        }

        let request = match CommandRequest::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad CommandRequest from {addr}: {e}");
                return;
            }
        };

        let sender_name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("{sender_name} issued command: {}", request.command);

        // Strip the leading '/'
        let command_str = request
            .command
            .strip_prefix('/')
            .unwrap_or(&request.command);
        let mut parts = command_str.split_whitespace();
        let cmd_name = parts.next().unwrap_or("");
        let raw_args: Vec<String> = parts.map(String::from).collect();

        // Plugin event: PlayerCommand (cancellable)
        if let Some(conn) = self.connections.get(&addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerCommand {
                player,
                command: cmd_name.to_string(),
                args: raw_args.clone(),
            };
            let snapshot = self.build_snapshot();
            let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
            if result == EventResult::Cancelled {
                return;
            }
        }

        // Permission check for operator-only commands
        let needs_op = matches!(
            cmd_name,
            "gamemode"
                | "tp"
                | "give"
                | "kill"
                | "kick"
                | "op"
                | "deop"
                | "ban"
                | "ban-ip"
                | "unban"
                | "unban-ip"
                | "whitelist"
                | "summon"
                | "enchant"
                | "time"
                | "weather"
                | "gamerule"
                | "stop"
                | "reload"
                | "setblock"
                | "fill"
                | "clone"
                | "title"
                | "particle"
                | "playsound"
                | "scoreboard"
                | "tag"
                | "bossbar"
                | "execute"
                | "transfer"
                | "tickingarea"
                | "import"
                | "export"
        );
        if needs_op && !self.permissions.ops.contains(&sender_name) {
            let result = CommandResult::err("You do not have permission to use this command");
            let output = CommandOutput::failure(request.origin, result.messages.join("\n"));
            self.send_packet(addr, packets::id::COMMAND_OUTPUT, &output)
                .await;
            for msg in &result.messages {
                self.send_packet(addr, packets::id::TEXT, &Text::raw(msg))
                    .await;
            }
            return;
        }

        // Try server commands first (need &mut self access)
        let server_result = match cmd_name {
            "gamemode" => Some(self.cmd_gamemode(addr, &sender_name, &raw_args).await),
            "tp" => Some(self.cmd_tp(addr, &sender_name, &raw_args).await),
            "give" => Some(self.cmd_give(addr, &raw_args).await),
            "kill" => Some(self.cmd_kill(addr, &sender_name, &raw_args).await),
            "kick" => Some(self.cmd_kick(addr, &raw_args).await),
            "op" => Some(self.cmd_op(addr, &raw_args).await),
            "deop" => Some(self.cmd_deop(addr, &raw_args).await),
            "ban" => Some(self.cmd_ban(addr, &raw_args).await),
            "ban-ip" => Some(self.cmd_ban_ip(addr, &raw_args).await),
            "unban" => Some(self.cmd_unban(&raw_args)),
            "unban-ip" => Some(self.cmd_unban_ip(&raw_args)),
            "whitelist" => Some(self.cmd_whitelist(&raw_args)),
            "summon" => Some(self.cmd_summon(addr, &raw_args)),
            "effect" => Some(self.cmd_effect(addr, &sender_name, &raw_args).await),
            "enchant" => Some(self.cmd_enchant(addr, &sender_name, &raw_args).await),
            "time" => Some(self.cmd_time(addr, &raw_args).await),
            "weather" => Some(self.cmd_weather(addr, &raw_args).await),
            "gamerule" => Some(self.cmd_gamerule(addr, &raw_args).await),
            "reload" => Some(self.cmd_reload(addr).await),
            "setblock" => Some(self.cmd_setblock(addr, &raw_args).await),
            "fill" => Some(self.cmd_fill(addr, &raw_args).await),
            "clone" => Some(self.cmd_clone(addr, &raw_args).await),
            "title" => Some(self.cmd_title(addr, &sender_name, &raw_args).await),
            "particle" => Some(self.cmd_particle(addr, &raw_args).await),
            "playsound" => Some(self.cmd_playsound(addr, &sender_name, &raw_args).await),
            "scoreboard" => Some(self.cmd_scoreboard(addr, &raw_args).await),
            "tag" => Some(self.cmd_tag(addr, &sender_name, &raw_args).await),
            "bossbar" => Some(self.cmd_bossbar(addr, &raw_args).await),
            "execute" => Some(self.cmd_execute(addr, &sender_name, &raw_args).await),
            "transfer" => Some(self.cmd_transfer(addr, &sender_name, &raw_args).await),
            "tickingarea" => Some(self.cmd_tickingarea(&raw_args)),
            "import" => Some(self.cmd_import(&raw_args)),
            "export" => Some(self.cmd_export(&raw_args)),
            _ => None,
        };

        let result = if let Some(r) = server_result {
            r
        } else if self.plugin_manager.plugin_commands.contains_key(cmd_name) {
            // Plugin-registered command
            let snapshot = self.build_snapshot();
            let (response, actions) =
                self.plugin_manager
                    .handle_command(cmd_name, &raw_args, &sender_name, &snapshot);
            self.apply_plugin_actions(actions).await;
            CommandResult::ok(response.unwrap_or_default())
        } else {
            // Fall through to registry (help, list, say, stop)
            let args = match cmd_name {
                "help" => self
                    .command_registry
                    .get_commands()
                    .values()
                    .map(|e| format!("{}:{}", e.name, e.description))
                    .collect(),
                "list" => self
                    .connections
                    .values()
                    .filter(|c| c.state == LoginState::InGame)
                    .filter_map(|c| c.login_data.as_ref())
                    .map(|d| d.display_name.clone())
                    .collect(),
                _ => raw_args,
            };
            let ctx = mc_rs_command::CommandContext {
                sender_name: sender_name.clone(),
                args,
            };
            self.command_registry.execute(cmd_name, &ctx)
        };

        // Send CommandOutput
        let output = if result.success {
            CommandOutput::success(request.origin, result.messages.join("\n"))
        } else {
            CommandOutput::failure(request.origin, result.messages.join("\n"))
        };
        self.send_packet(addr, packets::id::COMMAND_OUTPUT, &output)
            .await;

        // Send result messages as chat text to the sender
        for msg in &result.messages {
            self.send_packet(addr, packets::id::TEXT, &Text::raw(msg))
                .await;
        }

        // Broadcast if requested
        if let Some(broadcast_msg) = &result.broadcast {
            let text = Text::raw(broadcast_msg);
            self.broadcast_packet(packets::id::TEXT, &text).await;
        }

        // Shutdown if requested
        if result.should_stop {
            info!("Server stop requested by {sender_name}");
            self.save_all();
            let _ = self.shutdown_tx.send(true);
        }
    }

    // -----------------------------------------------------------------------
    // Server commands (need &mut self for connections/state access)
    // -----------------------------------------------------------------------

    /// /gamemode <mode> [player]
    async fn cmd_gamemode(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /gamemode <mode> [player]");
        }

        let gamemode = match parse_gamemode(&args[0]) {
            Some(gm) => gm,
            None => return CommandResult::err(format!("Unknown gamemode: {}", args[0])),
        };

        let targets = if args.len() >= 2 {
            match self.resolve_target(&args[1], sender_addr) {
                Ok(t) => t,
                Err(e) => return CommandResult::err(e),
            }
        } else {
            vec![sender_name.to_string()]
        };

        let mode_name = gamemode_name(gamemode);
        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Update server state
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.gamemode = gamemode;
            }

            // Send SetPlayerGameType to the target
            self.send_packet(
                target_addr,
                packets::id::SET_PLAYER_GAME_TYPE,
                &SetPlayerGameType { gamemode },
            )
            .await;

            // Send UpdateAbilities to the target
            let (entity_unique_id, perm, cmd_perm) = {
                let conn = &self.connections[&target_addr];
                let is_op = self.permissions.ops.contains(target_name.as_str());
                (
                    conn.entity_unique_id,
                    if is_op { 2u8 } else { 1u8 },
                    if is_op { 1u8 } else { 0u8 },
                )
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: cmd_perm,
                    permission_level: perm,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("Set {target_name}'s game mode to {mode_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /tp — three forms:
    /// /tp <x> <y> <z>
    /// /tp <target> <x> <y> <z>
    /// /tp <target> <destination>
    async fn cmd_tp(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        match args.len() {
            3 => {
                // /tp <x> <y> <z> (self)
                let (x, y, z) = match parse_coords(&args[0], &args[1], &args[2]) {
                    Some(c) => c,
                    None => return CommandResult::err("Invalid coordinates"),
                };
                self.teleport_player(sender_addr, sender_name, x, y, z)
                    .await
            }
            2 => {
                // /tp <target> <destination_player>
                let targets = match self.resolve_target(&args[0], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                let dest_names = match self.resolve_target(&args[1], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                if dest_names.len() != 1 {
                    return CommandResult::err("Destination must be a single player");
                }
                let dest_pos = match self.find_player_addr(&dest_names[0]) {
                    Some(a) => self
                        .connections
                        .get(&a)
                        .map(|c| c.position)
                        .unwrap_or(Vec3::ZERO),
                    None => {
                        return CommandResult::err(format!("Player not found: {}", dest_names[0]))
                    }
                };

                let mut messages = Vec::new();
                for target_name in &targets {
                    let target_addr = match self.find_player_addr(target_name) {
                        Some(a) => a,
                        None => {
                            messages.push(format!("Player not found: {target_name}"));
                            continue;
                        }
                    };
                    self.teleport_player(
                        target_addr,
                        target_name,
                        dest_pos.x,
                        dest_pos.y,
                        dest_pos.z,
                    )
                    .await;
                    messages.push(format!("Teleported {target_name} to {}", dest_names[0]));
                }
                CommandResult {
                    success: true,
                    messages,
                    broadcast: None,
                    should_stop: false,
                }
            }
            4 => {
                // /tp <target> <x> <y> <z>
                let targets = match self.resolve_target(&args[0], sender_addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                let (x, y, z) = match parse_coords(&args[1], &args[2], &args[3]) {
                    Some(c) => c,
                    None => return CommandResult::err("Invalid coordinates"),
                };
                let mut messages = Vec::new();
                for target_name in &targets {
                    let target_addr = match self.find_player_addr(target_name) {
                        Some(a) => a,
                        None => {
                            messages.push(format!("Player not found: {target_name}"));
                            continue;
                        }
                    };
                    self.teleport_player(target_addr, target_name, x, y, z)
                        .await;
                    messages.push(format!(
                        "Teleported {target_name} to {x:.1}, {y:.1}, {z:.1}"
                    ));
                }
                CommandResult {
                    success: true,
                    messages,
                    broadcast: None,
                    should_stop: false,
                }
            }
            _ => CommandResult::err(
                "Usage: /tp <x> <y> <z> OR /tp <target> <x> <y> <z> OR /tp <target> <destination>",
            ),
        }
    }

    /// Perform the actual teleport for a single player.
    async fn teleport_player(
        &mut self,
        target_addr: SocketAddr,
        target_name: &str,
        x: f32,
        y: f32,
        z: f32,
    ) -> CommandResult {
        let (runtime_id, tick, uid) = match self.connections.get_mut(&target_addr) {
            Some(conn) => {
                conn.position = Vec3::new(x, y, z);
                conn.on_ground = false;
                (
                    conn.entity_runtime_id,
                    conn.client_tick,
                    conn.entity_unique_id,
                )
            }
            None => return CommandResult::err(format!("Player not found: {target_name}")),
        };

        // Sync position to ECS mirror entity
        self.game_world.update_player_position(uid, x, y, z);

        let pkt = MovePlayer {
            runtime_entity_id: runtime_id,
            position: Vec3::new(x, y, z),
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            mode: MoveMode::Teleport,
            on_ground: false,
            ridden_entity_runtime_id: 0,
            teleport_cause: Some(0),
            teleport_entity_type: Some(0),
            tick,
        };
        self.send_packet(target_addr, packets::id::MOVE_PLAYER, &pkt)
            .await;

        // Broadcast to other players so they see the teleport
        self.broadcast_packet_except(target_addr, packets::id::MOVE_PLAYER, &pkt)
            .await;

        CommandResult::ok(format!(
            "Teleported {target_name} to {x:.1}, {y:.1}, {z:.1}"
        ))
    }

    /// /give <player> <item> [amount] [metadata]
    async fn cmd_give(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err("Usage: /give <player> <item> [amount] [metadata]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        // Normalize item name: add "minecraft:" prefix if missing
        let item_name = if args[1].contains(':') {
            args[1].clone()
        } else {
            format!("minecraft:{}", args[1])
        };

        let item_info = match self.item_registry.get_by_name(&item_name) {
            Some(info) => info.clone(),
            None => return CommandResult::err(format!("Unknown item: {}", args[1])),
        };

        let amount = if args.len() >= 3 {
            match args[2].parse::<u16>() {
                Ok(a) if (1..=255).contains(&a) => a,
                _ => return CommandResult::err("Amount must be 1-255"),
            }
        } else {
            1
        };

        let metadata = if args.len() >= 4 {
            match args[3].parse::<u16>() {
                Ok(m) => m,
                _ => return CommandResult::err("Invalid metadata value"),
            }
        } else {
            0
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Find first empty slot in main inventory
            let slot = match self.connections.get(&target_addr) {
                Some(c) => c.inventory.main.iter().position(|s| s.is_empty()),
                None => continue,
            };

            let slot = match slot {
                Some(s) => s as u8,
                None => {
                    messages.push(format!("{target_name}'s inventory is full"));
                    continue;
                }
            };

            // Create the item
            let stack_id = match self.connections.get(&target_addr) {
                Some(c) => c.inventory.next_stack_network_id(),
                None => continue,
            };

            let item = mc_rs_proto::item_stack::ItemStack::new_with_meta(
                item_info.numeric_id as i32,
                amount,
                metadata,
                stack_id,
            );

            // Set in server inventory
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.inventory.set_slot(0, slot, item.clone());
            }

            // Send InventorySlot to client
            self.send_packet(
                target_addr,
                packets::id::INVENTORY_SLOT,
                &InventorySlot {
                    window_id: 0,
                    slot: slot as u32,
                    item,
                },
            )
            .await;

            messages.push(format!("Gave {amount} {} to {target_name}", item_info.name));
        }

        CommandResult {
            success: !messages.is_empty(),
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /kill [player] (default = self)
    async fn cmd_kill(
        &mut self,
        sender_addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        let targets = if args.is_empty() {
            vec![sender_name.to_string()]
        } else {
            match self.resolve_target(&args[0], sender_addr) {
                Ok(t) => t,
                Err(e) => return CommandResult::err(e),
            }
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            let runtime_id = match self.connections.get(&target_addr) {
                Some(conn) => conn.entity_runtime_id,
                None => continue,
            };

            // Set health to 0 and mark as dead
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.health = 0.0;
                conn.is_dead = true;
            }

            // Send health=0 to the victim
            let tick = self
                .connections
                .get(&target_addr)
                .map(|c| c.client_tick)
                .unwrap_or(0);
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ATTRIBUTES,
                &UpdateAttributes::health(runtime_id, 0.0, tick),
            )
            .await;

            // Broadcast death event
            self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(runtime_id))
                .await;

            // Send Respawn(searching) to trigger death screen
            let spawn_pos = self.spawn_position;
            self.send_packet(
                target_addr,
                packets::id::RESPAWN,
                &Respawn {
                    position: spawn_pos,
                    state: 0, // searching — shows death screen
                    runtime_entity_id: runtime_id,
                },
            )
            .await;

            messages.push(format!("Killed {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /kick <player> [reason]
    async fn cmd_kick(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /kick <player> [reason]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let reason = if args.len() >= 2 {
            args[1..].join(" ")
        } else {
            "Kicked by an operator".to_string()
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Send Disconnect packet — cleanup happens via handle_session_disconnected
            self.send_packet(
                target_addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(&reason),
            )
            .await;

            messages.push(format!("Kicked {target_name}: {reason}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /op <player>
    async fn cmd_op(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /op <player>");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.permissions.ops.insert(target_name.clone());
            self.permissions.save_ops();

            let (entity_unique_id, gamemode) = {
                let conn = &self.connections[&target_addr];
                (conn.entity_unique_id, conn.gamemode)
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: 1,
                    permission_level: 2,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("Opped {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /deop <player>
    async fn cmd_deop(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /deop <player>");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.permissions.ops.remove(target_name.as_str());
            self.permissions.save_ops();

            let (entity_unique_id, gamemode) = {
                let conn = &self.connections[&target_addr];
                (conn.entity_unique_id, conn.gamemode)
            };
            self.send_packet(
                target_addr,
                packets::id::UPDATE_ABILITIES,
                &UpdateAbilities {
                    command_permission_level: 0,
                    permission_level: 1,
                    entity_unique_id,
                    gamemode,
                },
            )
            .await;

            messages.push(format!("De-opped {target_name}"));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    async fn cmd_ban(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /ban <player> [reason]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "Banned by an operator".to_string()
        };

        let mut messages = Vec::new();

        for target_name in &targets {
            self.permissions.banned_players.insert(
                target_name.clone(),
                BanEntry {
                    reason: reason.clone(),
                },
            );

            // Kick the player if online
            if let Some(target_addr) = self.find_player_addr(target_name) {
                self.send_packet(
                    target_addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message(format!("You are banned: {reason}")),
                )
                .await;
            }

            messages.push(format!("Banned {target_name}: {reason}"));
        }

        self.permissions.save_banned_players();

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    async fn cmd_ban_ip(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /ban-ip <ip> [reason]");
        }

        let ip = &args[0];
        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "Banned by an operator".to_string()
        };

        self.permissions.banned_ips.insert(
            ip.clone(),
            BanEntry {
                reason: reason.clone(),
            },
        );
        self.permissions.save_banned_ips();

        // Kick all players connected from this IP
        let addrs_to_kick: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(a, c)| a.ip().to_string() == *ip && c.state == LoginState::InGame)
            .map(|(&a, _)| a)
            .collect();

        for kick_addr in &addrs_to_kick {
            self.send_packet(
                *kick_addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {reason}")),
            )
            .await;
        }

        let _ = sender_addr; // used for consistency with other commands
        CommandResult::ok(format!(
            "Banned IP {ip}: {reason} ({} player(s) kicked)",
            addrs_to_kick.len()
        ))
    }

    fn cmd_unban(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /unban <player>");
        }

        let name = &args[0];
        if self.permissions.banned_players.remove(name).is_some() {
            self.permissions.save_banned_players();
            CommandResult::ok(format!("Unbanned {name}"))
        } else {
            CommandResult::err(format!("{name} is not banned"))
        }
    }

    fn cmd_unban_ip(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /unban-ip <ip>");
        }

        let ip = &args[0];
        if self.permissions.banned_ips.remove(ip.as_str()).is_some() {
            self.permissions.save_banned_ips();
            CommandResult::ok(format!("Unbanned IP {ip}"))
        } else {
            CommandResult::err(format!("IP {ip} is not banned"))
        }
    }

    fn cmd_whitelist(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /whitelist <add|remove|list|on|off> [player]");
        }

        match args[0].as_str() {
            "add" => {
                let name = match args.get(1) {
                    Some(n) => n,
                    None => return CommandResult::err("Usage: /whitelist add <player>"),
                };
                self.permissions.whitelist.insert(name.clone());
                self.permissions.save_whitelist();
                CommandResult::ok(format!("Added {name} to the whitelist"))
            }
            "remove" => {
                let name = match args.get(1) {
                    Some(n) => n,
                    None => return CommandResult::err("Usage: /whitelist remove <player>"),
                };
                if self.permissions.whitelist.remove(name.as_str()) {
                    self.permissions.save_whitelist();
                    CommandResult::ok(format!("Removed {name} from the whitelist"))
                } else {
                    CommandResult::err(format!("{name} is not on the whitelist"))
                }
            }
            "list" => {
                let mut names: Vec<&String> = self.permissions.whitelist.iter().collect();
                names.sort();
                if names.is_empty() {
                    CommandResult::ok("Whitelist is empty")
                } else {
                    CommandResult::ok(format!(
                        "Whitelisted players ({}): {}",
                        names.len(),
                        names
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                }
            }
            "on" => {
                self.permissions.whitelist_enabled = true;
                CommandResult::ok("Whitelist enabled")
            }
            "off" => {
                self.permissions.whitelist_enabled = false;
                CommandResult::ok("Whitelist disabled")
            }
            other => CommandResult::err(format!(
                "Unknown whitelist action: {other}. Use add, remove, list, on, or off."
            )),
        }
    }

    /// /summon <entity_type> [x y z]
    fn cmd_summon(&mut self, sender_addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            let known: Vec<&str> = self
                .game_world
                .mob_registry
                .all()
                .iter()
                .map(|m| m.type_id.as_str())
                .collect();
            return CommandResult::err(format!(
                "Usage: /summon <type> [x y z]. Available: {}",
                known.join(", ")
            ));
        }

        let entity_type = &args[0];
        let full_type = if entity_type.contains(':') {
            entity_type.clone()
        } else {
            format!("minecraft:{entity_type}")
        };

        if self.game_world.mob_registry.get(&full_type).is_none() {
            let known: Vec<&str> = self
                .game_world
                .mob_registry
                .all()
                .iter()
                .map(|m| m.type_id.as_str())
                .collect();
            return CommandResult::err(format!(
                "Unknown entity type: {full_type}. Available: {}",
                known.join(", ")
            ));
        }

        let (x, y, z) = if args.len() >= 4 {
            match parse_coords(&args[1], &args[2], &args[3]) {
                Some(c) => c,
                None => return CommandResult::err("Invalid coordinates"),
            }
        } else {
            match self.connections.get(&sender_addr) {
                Some(c) => (c.position.x, c.position.y, c.position.z),
                None => return CommandResult::err("Sender not found"),
            }
        };

        match self.game_world.spawn_mob(&full_type, x, y, z) {
            Some(_) => {
                CommandResult::ok(format!("Summoned {full_type} at ({x:.1}, {y:.1}, {z:.1})"))
            }
            None => CommandResult::err(format!("Failed to summon {full_type}")),
        }
    }

    /// /effect <target> <effect> [amplifier] [duration_seconds]
    /// /effect <target> clear
    async fn cmd_effect(
        &mut self,
        sender_addr: SocketAddr,
        _sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err(
                "Usage: /effect <target> <effect> [amplifier] [duration] or /effect <target> clear",
            );
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        // /effect <target> clear
        if args[1] == "clear" {
            let mut messages = Vec::new();
            for target_name in &targets {
                let target_addr = match self.find_player_addr(target_name) {
                    Some(a) => a,
                    None => {
                        messages.push(format!("Player not found: {target_name}"));
                        continue;
                    }
                };
                self.clear_effects(target_addr).await;
                messages.push(format!("Cleared effects for {target_name}"));
            }
            return CommandResult {
                success: true,
                messages,
                broadcast: None,
                should_stop: false,
            };
        }

        // Parse effect name
        let effect_id = match effect_name_to_id(&args[1]) {
            Some(id) => id,
            None => {
                return CommandResult::err(format!(
                "Unknown effect: {}. Available: speed, slowness, strength, weakness, resistance, \
                     haste, mining_fatigue, jump_boost, nausea, regeneration, fire_resistance, \
                     water_breathing, invisibility, blindness, night_vision, hunger, poison, \
                     wither, absorption",
                args[1]
            ))
            }
        };

        let amplifier = if args.len() >= 3 {
            match args[2].parse::<i32>() {
                Ok(a) => a.clamp(0, 255),
                Err(_) => return CommandResult::err("Invalid amplifier (must be 0-255)"),
            }
        } else {
            0
        };

        let duration_secs = if args.len() >= 4 {
            match args[3].parse::<i32>() {
                Ok(d) if d > 0 => d,
                _ => return CommandResult::err("Invalid duration (must be > 0)"),
            }
        } else {
            30
        };
        let duration_ticks = duration_secs * 20;

        let mut messages = Vec::new();
        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            self.apply_effect(target_addr, effect_id, amplifier, duration_ticks)
                .await;
            messages.push(format!(
                "Applied {} {} to {target_name} for {duration_secs}s",
                args[1],
                amplifier + 1
            ));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    /// /enchant <target> <enchantment_name> [level]
    async fn cmd_enchant(
        &mut self,
        sender_addr: SocketAddr,
        _sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err("Usage: /enchant <target> <enchantment> [level]");
        }

        let targets = match self.resolve_target(&args[0], sender_addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let ench_name = &args[1];
        let info = match game_combat::enchantment_by_name(ench_name) {
            Some(i) => i,
            None => {
                return CommandResult::err(format!("Unknown enchantment: {ench_name}"));
            }
        };

        let level: i16 = if args.len() >= 3 {
            match args[2].parse::<i16>() {
                Ok(l) if l >= 1 => l.min(info.max_level),
                _ => return CommandResult::err("Invalid level (must be >= 1)"),
            }
        } else {
            1
        };

        let mut messages = Vec::new();
        for target_name in &targets {
            let target_addr = match self.find_player_addr(target_name) {
                Some(a) => a,
                None => {
                    messages.push(format!("Player not found: {target_name}"));
                    continue;
                }
            };

            // Get current enchantments on held item
            let old_nbt = match self.connections.get(&target_addr) {
                Some(c) => {
                    let held = c.inventory.held_item();
                    if held.runtime_id == 0 {
                        messages.push(format!("{target_name} is not holding an item"));
                        continue;
                    }
                    held.nbt_data.clone()
                }
                None => continue,
            };

            // Parse existing enchantments, update/add the new one
            let mut enchants = game_combat::parse_enchantments(&old_nbt);
            if let Some(existing) = enchants.iter_mut().find(|e| e.id == info.id) {
                existing.level = level;
            } else {
                enchants.push(game_combat::Enchantment { id: info.id, level });
            }

            // Rebuild NBT
            let new_nbt = game_combat::build_enchantment_nbt(&enchants);

            // Apply to held item
            if let Some(conn) = self.connections.get_mut(&target_addr) {
                conn.inventory.held_item_mut().nbt_data = new_nbt;
            }

            // Send updated inventory
            self.send_inventory(target_addr).await;

            messages.push(format!(
                "Applied {} {} to {target_name}'s held item",
                info.name, level
            ));
        }

        CommandResult {
            success: true,
            messages,
            broadcast: None,
            should_stop: false,
        }
    }

    // -----------------------------------------------------------------------
    // /time command
    // -----------------------------------------------------------------------

    async fn cmd_time(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /time set <value>|day|noon|night|midnight|sunrise|sunset\n/time add <value>\n/time query daytime|gametime");
        }

        match args[0].as_str() {
            "set" => {
                if args.len() < 2 {
                    return CommandResult::err(
                        "Usage: /time set <value>|day|noon|night|midnight|sunrise|sunset",
                    );
                }
                let time = match args[1].as_str() {
                    "day" | "sunrise" => 23000_i64,
                    "noon" => 6000,
                    "sunset" => 12000,
                    "night" => 13000,
                    "midnight" => 18000,
                    v => match v.parse::<i64>() {
                        Ok(t) => t,
                        Err(_) => return CommandResult::err(format!("Invalid time: {v}")),
                    },
                };
                // Plugin event: TimeChange (cancellable)
                {
                    let event = PluginEvent::TimeChange { new_time: time };
                    let snapshot = self.build_snapshot();
                    let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                    self.apply_plugin_actions(actions).await;
                    if result == EventResult::Cancelled {
                        return CommandResult::ok("Time change cancelled by plugin".to_string());
                    }
                }
                self.world_time = time;
                let pkt = SetTime {
                    time: self.world_time as i32,
                };
                self.broadcast_packet(packets::id::SET_TIME, &pkt).await;
                CommandResult::ok(format!("Set the time to {}", self.world_time))
            }
            "add" => {
                if args.len() < 2 {
                    return CommandResult::err("Usage: /time add <value>");
                }
                let amount: i64 = match args[1].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err(format!("Invalid amount: {}", args[1])),
                };
                let new_time = self.world_time + amount;
                // Plugin event: TimeChange (cancellable)
                {
                    let event = PluginEvent::TimeChange { new_time };
                    let snapshot = self.build_snapshot();
                    let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                    self.apply_plugin_actions(actions).await;
                    if result == EventResult::Cancelled {
                        return CommandResult::ok("Time change cancelled by plugin".to_string());
                    }
                }
                self.world_time = new_time;
                let pkt = SetTime {
                    time: self.world_time as i32,
                };
                self.broadcast_packet(packets::id::SET_TIME, &pkt).await;
                CommandResult::ok(format!(
                    "Added {} to the time (now {})",
                    amount, self.world_time
                ))
            }
            "query" => {
                if args.len() < 2 {
                    return CommandResult::err("Usage: /time query daytime|gametime");
                }
                match args[1].as_str() {
                    "daytime" => {
                        let daytime = self.world_time % 24000;
                        CommandResult::ok(format!("The time is {daytime}"))
                    }
                    "gametime" => {
                        let total = self.game_world.current_tick();
                        CommandResult::ok(format!("The game time is {total}"))
                    }
                    _ => CommandResult::err(format!("Unknown query: {}", args[1])),
                }
            }
            _ => CommandResult::err(format!("Unknown subcommand: {}", args[0])),
        }
    }

    // -----------------------------------------------------------------------
    // /weather command
    // -----------------------------------------------------------------------

    async fn cmd_weather(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /weather clear|rain|thunder [duration_secs]");
        }

        let duration_secs: u64 = if args.len() >= 2 {
            match args[1].parse() {
                Ok(v) => v,
                Err(_) => return CommandResult::err(format!("Invalid duration: {}", args[1])),
            }
        } else {
            300 // default 5 minutes
        };
        let duration_ticks = (duration_secs * 20) as i32;

        match args[0].as_str() {
            "clear" => {
                if self.is_raining {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::stop_rain())
                        .await;
                }
                if self.is_thundering {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::stop_thunder())
                        .await;
                }
                self.rain_target = 0.0;
                self.lightning_target = 0.0;
                self.is_raining = false;
                self.is_thundering = false;
                self.weather_duration = duration_ticks;
                CommandResult::ok(format!("Set the weather to clear for {duration_secs}s"))
            }
            "rain" => {
                if !self.is_raining {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::start_rain())
                        .await;
                }
                if self.is_thundering {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::stop_thunder())
                        .await;
                }
                self.rain_target = 1.0;
                self.lightning_target = 0.0;
                self.is_raining = true;
                self.is_thundering = false;
                self.weather_duration = duration_ticks;
                CommandResult::ok(format!("Set the weather to rain for {duration_secs}s"))
            }
            "thunder" => {
                if !self.is_raining {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::start_rain())
                        .await;
                }
                if !self.is_thundering {
                    self.broadcast_packet(packets::id::LEVEL_EVENT, &LevelEvent::start_thunder())
                        .await;
                }
                self.rain_target = 1.0;
                self.lightning_target = 1.0;
                self.is_raining = true;
                self.is_thundering = true;
                self.weather_duration = duration_ticks;
                CommandResult::ok(format!("Set the weather to thunder for {duration_secs}s"))
            }
            _ => CommandResult::err(format!("Unknown weather type: {}", args[0])),
        }
    }

    // -----------------------------------------------------------------------
    // /gamerule command
    // -----------------------------------------------------------------------

    async fn cmd_gamerule(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err(
                "Usage: /gamerule <rule> [value]\nAvailable: doDaylightCycle, doWeatherCycle",
            );
        }

        let rule_name = &args[0];
        let canonical = rule_name.to_lowercase();

        if args.len() == 1 {
            // Query mode
            match canonical.as_str() {
                "dodaylightcycle" => {
                    CommandResult::ok(format!("doDaylightCycle = {}", self.do_daylight_cycle))
                }
                "doweathercycle" => {
                    CommandResult::ok(format!("doWeatherCycle = {}", self.do_weather_cycle))
                }
                _ => CommandResult::err(format!("Unknown game rule: {rule_name}")),
            }
        } else {
            // Set mode
            let value_str = &args[1];
            let value = match value_str.to_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return CommandResult::err(format!(
                        "Invalid value: {value_str} (expected true/false)"
                    ))
                }
            };

            match canonical.as_str() {
                "dodaylightcycle" => {
                    self.do_daylight_cycle = value;
                    let pkt = GameRulesChanged {
                        rules: vec![GameRule {
                            name: "dodaylightcycle".into(),
                            editable: false,
                            value: GameRuleValue::Bool(value),
                        }],
                    };
                    self.broadcast_packet(packets::id::GAME_RULES_CHANGED, &pkt)
                        .await;
                    CommandResult::ok(format!("Game rule doDaylightCycle set to {value}"))
                }
                "doweathercycle" => {
                    self.do_weather_cycle = value;
                    let pkt = GameRulesChanged {
                        rules: vec![GameRule {
                            name: "doweathercycle".into(),
                            editable: false,
                            value: GameRuleValue::Bool(value),
                        }],
                    };
                    self.broadcast_packet(packets::id::GAME_RULES_CHANGED, &pkt)
                        .await;
                    CommandResult::ok(format!("Game rule doWeatherCycle set to {value}"))
                }
                _ => CommandResult::err(format!("Unknown game rule: {rule_name}")),
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 4.2/4.3: /reload (hot-reload plugins)
    // -----------------------------------------------------------------------

    async fn cmd_reload(&mut self, _addr: SocketAddr) -> CommandResult {
        self.plugin_manager.reload();

        let plugins_dir = std::path::PathBuf::from("plugins");
        std::fs::create_dir_all(&plugins_dir).ok();
        let engine = mc_rs_plugin_wasm::create_engine();
        let wasm_plugins = mc_rs_plugin_wasm::load_wasm_plugins(&plugins_dir, &engine);
        for plugin in wasm_plugins {
            self.plugin_manager.register(plugin);
        }
        let lua_plugins = mc_rs_plugin_lua::load_lua_plugins(&plugins_dir);
        for plugin in lua_plugins {
            self.plugin_manager.register(plugin);
        }
        let snapshot = self.build_snapshot();
        self.plugin_manager.enable_all(&snapshot);
        self.plugin_manager.load_configs();

        let cmd_count = self.plugin_manager.plugin_commands.len();
        CommandResult::ok(format!(
            "Plugins reloaded. {cmd_count} plugin command(s) registered."
        ))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /setblock
    // -----------------------------------------------------------------------

    async fn cmd_setblock(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 4 {
            return CommandResult::err(
                "Usage: /setblock <x> <y> <z> <block> [replace|destroy|keep]",
            );
        }
        let x: i32 = match args[0].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid x coordinate"),
        };
        let y: i32 = match args[1].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid y coordinate"),
        };
        let z: i32 = match args[2].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid z coordinate"),
        };

        let block_name = if args[3].contains(':') {
            args[3].clone()
        } else {
            format!("minecraft:{}", args[3])
        };
        let rid = hash_block_state(&block_name);
        if rid == self.tick_blocks.air && block_name != "minecraft:air" {
            return CommandResult::err(format!("Unknown block: {}", args[3]));
        }

        let mode = args.get(4).map(|s| s.as_str()).unwrap_or("replace");

        match mode {
            "keep" => {
                if let Some(current) = self.get_block(x, y, z) {
                    if current != self.tick_blocks.air {
                        return CommandResult::err("Block is not air (keep mode)");
                    }
                }
            }
            "destroy" => {
                if let Some(current) = self.get_block(x, y, z) {
                    if current != self.tick_blocks.air {
                        let pkt = LevelEvent::destroy_block(x, y, z, current);
                        self.broadcast_packet(packets::id::LEVEL_EVENT, &pkt).await;
                    }
                }
            }
            "replace" => {}
            _ => return CommandResult::err("Mode must be replace, destroy, or keep"),
        }

        self.set_block_and_broadcast(x, y, z, rid).await;
        self.schedule_fluid_neighbors(x, y, z);
        self.schedule_piston_neighbors(x, y, z);
        self.update_redstone_from(x, y, z).await;

        CommandResult::ok(format!("Set block at {x} {y} {z} to {block_name}"))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /fill
    // -----------------------------------------------------------------------

    async fn cmd_fill(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 7 {
            return CommandResult::err(
                "Usage: /fill <x1> <y1> <z1> <x2> <y2> <z2> <block> [replace|destroy|hollow|outline|keep]",
            );
        }
        let coords: Vec<i32> = match args[..6].iter().map(|s| s.parse()).collect() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid coordinates"),
        };
        let (x1, y1, z1) = (
            coords[0].min(coords[3]),
            coords[1].min(coords[4]),
            coords[2].min(coords[5]),
        );
        let (x2, y2, z2) = (
            coords[0].max(coords[3]),
            coords[1].max(coords[4]),
            coords[2].max(coords[5]),
        );

        let volume = (x2 - x1 + 1) as u64 * (y2 - y1 + 1) as u64 * (z2 - z1 + 1) as u64;
        if volume > 32768 {
            return CommandResult::err(format!("Too many blocks ({volume}). Maximum is 32768."));
        }

        let block_name = if args[6].contains(':') {
            args[6].clone()
        } else {
            format!("minecraft:{}", args[6])
        };
        let rid = hash_block_state(&block_name);
        if rid == self.tick_blocks.air && block_name != "minecraft:air" {
            return CommandResult::err(format!("Unknown block: {}", args[6]));
        }

        let mode = args.get(7).map(|s| s.as_str()).unwrap_or("replace");
        let air = self.tick_blocks.air;
        let mut count = 0u64;

        for bx in x1..=x2 {
            for by in y1..=y2 {
                for bz in z1..=z2 {
                    let on_edge =
                        bx == x1 || bx == x2 || by == y1 || by == y2 || bz == z1 || bz == z2;

                    let should_set = match mode {
                        "replace" => true,
                        "destroy" => {
                            if let Some(current) = self.get_block(bx, by, bz) {
                                if current != air {
                                    let pkt = LevelEvent::destroy_block(bx, by, bz, current);
                                    self.broadcast_packet(packets::id::LEVEL_EVENT, &pkt).await;
                                }
                            }
                            true
                        }
                        "keep" => {
                            matches!(self.get_block(bx, by, bz), Some(b) if b == air)
                        }
                        "hollow" => {
                            if on_edge {
                                true
                            } else {
                                // Fill interior with air
                                self.set_block_and_broadcast(bx, by, bz, air).await;
                                count += 1;
                                false
                            }
                        }
                        "outline" => on_edge,
                        _ => return CommandResult::err("Invalid mode"),
                    };

                    if should_set {
                        self.set_block_and_broadcast(bx, by, bz, rid).await;
                        count += 1;
                    }
                }
            }
        }

        CommandResult::ok(format!("{count} block(s) filled"))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /clone
    // -----------------------------------------------------------------------

    async fn cmd_clone(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 9 {
            return CommandResult::err(
                "Usage: /clone <x1> <y1> <z1> <x2> <y2> <z2> <dx> <dy> <dz> [replace|masked]",
            );
        }
        let coords: Vec<i32> = match args[..9].iter().map(|s| s.parse()).collect() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid coordinates"),
        };
        let (sx1, sy1, sz1) = (
            coords[0].min(coords[3]),
            coords[1].min(coords[4]),
            coords[2].min(coords[5]),
        );
        let (sx2, sy2, sz2) = (
            coords[0].max(coords[3]),
            coords[1].max(coords[4]),
            coords[2].max(coords[5]),
        );
        let (dx, dy, dz) = (coords[6], coords[7], coords[8]);

        let volume = (sx2 - sx1 + 1) as u64 * (sy2 - sy1 + 1) as u64 * (sz2 - sz1 + 1) as u64;
        if volume > 32768 {
            return CommandResult::err(format!("Too many blocks ({volume}). Maximum is 32768."));
        }

        let masked = args.get(9).map(|s| s.as_str()) == Some("masked");
        let air = self.tick_blocks.air;

        // Snapshot source region
        let mut blocks = Vec::new();
        for bx in sx1..=sx2 {
            for by in sy1..=sy2 {
                for bz in sz1..=sz2 {
                    let rid = self.get_block(bx, by, bz).unwrap_or(air);
                    if masked && rid == air {
                        continue;
                    }
                    blocks.push((bx - sx1, by - sy1, bz - sz1, rid));
                }
            }
        }

        // Paste at destination
        let mut count = 0u64;
        for (ox, oy, oz, rid) in &blocks {
            self.set_block_and_broadcast(dx + ox, dy + oy, dz + oz, *rid)
                .await;
            count += 1;
        }

        CommandResult::ok(format!("{count} block(s) cloned"))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /title
    // -----------------------------------------------------------------------

    async fn cmd_title(
        &mut self,
        addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err(
                "Usage: /title <target> <clear|reset|title|subtitle|actionbar|times> [text] [fadeIn stay fadeOut]",
            );
        }

        let targets = match self.resolve_target(&args[0], addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let action = args[1].as_str();
        let pkt = match action {
            "clear" => SetTitle::clear(),
            "reset" => SetTitle::reset(),
            "title" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /title <target> title <text>");
                }
                SetTitle::title(args[2..].join(" "))
            }
            "subtitle" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /title <target> subtitle <text>");
                }
                SetTitle::subtitle(args[2..].join(" "))
            }
            "actionbar" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /title <target> actionbar <text>");
                }
                SetTitle::actionbar(args[2..].join(" "))
            }
            "times" => {
                if args.len() < 5 {
                    return CommandResult::err(
                        "Usage: /title <target> times <fadeIn> <stay> <fadeOut>",
                    );
                }
                let fade_in: i32 = args[2].parse().unwrap_or(10);
                let stay: i32 = args[3].parse().unwrap_or(70);
                let fade_out: i32 = args[4].parse().unwrap_or(20);
                SetTitle::times(fade_in, stay, fade_out)
            }
            _ => {
                return CommandResult::err(
                    "Action must be clear, reset, title, subtitle, actionbar, or times",
                )
            }
        };

        let target_addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(_, c)| {
                c.state == LoginState::InGame
                    && c.login_data
                        .as_ref()
                        .map(|d| targets.contains(&d.display_name))
                        .unwrap_or(false)
            })
            .map(|(a, _)| *a)
            .collect();

        for target_addr in &target_addrs {
            self.send_packet(*target_addr, packets::id::SET_TITLE, &pkt)
                .await;
        }

        let _ = sender_name; // used for resolve_target context
        CommandResult::ok(format!(
            "Title {action} sent to {} player(s)",
            target_addrs.len()
        ))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /particle
    // -----------------------------------------------------------------------

    async fn cmd_particle(&mut self, _addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.len() < 4 {
            return CommandResult::err("Usage: /particle <name> <x> <y> <z>");
        }
        let particle_name = if args[0].contains(':') {
            args[0].clone()
        } else {
            format!("minecraft:{}", args[0])
        };
        let x: f32 = match args[1].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid x coordinate"),
        };
        let y: f32 = match args[2].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid y coordinate"),
        };
        let z: f32 = match args[3].parse() {
            Ok(v) => v,
            Err(_) => return CommandResult::err("Invalid z coordinate"),
        };

        let pkt = SpawnParticleEffect {
            dimension_id: self.dimension_id as u8,
            entity_unique_id: -1,
            position: Vec3::new(x, y, z),
            particle_name: particle_name.clone(),
        };
        self.broadcast_packet(packets::id::SPAWN_PARTICLE_EFFECT, &pkt)
            .await;
        CommandResult::ok(format!("Spawned particle {particle_name} at {x} {y} {z}"))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /playsound
    // -----------------------------------------------------------------------

    async fn cmd_playsound(
        &mut self,
        addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err(
                "Usage: /playsound <sound> <target> [x] [y] [z] [volume] [pitch]",
            );
        }
        let sound_name = args[0].clone();
        let targets = match self.resolve_target(&args[1], addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let volume: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let pitch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(1.0);

        let _ = sender_name;
        let target_addrs: Vec<(SocketAddr, Vec3)> = self
            .connections
            .iter()
            .filter(|(_, c)| {
                c.state == LoginState::InGame
                    && c.login_data
                        .as_ref()
                        .map(|d| targets.contains(&d.display_name))
                        .unwrap_or(false)
            })
            .map(|(a, c)| (*a, c.position))
            .collect();

        for (target_addr, target_pos) in &target_addrs {
            let x: f32 = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(target_pos.x);
            let y: f32 = args
                .get(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(target_pos.y);
            let z: f32 = args
                .get(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(target_pos.z);

            let pkt = PlaySound::new(&sound_name, x, y, z, volume, pitch);
            self.send_packet(*target_addr, packets::id::PLAY_SOUND, &pkt)
                .await;
        }

        CommandResult::ok(format!(
            "Played {} to {} player(s)",
            sound_name,
            target_addrs.len()
        ))
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /scoreboard
    // -----------------------------------------------------------------------

    async fn cmd_scoreboard(&mut self, addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err(
                "Usage: /scoreboard <objectives|players> <subcommand> [args...]",
            );
        }

        match args[0].as_str() {
            "objectives" => self.cmd_scoreboard_objectives(addr, &args[1..]).await,
            "players" => self.cmd_scoreboard_players(addr, &args[1..]).await,
            _ => CommandResult::err("Usage: /scoreboard <objectives|players>"),
        }
    }

    async fn cmd_scoreboard_objectives(
        &mut self,
        _addr: SocketAddr,
        args: &[String],
    ) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err(
                "Usage: /scoreboard objectives <add|remove|list|setdisplay>",
            );
        }
        match args[0].as_str() {
            "add" => {
                if args.len() < 3 {
                    return CommandResult::err(
                        "Usage: /scoreboard objectives add <name> <criteria> [displayName]",
                    );
                }
                let name = args[1].clone();
                let criteria = args[2].clone();
                let display_name = if args.len() > 3 {
                    args[3..].join(" ")
                } else {
                    name.clone()
                };
                if self.scoreboard_objectives.contains_key(&name) {
                    return CommandResult::err(format!("Objective '{name}' already exists"));
                }
                self.scoreboard_objectives
                    .insert(name.clone(), (display_name, criteria));
                self.scoreboard_scores.entry(name.clone()).or_default();
                CommandResult::ok(format!("Added objective '{name}'"))
            }
            "remove" => {
                if args.len() < 2 {
                    return CommandResult::err("Usage: /scoreboard objectives remove <name>");
                }
                let name = &args[1];
                if self.scoreboard_objectives.remove(name).is_none() {
                    return CommandResult::err(format!("Objective '{name}' not found"));
                }
                self.scoreboard_scores.remove(name);
                // Remove from any display slots
                self.scoreboard_displays.retain(|_, v| v != name);
                // Clear display on clients
                for slot in &["sidebar", "list", "belowname"] {
                    let pkt = SetDisplayObjective::clear(*slot);
                    self.broadcast_packet(packets::id::SET_DISPLAY_OBJECTIVE, &pkt)
                        .await;
                }
                CommandResult::ok(format!("Removed objective '{name}'"))
            }
            "list" => {
                if self.scoreboard_objectives.is_empty() {
                    return CommandResult::ok("No objectives defined");
                }
                let list: Vec<String> = self
                    .scoreboard_objectives
                    .iter()
                    .map(|(name, (display, criteria))| format!("- {name} ({criteria}): {display}"))
                    .collect();
                CommandResult::ok(format!("{} objective(s):\n{}", list.len(), list.join("\n")))
            }
            "setdisplay" => {
                if args.len() < 2 {
                    return CommandResult::err(
                        "Usage: /scoreboard objectives setdisplay <slot> [objective]",
                    );
                }
                let slot = args[1].clone();
                if !["sidebar", "list", "belowname"].contains(&slot.as_str()) {
                    return CommandResult::err("Slot must be sidebar, list, or belowname");
                }

                if args.len() < 3 {
                    // Clear slot
                    self.scoreboard_displays.remove(&slot);
                    let pkt = SetDisplayObjective::clear(&slot);
                    self.broadcast_packet(packets::id::SET_DISPLAY_OBJECTIVE, &pkt)
                        .await;
                    return CommandResult::ok(format!("Cleared display slot '{slot}'"));
                }

                let obj_name = &args[2];
                let (display_name, criteria) = match self.scoreboard_objectives.get(obj_name) {
                    Some(v) => v.clone(),
                    None => return CommandResult::err(format!("Objective '{obj_name}' not found")),
                };
                self.scoreboard_displays
                    .insert(slot.clone(), obj_name.clone());
                let pkt = SetDisplayObjective {
                    display_slot: slot.clone(),
                    objective_name: obj_name.clone(),
                    display_name,
                    criteria,
                    sort_order: 1,
                };
                self.broadcast_packet(packets::id::SET_DISPLAY_OBJECTIVE, &pkt)
                    .await;

                // Send existing scores for this objective
                self.send_scoreboard_scores(obj_name).await;

                CommandResult::ok(format!("Set display slot '{slot}' to '{obj_name}'"))
            }
            _ => CommandResult::err("Usage: /scoreboard objectives <add|remove|list|setdisplay>"),
        }
    }

    async fn cmd_scoreboard_players(
        &mut self,
        _addr: SocketAddr,
        args: &[String],
    ) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /scoreboard players <set|add|remove|list|reset>");
        }
        match args[0].as_str() {
            "set" => {
                if args.len() < 4 {
                    return CommandResult::err(
                        "Usage: /scoreboard players set <player> <objective> <score>",
                    );
                }
                let player = &args[1];
                let obj = &args[2];
                let score: i32 = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid score value"),
                };
                if !self.scoreboard_objectives.contains_key(obj) {
                    return CommandResult::err(format!("Objective '{obj}' not found"));
                }
                self.scoreboard_scores
                    .entry(obj.clone())
                    .or_default()
                    .insert(player.clone(), score);
                self.send_scoreboard_scores(obj).await;
                CommandResult::ok(format!("Set {player}'s {obj} to {score}"))
            }
            "add" => {
                if args.len() < 4 {
                    return CommandResult::err(
                        "Usage: /scoreboard players add <player> <objective> <count>",
                    );
                }
                let player = &args[1];
                let obj = &args[2];
                let count: i32 = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid count value"),
                };
                if !self.scoreboard_objectives.contains_key(obj) {
                    return CommandResult::err(format!("Objective '{obj}' not found"));
                }
                let entry = self
                    .scoreboard_scores
                    .entry(obj.clone())
                    .or_default()
                    .entry(player.clone())
                    .or_insert(0);
                *entry += count;
                let new_score = *entry;
                self.send_scoreboard_scores(obj).await;
                CommandResult::ok(format!(
                    "Added {count} to {player}'s {obj} (now {new_score})"
                ))
            }
            "remove" => {
                if args.len() < 4 {
                    return CommandResult::err(
                        "Usage: /scoreboard players remove <player> <objective> <count>",
                    );
                }
                let player = &args[1];
                let obj = &args[2];
                let count: i32 = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid count value"),
                };
                if !self.scoreboard_objectives.contains_key(obj) {
                    return CommandResult::err(format!("Objective '{obj}' not found"));
                }
                let entry = self
                    .scoreboard_scores
                    .entry(obj.clone())
                    .or_default()
                    .entry(player.clone())
                    .or_insert(0);
                *entry -= count;
                let new_score = *entry;
                self.send_scoreboard_scores(obj).await;
                CommandResult::ok(format!(
                    "Removed {count} from {player}'s {obj} (now {new_score})"
                ))
            }
            "list" => {
                if args.len() < 2 {
                    // List all players with scores
                    let mut all_players: HashSet<String> = HashSet::new();
                    for scores in self.scoreboard_scores.values() {
                        all_players.extend(scores.keys().cloned());
                    }
                    if all_players.is_empty() {
                        return CommandResult::ok("No tracked players");
                    }
                    let list: Vec<String> = all_players.into_iter().collect();
                    return CommandResult::ok(format!(
                        "{} tracked player(s): {}",
                        list.len(),
                        list.join(", ")
                    ));
                }
                let player = &args[1];
                let mut lines = Vec::new();
                for (obj, scores) in &self.scoreboard_scores {
                    if let Some(score) = scores.get(player) {
                        lines.push(format!("- {obj}: {score}"));
                    }
                }
                if lines.is_empty() {
                    CommandResult::ok(format!("{player} has no scores"))
                } else {
                    CommandResult::ok(format!("{player}'s scores:\n{}", lines.join("\n")))
                }
            }
            "reset" => {
                if args.len() < 2 {
                    return CommandResult::err(
                        "Usage: /scoreboard players reset <player> [objective]",
                    );
                }
                let player = &args[1];
                if args.len() >= 3 {
                    let obj = &args[2];
                    if let Some(scores) = self.scoreboard_scores.get_mut(obj) {
                        scores.remove(player);
                    }
                    self.send_scoreboard_scores(obj).await;
                    CommandResult::ok(format!("Reset {player}'s {obj}"))
                } else {
                    for scores in self.scoreboard_scores.values_mut() {
                        scores.remove(player);
                    }
                    // Refresh all displayed objectives
                    let displayed: Vec<String> =
                        self.scoreboard_displays.values().cloned().collect();
                    for obj in displayed {
                        self.send_scoreboard_scores(&obj).await;
                    }
                    CommandResult::ok(format!("Reset all scores for {player}"))
                }
            }
            _ => CommandResult::err("Usage: /scoreboard players <set|add|remove|list|reset>"),
        }
    }

    /// Send all scores for an objective to all clients via SetScore packet.
    async fn send_scoreboard_scores(&mut self, objective: &str) {
        let scores = match self.scoreboard_scores.get(objective) {
            Some(s) => s,
            None => return,
        };
        let mut entries = Vec::new();
        let mut entry_id = self.next_score_entry_id;
        for (player_name, score) in scores {
            entries.push(ScoreEntry {
                entry_id,
                objective_name: objective.to_string(),
                score: *score,
                identity_type: packets::set_score::IDENTITY_FAKE_PLAYER,
                custom_name: player_name.clone(),
            });
            entry_id += 1;
        }
        self.next_score_entry_id = entry_id;

        let pkt = SetScore::change(entries);
        self.broadcast_packet(packets::id::SET_SCORE, &pkt).await;
    }

    // -----------------------------------------------------------------------
    // Phase 5.2: /tag
    // -----------------------------------------------------------------------

    async fn cmd_tag(
        &mut self,
        addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err("Usage: /tag <target> <add|remove|list> [tag]");
        }

        let targets = match self.resolve_target(&args[0], addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let _ = sender_name;
        let action = args[1].as_str();

        match action {
            "add" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /tag <target> add <tag>");
                }
                let tag = args[2].clone();
                let mut count = 0;
                for conn in self.connections.values_mut() {
                    if conn.state == LoginState::InGame {
                        if let Some(name) = conn.login_data.as_ref().map(|d| &d.display_name) {
                            if targets.contains(name) {
                                conn.tags.insert(tag.clone());
                                count += 1;
                            }
                        }
                    }
                }
                CommandResult::ok(format!("Added tag '{tag}' to {count} player(s)"))
            }
            "remove" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /tag <target> remove <tag>");
                }
                let tag = &args[2];
                let mut count = 0;
                for conn in self.connections.values_mut() {
                    if conn.state == LoginState::InGame {
                        if let Some(name) = conn.login_data.as_ref().map(|d| &d.display_name) {
                            if targets.contains(name) && conn.tags.remove(tag) {
                                count += 1;
                            }
                        }
                    }
                }
                CommandResult::ok(format!("Removed tag '{tag}' from {count} player(s)"))
            }
            "list" => {
                let mut msgs = Vec::new();
                for conn in self.connections.values() {
                    if conn.state == LoginState::InGame {
                        if let Some(name) = conn.login_data.as_ref().map(|d| &d.display_name) {
                            if targets.contains(name) {
                                if conn.tags.is_empty() {
                                    msgs.push(format!("{name}: no tags"));
                                } else {
                                    let tags: Vec<&String> = conn.tags.iter().collect();
                                    msgs.push(format!(
                                        "{name}: {}",
                                        tags.iter()
                                            .map(|t| t.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ));
                                }
                            }
                        }
                    }
                }
                CommandResult::ok(msgs.join("\n"))
            }
            _ => CommandResult::err("Action must be add, remove, or list"),
        }
    }

    // -----------------------------------------------------------------------
    // Phase 5.2 (fin): /bossbar, /execute, /transfer, /tickingarea
    // -----------------------------------------------------------------------

    async fn cmd_bossbar(&mut self, addr: SocketAddr, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /bossbar <add|remove|set|players|list> [id] [...]");
        }
        let sub = args[0].as_str();
        match sub {
            "add" => {
                if args.len() < 3 {
                    return CommandResult::err("Usage: /bossbar add <id> <title>");
                }
                let id = args[1].clone();
                let title = args[2..].join(" ");
                if self.boss_bars.contains_key(&id) {
                    return CommandResult::err(format!("Boss bar '{id}' already exists"));
                }
                let boss_id = self.next_score_entry_id;
                self.next_score_entry_id += 1;
                self.boss_bars.insert(
                    id.clone(),
                    super::BossBarData {
                        title,
                        color: 0,
                        max: 100.0,
                        value: 100.0,
                        visible: true,
                        players: HashSet::new(),
                        boss_id,
                    },
                );
                CommandResult::ok(format!("Created boss bar '{id}'"))
            }
            "remove" => {
                if args.len() < 2 {
                    return CommandResult::err("Usage: /bossbar remove <id>");
                }
                let id = &args[1];
                if let Some(bar) = self.boss_bars.remove(id) {
                    let pkt = BossEvent::hide(bar.boss_id);
                    for &player_addr in &bar.players {
                        self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                            .await;
                    }
                    CommandResult::ok(format!("Removed boss bar '{id}'"))
                } else {
                    CommandResult::err(format!("Boss bar '{id}' not found"))
                }
            }
            "set" => {
                if args.len() < 4 {
                    return CommandResult::err(
                        "Usage: /bossbar set <id> <name|color|max|value|visible> <value>",
                    );
                }
                let id = &args[1];
                let prop = args[2].as_str();
                let val = &args[3];
                if !self.boss_bars.contains_key(id) {
                    return CommandResult::err(format!("Boss bar '{id}' not found"));
                }
                let bar = self.boss_bars.get_mut(id).unwrap();
                match prop {
                    "name" => {
                        bar.title = args[3..].join(" ");
                        let pkt = BossEvent::update_title(bar.boss_id, &bar.title);
                        let players: Vec<SocketAddr> = bar.players.iter().copied().collect();
                        for player_addr in players {
                            self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                .await;
                        }
                    }
                    "color" => {
                        bar.color = val.parse().unwrap_or(0);
                    }
                    "max" => {
                        bar.max = val.parse().unwrap_or(100.0);
                        let health = if bar.max > 0.0 {
                            bar.value / bar.max
                        } else {
                            0.0
                        };
                        let pkt = BossEvent::update_health(bar.boss_id, health);
                        let players: Vec<SocketAddr> = bar.players.iter().copied().collect();
                        for player_addr in players {
                            self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                .await;
                        }
                    }
                    "value" => {
                        bar.value = val.parse().unwrap_or(0.0);
                        let health = if bar.max > 0.0 {
                            bar.value / bar.max
                        } else {
                            0.0
                        };
                        let pkt = BossEvent::update_health(bar.boss_id, health);
                        let players: Vec<SocketAddr> = bar.players.iter().copied().collect();
                        for player_addr in players {
                            self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                .await;
                        }
                    }
                    "visible" => {
                        let vis = val == "true" || val == "1";
                        bar.visible = vis;
                        if vis {
                            let health = if bar.max > 0.0 {
                                bar.value / bar.max
                            } else {
                                0.0
                            };
                            let pkt = BossEvent::show(bar.boss_id, &bar.title, health, bar.color);
                            let players: Vec<SocketAddr> = bar.players.iter().copied().collect();
                            for player_addr in players {
                                self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                    .await;
                            }
                        } else {
                            let pkt = BossEvent::hide(bar.boss_id);
                            let players: Vec<SocketAddr> = bar.players.iter().copied().collect();
                            for player_addr in players {
                                self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                    .await;
                            }
                        }
                    }
                    _ => {
                        return CommandResult::err(
                            "Property must be name, color, max, value, or visible",
                        );
                    }
                }
                CommandResult::ok(format!("Updated boss bar '{id}' {prop}"))
            }
            "players" => {
                if args.len() < 4 {
                    return CommandResult::err(
                        "Usage: /bossbar players <id> <add|remove> <target>",
                    );
                }
                let id = &args[1];
                let action = args[2].as_str();
                let target = &args[3];
                if !self.boss_bars.contains_key(id) {
                    return CommandResult::err(format!("Boss bar '{id}' not found"));
                }
                let targets = match self.resolve_target(target, addr) {
                    Ok(t) => t,
                    Err(e) => return CommandResult::err(e),
                };
                let bar = self.boss_bars.get(id).unwrap();
                let boss_id = bar.boss_id;
                let health = if bar.max > 0.0 {
                    bar.value / bar.max
                } else {
                    0.0
                };
                let title = bar.title.clone();
                let color = bar.color;

                match action {
                    "add" => {
                        let mut count = 0;
                        for name in &targets {
                            if let Some(player_addr) = self.find_player_addr(name) {
                                self.boss_bars
                                    .get_mut(id)
                                    .unwrap()
                                    .players
                                    .insert(player_addr);
                                let pkt = BossEvent::show(boss_id, &title, health, color);
                                self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                    .await;
                                count += 1;
                            }
                        }
                        CommandResult::ok(format!("Added {count} player(s) to boss bar '{id}'"))
                    }
                    "remove" => {
                        let mut count = 0;
                        for name in &targets {
                            if let Some(player_addr) = self.find_player_addr(name) {
                                self.boss_bars
                                    .get_mut(id)
                                    .unwrap()
                                    .players
                                    .remove(&player_addr);
                                let pkt = BossEvent::hide(boss_id);
                                self.send_packet(player_addr, packets::id::BOSS_EVENT, &pkt)
                                    .await;
                                count += 1;
                            }
                        }
                        CommandResult::ok(format!("Removed {count} player(s) from boss bar '{id}'"))
                    }
                    _ => CommandResult::err("Action must be add or remove"),
                }
            }
            "list" => {
                if self.boss_bars.is_empty() {
                    return CommandResult::ok("No boss bars active");
                }
                let list: Vec<String> = self
                    .boss_bars
                    .iter()
                    .map(|(id, bar)| {
                        format!(
                            "{id}: \"{}\" ({}/{}) [{} player(s)]",
                            bar.title,
                            bar.value,
                            bar.max,
                            bar.players.len()
                        )
                    })
                    .collect();
                CommandResult::ok(list.join("\n"))
            }
            _ => CommandResult::err("Subcommand must be add, remove, set, players, or list"),
        }
    }

    async fn cmd_execute(
        &mut self,
        addr: SocketAddr,
        sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /execute <as|at|positioned|if|unless|run> ...");
        }

        let pos = self
            .connections
            .get(&addr)
            .map(|c| (c.position.x, c.position.y, c.position.z))
            .unwrap_or((0.0, 0.0, 0.0));

        let mut results = Vec::new();
        self.execute_chain(args, sender_name.to_string(), addr, pos, &mut results)
            .await;

        if results.is_empty() {
            CommandResult::ok("Execute: no results")
        } else {
            CommandResult::ok(results.join("\n"))
        }
    }

    fn execute_chain<'a>(
        &'a mut self,
        args: &'a [String],
        executor_name: String,
        executor_addr: SocketAddr,
        position: (f32, f32, f32),
        results: &'a mut Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if args.is_empty() {
                return;
            }
            let sub = args[0].as_str();
            match sub {
                "as" => {
                    if args.len() < 2 {
                        results.push("Error: /execute as <target> ...".into());
                        return;
                    }
                    let targets = match self.resolve_target(&args[1], executor_addr) {
                        Ok(t) => t,
                        Err(e) => {
                            results.push(format!("Error: {e}"));
                            return;
                        }
                    };
                    let rest = &args[2..];
                    for target_name in targets {
                        let target_addr =
                            self.find_player_addr(&target_name).unwrap_or(executor_addr);
                        let target_pos = self
                            .connections
                            .get(&target_addr)
                            .map(|c| (c.position.x, c.position.y, c.position.z))
                            .unwrap_or(position);
                        self.execute_chain(rest, target_name, target_addr, target_pos, results)
                            .await;
                    }
                }
                "at" => {
                    if args.len() < 2 {
                        results.push("Error: /execute at <target> ...".into());
                        return;
                    }
                    let targets = match self.resolve_target(&args[1], executor_addr) {
                        Ok(t) => t,
                        Err(e) => {
                            results.push(format!("Error: {e}"));
                            return;
                        }
                    };
                    let rest = &args[2..];
                    for target_name in &targets {
                        if let Some(target_addr) = self.find_player_addr(target_name) {
                            let target_pos = self
                                .connections
                                .get(&target_addr)
                                .map(|c| (c.position.x, c.position.y, c.position.z))
                                .unwrap_or(position);
                            self.execute_chain(
                                rest,
                                executor_name.clone(),
                                executor_addr,
                                target_pos,
                                results,
                            )
                            .await;
                        }
                    }
                }
                "positioned" => {
                    if args.len() < 4 {
                        results.push("Error: /execute positioned <x> <y> <z> ...".into());
                        return;
                    }
                    let x = self.parse_relative_coord(&args[1], position.0);
                    let y = self.parse_relative_coord(&args[2], position.1);
                    let z = self.parse_relative_coord(&args[3], position.2);
                    let rest = &args[4..];
                    self.execute_chain(rest, executor_name, executor_addr, (x, y, z), results)
                        .await;
                }
                "if" | "unless" => {
                    // /execute if block <x> <y> <z> <block> ...
                    if args.len() < 6 || args[1] != "block" {
                        results
                            .push("Error: /execute if|unless block <x> <y> <z> <block> ...".into());
                        return;
                    }
                    let bx: i32 = match args[2].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            results.push("Error: invalid x coordinate".into());
                            return;
                        }
                    };
                    let by: i32 = match args[3].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            results.push("Error: invalid y coordinate".into());
                            return;
                        }
                    };
                    let bz: i32 = match args[4].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            results.push("Error: invalid z coordinate".into());
                            return;
                        }
                    };
                    let block_name = if args[5].contains(':') {
                        args[5].clone()
                    } else {
                        format!("minecraft:{}", args[5])
                    };
                    let expected_rid = hash_block_state(&block_name);
                    let actual_rid = self.get_block(bx, by, bz).unwrap_or(0);
                    let matches = actual_rid == expected_rid;
                    let should_continue = if sub == "if" { matches } else { !matches };
                    if should_continue {
                        let rest = &args[6..];
                        self.execute_chain(rest, executor_name, executor_addr, position, results)
                            .await;
                    }
                }
                "run" => {
                    if args.len() < 2 {
                        results.push("Error: /execute run <command> ...".into());
                        return;
                    }
                    // Build the command string and dispatch
                    let cmd_str = args[1..].join(" ");
                    let mut parts = cmd_str.split_whitespace();
                    let cmd_name = parts.next().unwrap_or("");
                    let cmd_args: Vec<String> = parts.map(String::from).collect();

                    let result = match cmd_name {
                        "setblock" => self.cmd_setblock(executor_addr, &cmd_args).await,
                        "fill" => self.cmd_fill(executor_addr, &cmd_args).await,
                        "clone" => self.cmd_clone(executor_addr, &cmd_args).await,
                        "title" => {
                            self.cmd_title(executor_addr, &executor_name, &cmd_args)
                                .await
                        }
                        "particle" => self.cmd_particle(executor_addr, &cmd_args).await,
                        "playsound" => {
                            self.cmd_playsound(executor_addr, &executor_name, &cmd_args)
                                .await
                        }
                        "say" => {
                            let msg = cmd_args.join(" ");
                            let text = Text::raw(format!("[{executor_name}] {msg}"));
                            self.broadcast_packet(packets::id::TEXT, &text).await;
                            CommandResult::ok(format!("Said: {msg}"))
                        }
                        "kill" => {
                            self.cmd_kill(executor_addr, &executor_name, &cmd_args)
                                .await
                        }
                        "give" => self.cmd_give(executor_addr, &cmd_args).await,
                        "tp" => self.cmd_tp(executor_addr, &executor_name, &cmd_args).await,
                        "effect" => {
                            self.cmd_effect(executor_addr, &executor_name, &cmd_args)
                                .await
                        }
                        _ => CommandResult::err(format!(
                            "Unknown command in execute run: {cmd_name}"
                        )),
                    };
                    for msg in &result.messages {
                        results.push(msg.clone());
                    }
                }
                _ => {
                    results.push(format!("Unknown execute subcommand: {sub}"));
                }
            }
        })
    }

    /// Parse a coordinate that may use ~ for relative positioning.
    fn parse_relative_coord(&self, s: &str, base: f32) -> f32 {
        if let Some(rest) = s.strip_prefix('~') {
            let offset: f32 = rest.parse().unwrap_or(0.0);
            base + offset
        } else {
            s.parse().unwrap_or(base)
        }
    }

    async fn cmd_transfer(
        &mut self,
        addr: SocketAddr,
        _sender_name: &str,
        args: &[String],
    ) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::err("Usage: /transfer <target> <host> [port]");
        }
        let target = &args[0];
        let host = &args[1];
        let port: u16 = if args.len() >= 3 {
            args[2].parse().unwrap_or(19132)
        } else {
            19132
        };

        let targets = match self.resolve_target(target, addr) {
            Ok(t) => t,
            Err(e) => return CommandResult::err(e),
        };

        let pkt = Transfer::new(host.as_str(), port);
        let mut count = 0;
        for name in &targets {
            if let Some(player_addr) = self.find_player_addr(name) {
                self.send_packet(player_addr, packets::id::TRANSFER, &pkt)
                    .await;
                count += 1;
            }
        }
        CommandResult::ok(format!("Transferred {count} player(s) to {host}:{port}"))
    }

    fn cmd_tickingarea(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /tickingarea <add|remove|list> [...]");
        }
        let sub = args[0].as_str();
        match sub {
            "add" => {
                // /tickingarea add <x1> <z1> <x2> <z2> <name>
                if args.len() < 6 {
                    return CommandResult::err(
                        "Usage: /tickingarea add <x1> <z1> <x2> <z2> <name>",
                    );
                }
                let x1: i32 = match args[1].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid x1"),
                };
                let z1: i32 = match args[2].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid z1"),
                };
                let x2: i32 = match args[3].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid x2"),
                };
                let z2: i32 = match args[4].parse() {
                    Ok(v) => v,
                    Err(_) => return CommandResult::err("Invalid z2"),
                };
                let name = args[5..].join(" ");

                // Convert to chunk coords
                let cx1 = x1 >> 4;
                let cz1 = z1 >> 4;
                let cx2 = x2 >> 4;
                let cz2 = z2 >> 4;
                let from = (cx1.min(cx2), cz1.min(cz2));
                let to = (cx1.max(cx2), cz1.max(cz2));

                if self.ticking_areas.iter().any(|a| a.name == name) {
                    return CommandResult::err(format!("Ticking area '{name}' already exists"));
                }

                self.ticking_areas.push(super::TickingArea {
                    name: name.clone(),
                    from,
                    to,
                });
                let chunk_count = (to.0 - from.0 + 1) as u64 * (to.1 - from.1 + 1) as u64;
                CommandResult::ok(format!(
                    "Added ticking area '{name}' ({chunk_count} chunks)"
                ))
            }
            "remove" => {
                if args.len() < 2 {
                    return CommandResult::err("Usage: /tickingarea remove <name>");
                }
                let name = args[1..].join(" ");
                let before = self.ticking_areas.len();
                self.ticking_areas.retain(|a| a.name != name);
                if self.ticking_areas.len() < before {
                    CommandResult::ok(format!("Removed ticking area '{name}'"))
                } else {
                    CommandResult::err(format!("Ticking area '{name}' not found"))
                }
            }
            "list" => {
                if self.ticking_areas.is_empty() {
                    return CommandResult::ok("No ticking areas defined");
                }
                let list: Vec<String> = self
                    .ticking_areas
                    .iter()
                    .map(|a| {
                        let chunks =
                            (a.to.0 - a.from.0 + 1) as u64 * (a.to.1 - a.from.1 + 1) as u64;
                        format!(
                            "{}: chunks ({},{}) to ({},{}) [{} chunks]",
                            a.name, a.from.0, a.from.1, a.to.0, a.to.1, chunks
                        )
                    })
                    .collect();
                CommandResult::ok(list.join("\n"))
            }
            _ => CommandResult::err("Subcommand must be add, remove, or list"),
        }
    }

    // ─── /import ─────────────────────────────────────────────────────────

    fn cmd_import(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /import <bds_world_path>");
        }

        let bds_path = std::path::Path::new(&args[0]).join("db");
        if !bds_path.exists() {
            return CommandResult::err(format!("BDS world not found at: {}", bds_path.display()));
        }

        match mc_rs_world::bds_compat::import_bds_world(
            &bds_path,
            &mut self.chunk_storage,
            self.dimension_id,
        ) {
            Ok(result) => CommandResult::ok(format!(
                "Imported {} chunks, {} block entities from BDS world",
                result.chunks, result.block_entities
            )),
            Err(e) => CommandResult::err(format!("Import failed: {e}")),
        }
    }

    // ─── /export ─────────────────────────────────────────────────────────

    fn cmd_export(&mut self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::err("Usage: /export <output_path>");
        }

        let export_path = std::path::Path::new(&args[0]).join("db");
        if let Err(e) = std::fs::create_dir_all(&export_path) {
            return CommandResult::err(format!("Cannot create export directory: {e}"));
        }

        let registry = mc_rs_world::block_state_registry::BlockStateRegistry::new();
        let empty = HashMap::new();
        let chunks = self.dim_chunks(self.dimension_id).unwrap_or(&empty);

        match mc_rs_world::bds_compat::export_bds_world(
            chunks,
            self.dimension_id,
            &export_path,
            &registry,
        ) {
            Ok(result) => CommandResult::ok(format!(
                "Exported {} chunks to BDS format at {}",
                result.chunks, args[0]
            )),
            Err(e) => CommandResult::err(format!("Export failed: {e}")),
        }
    }
}
