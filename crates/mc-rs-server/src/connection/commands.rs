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
}
