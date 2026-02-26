use super::*;

impl ConnectionHandler {
    /// Handle Animate packet (arm swing broadcast).
    pub(super) async fn handle_animate(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let pkt = match Animate::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad Animate from {addr}: {e}");
                return;
            }
        };

        if pkt.action_type != mc_rs_proto::packets::animate::ACTION_SWING_ARM {
            return;
        }

        let runtime_id = match self.connections.get(&addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        let broadcast = Animate {
            action_type: mc_rs_proto::packets::animate::ACTION_SWING_ARM,
            entity_runtime_id: runtime_id,
        };
        self.broadcast_packet_except(addr, packets::id::ANIMATE, &broadcast)
            .await;
    }

    /// Handle a player attacking another entity (PvP or PvE).
    pub(super) async fn handle_attack(
        &mut self,
        attacker_addr: SocketAddr,
        victim_runtime_id: u64,
    ) {
        let (
            attacker_gamemode,
            attacker_pos,
            held_item_rid,
            is_sprinting,
            on_ground,
            delta_y,
            weapon_nbt,
        ) = match self.connections.get(&attacker_addr) {
            Some(c) => {
                let item = c.inventory.held_item();
                (
                    c.gamemode,
                    c.position,
                    item.runtime_id,
                    c.is_sprinting,
                    c.on_ground,
                    c.last_position_delta_y,
                    item.nbt_data.clone(),
                )
            }
            None => return,
        };

        // Creative/Spectator cannot attack
        if attacker_gamemode == 1 || attacker_gamemode == 3 {
            return;
        }

        // Attack exhaustion
        if let Some(conn) = self.connections.get_mut(&attacker_addr) {
            conn.exhaustion += 0.1;
        }

        let base_damage = base_attack_damage(&self.item_registry, held_item_rid);
        let is_critical = game_combat::is_critical_hit(on_ground, delta_y);
        let (strength_bonus, weakness_penalty) = self.get_attacker_bonuses(attacker_addr);

        // Check if target is a mob (PvE)
        if self.game_world.is_mob(victim_runtime_id) {
            let mob_pos = match self.game_world.mob_position(victim_runtime_id) {
                Some(p) => Vec3::new(p.0, p.1, p.2),
                None => return,
            };

            if attacker_pos.distance(&mob_pos) > 6.0 {
                return;
            }

            // PvE: no armor/protection/resistance on mobs (simplified)
            let damage = game_combat::calculate_damage(&game_combat::DamageInput {
                base_damage,
                weapon_nbt: &weapon_nbt,
                armor_defense: 0.0,
                armor_nbt_slots: &[],
                is_critical,
                strength_bonus,
                weakness_penalty,
                resistance_factor: 0.0,
            });

            let attacker_tick = self
                .connections
                .get(&attacker_addr)
                .map(|c| c.client_tick)
                .unwrap_or(0);

            let attacker_rid = self
                .connections
                .get(&attacker_addr)
                .map(|c| c.entity_runtime_id)
                .unwrap_or(0);
            if self
                .game_world
                .damage_mob(victim_runtime_id, damage, attacker_tick, Some(attacker_rid))
                .is_none()
            {
                return; // invulnerable
            }

            // Broadcast critical hit animation
            if is_critical {
                let attacker_rid = self
                    .connections
                    .get(&attacker_addr)
                    .map(|c| c.entity_runtime_id)
                    .unwrap_or(0);
                self.broadcast_packet(
                    packets::id::ANIMATE,
                    &Animate {
                        action_type: 4, // critical hit particles
                        entity_runtime_id: attacker_rid,
                    },
                )
                .await;
            }

            // Knockback (with enchantment bonus)
            let kb_enchant = game_combat::knockback_bonus(&weapon_nbt);
            let dx = mob_pos.x - attacker_pos.x;
            let dz = mob_pos.z - attacker_pos.z;
            let horizontal_len = (dx * dx + dz * dz).sqrt();
            if horizontal_len > 0.001 {
                let norm_x = dx / horizontal_len;
                let norm_z = dz / horizontal_len;
                let kb = 0.4 + kb_enchant as f32 * 0.3;
                let sprint_mult = if is_sprinting { 1.5 } else { 1.0 };
                let vx = norm_x * kb * sprint_mult;
                let vy = 0.4;
                let vz = norm_z * kb * sprint_mult;

                self.game_world
                    .apply_knockback(victim_runtime_id, vx, vy, vz);

                self.broadcast_packet(
                    packets::id::SET_ENTITY_MOTION,
                    &SetEntityMotion {
                        entity_runtime_id: victim_runtime_id,
                        motion: Vec3::new(vx, vy, vz),
                    },
                )
                .await;
            }

            return;
        }

        // --- PvP: find victim player by runtime_id ---
        let victim_addr = match self.find_addr_by_runtime_id(victim_runtime_id) {
            Some(a) => a,
            None => return,
        };

        let (victim_gamemode, victim_pos) = match self.connections.get(&victim_addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => (c.gamemode, c.position),
            _ => return,
        };

        if victim_gamemode == 1 || victim_gamemode == 3 {
            return;
        }

        let distance = attacker_pos.distance(&victim_pos);
        if distance > 6.0 {
            debug!("Attack rejected: distance {distance:.2} > 6.0 from {attacker_addr}");
            return;
        }

        // Invulnerability check (10 ticks = 500ms)
        let current_tick = self
            .connections
            .get(&attacker_addr)
            .map(|c| c.client_tick)
            .unwrap_or(0);
        if let Some(last_tick) = self
            .connections
            .get(&victim_addr)
            .and_then(|c| c.last_damage_tick)
        {
            if current_tick.saturating_sub(last_tick) < 10 {
                return;
            }
        }

        // Gather victim armor data for damage calculation
        let (armor_defense, armor_nbt_slots) = {
            let victim_conn = match self.connections.get(&victim_addr) {
                Some(c) => c,
                None => return,
            };
            let defense =
                game_combat::total_armor_defense(&self.item_registry, &victim_conn.inventory.armor);
            let nbt_slots: Vec<Vec<u8>> = victim_conn
                .inventory
                .armor
                .iter()
                .map(|item| item.nbt_data.clone())
                .collect();
            (defense, nbt_slots)
        };
        let armor_nbt_refs: Vec<&[u8]> = armor_nbt_slots.iter().map(|v| v.as_slice()).collect();
        let resistance_factor = self.get_resistance_factor(victim_addr);

        // Full damage pipeline
        let damage = game_combat::calculate_damage(&game_combat::DamageInput {
            base_damage,
            weapon_nbt: &weapon_nbt,
            armor_defense,
            armor_nbt_slots: &armor_nbt_refs,
            is_critical,
            strength_bonus,
            weakness_penalty,
            resistance_factor,
        });

        // Plugin event: PlayerDamage (cancellable)
        if let Some(conn) = self.connections.get(&victim_addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerDamage {
                player,
                damage,
                cause: DamageCause::Attack,
            };
            let snapshot = self.build_snapshot();
            let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
            if result == EventResult::Cancelled {
                return;
            }
        }

        // Apply damage
        let (new_health, victim_rid, victim_name) = {
            let conn = match self.connections.get_mut(&victim_addr) {
                Some(c) => c,
                None => return,
            };
            conn.health = (conn.health - damage).max(0.0);
            conn.last_damage_tick = Some(current_tick);
            let name = conn
                .login_data
                .as_ref()
                .map(|d| d.display_name.clone())
                .unwrap_or_default();
            (conn.health, conn.entity_runtime_id, name)
        };

        // Send UpdateAttributes (health) to victim
        self.send_packet(
            victim_addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::health(victim_rid, new_health, current_tick),
        )
        .await;

        // Broadcast EntityEvent(hurt) to all
        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(victim_rid))
            .await;

        // Broadcast critical hit animation
        if is_critical {
            let attacker_rid = self
                .connections
                .get(&attacker_addr)
                .map(|c| c.entity_runtime_id)
                .unwrap_or(0);
            self.broadcast_packet(
                packets::id::ANIMATE,
                &Animate {
                    action_type: 4, // critical hit particles
                    entity_runtime_id: attacker_rid,
                },
            )
            .await;
        }

        // Knockback (with enchantment bonus)
        let kb_enchant = game_combat::knockback_bonus(&weapon_nbt);
        let dx = victim_pos.x - attacker_pos.x;
        let dz = victim_pos.z - attacker_pos.z;
        let horizontal_len = (dx * dx + dz * dz).sqrt();

        if horizontal_len > 0.001 {
            let norm_x = dx / horizontal_len;
            let norm_z = dz / horizontal_len;
            let kb_horizontal = 0.4 + kb_enchant as f32 * 0.3;
            let kb_vertical = 0.4;
            let sprint_mult = if is_sprinting { 1.5 } else { 1.0 };

            let motion = Vec3::new(
                norm_x * kb_horizontal * sprint_mult,
                kb_vertical,
                norm_z * kb_horizontal * sprint_mult,
            );

            self.send_packet(
                victim_addr,
                packets::id::SET_ENTITY_MOTION,
                &SetEntityMotion {
                    entity_runtime_id: victim_rid,
                    motion,
                },
            )
            .await;
        }

        // Fire Aspect: set victim on fire
        let fire_level = game_combat::fire_aspect_level(&weapon_nbt);
        if fire_level > 0 {
            if let Some(conn) = self.connections.get_mut(&victim_addr) {
                conn.fire_ticks = fire_level as i32 * 80; // 4 seconds per level
            }
        }

        // Check for death
        if new_health <= 0.0 {
            self.handle_player_death(victim_addr, &victim_name, attacker_addr)
                .await;
        }
    }

    /// Handle a player dying.
    pub(super) async fn handle_player_death(
        &mut self,
        victim_addr: SocketAddr,
        victim_name: &str,
        killer_addr: SocketAddr,
    ) {
        let victim_rid = match self.connections.get(&victim_addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        // Plugin event: PlayerDeath
        {
            let killer_name = self
                .connections
                .get(&killer_addr)
                .and_then(|c| c.login_data.as_ref())
                .map(|d| d.display_name.clone())
                .unwrap_or_else(|| "???".to_string());
            if let Some(conn) = self.connections.get(&victim_addr) {
                let player = Self::make_plugin_player(conn);
                let event = PluginEvent::PlayerDeath {
                    player,
                    message: format!("{victim_name} was slain by {killer_name}"),
                };
                let snapshot = self.build_snapshot();
                let (_, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                self.apply_plugin_actions(actions).await;
            }
        }

        // XP loss on death + mark as dead
        if let Some(conn) = self.connections.get_mut(&victim_addr) {
            let (nl, nt) = xp::after_death(conn.xp_level, conn.xp_total);
            conn.xp_level = nl;
            conn.xp_total = nt;
            conn.is_dead = true;
        }

        // Broadcast death event to all
        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(victim_rid))
            .await;

        // Send Respawn(searching) to dead player — triggers death screen
        let spawn_pos = self.spawn_position;
        self.send_packet(
            victim_addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 0, // searching
                runtime_entity_id: victim_rid,
            },
        )
        .await;

        // Broadcast death message
        let killer_name = self
            .connections
            .get(&killer_addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_else(|| "???".to_string());

        let death_msg = Text::system(format!("{victim_name} was slain by {killer_name}"));
        self.broadcast_packet(packets::id::TEXT, &death_msg).await;
    }

    /// Handle a player death with a custom death message.
    pub(super) async fn handle_player_death_with_message(
        &mut self,
        victim_addr: SocketAddr,
        message: &str,
    ) {
        let victim_rid = match self.connections.get(&victim_addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        // Plugin event: PlayerDeath
        if let Some(conn) = self.connections.get(&victim_addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerDeath {
                player,
                message: message.to_string(),
            };
            let snapshot = self.build_snapshot();
            let (_, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
        }

        if let Some(conn) = self.connections.get_mut(&victim_addr) {
            let (nl, nt) = xp::after_death(conn.xp_level, conn.xp_total);
            conn.xp_level = nl;
            conn.xp_total = nt;
            conn.is_dead = true;
            conn.fire_ticks = 0;
            conn.effects.clear();
        }

        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::death(victim_rid))
            .await;

        let spawn_pos = self.spawn_position;
        self.send_packet(
            victim_addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 0,
                runtime_entity_id: victim_rid,
            },
        )
        .await;

        let death_msg = Text::system(message.to_string());
        self.broadcast_packet(packets::id::TEXT, &death_msg).await;
    }

    /// Handle Respawn packet from client (state=2, client clicked "Respawn").
    pub(super) async fn handle_respawn(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && c.is_dead => {}
            _ => return,
        }

        let pkt = match Respawn::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad Respawn from {addr}: {e}");
                return;
            }
        };

        // Client sends state=2 (client_ready) when "Respawn" is clicked
        if pkt.state != 2 {
            return;
        }

        let spawn_pos = self.spawn_position;

        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                conn.health = 20.0;
                conn.is_dead = false;
                conn.last_damage_tick = None;
                conn.position = spawn_pos;
                conn.effects.clear();
                conn.fire_ticks = 0;
                conn.food = 20;
                conn.saturation = 5.0;
                conn.exhaustion = 0.0;
                conn.fall_distance = 0.0;
                conn.air_ticks = 300;
                conn.is_swimming = false;
                conn.entity_runtime_id
            }
            None => return,
        };

        // Plugin event: PlayerRespawn
        if let Some(conn) = self.connections.get(&addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerRespawn { player };
            let snapshot = self.build_snapshot();
            let (_, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
        }

        // Send Respawn(server_ready)
        self.send_packet(
            addr,
            packets::id::RESPAWN,
            &Respawn {
                position: spawn_pos,
                state: 1, // server_ready
                runtime_entity_id: runtime_id,
            },
        )
        .await;

        // Send full health + hunger + XP
        let (tick, xl, xt) = match self.connections.get(&addr) {
            Some(c) => (c.client_tick, c.xp_level, c.xp_total),
            None => return,
        };
        let xp_prog = xp::xp_progress(xl, xt);
        self.send_packet(
            addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::all(runtime_id, 20.0, 20.0, 5.0, 0.0, xl, xp_prog, tick),
        )
        .await;

        // Broadcast position reset
        let move_pkt = MovePlayer::reset(runtime_id, spawn_pos, 0.0, 0.0, 0.0, true, tick);
        self.broadcast_packet(packets::id::MOVE_PLAYER, &move_pkt)
            .await;
    }
}
