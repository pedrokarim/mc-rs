use super::*;

impl ConnectionHandler {
    // -----------------------------------------------------------------------
    // Phase 1.1: Movement
    // -----------------------------------------------------------------------

    /// Maximum horizontal distance (blocks) a player can move per tick.
    /// Sprint = ~0.28 b/t; 1.0 gives generous margin for latency.
    const MAX_MOVE_DISTANCE_PER_TICK: f32 = 1.0;

    /// Minimum allowed Y position (world bottom).
    const MIN_Y_POSITION: f32 = -64.0;

    pub(super) async fn handle_player_auth_input(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let input = match PlayerAuthInput::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad PlayerAuthInput from {addr}: {e}");
                return;
            }
        };

        let (prev_position, entity_runtime_id, gamemode) = match self.connections.get(&addr) {
            Some(c) => (c.position, c.entity_runtime_id, c.gamemode),
            None => return,
        };

        // --- Validation ---
        let mut needs_correction = false;

        // 1. Reject NaN/Infinity positions
        if !input.position.x.is_finite()
            || !input.position.y.is_finite()
            || !input.position.z.is_finite()
        {
            warn!("Invalid position (NaN/Inf) from {addr}");
            needs_correction = true;
        }

        // 2. Horizontal speed check
        if !needs_correction {
            let dx = input.position.x - prev_position.x;
            let dz = input.position.z - prev_position.z;
            let horizontal_distance = (dx * dx + dz * dz).sqrt();

            if horizontal_distance > Self::MAX_MOVE_DISTANCE_PER_TICK {
                debug!("Movement too fast from {addr}: {horizontal_distance:.2} blocks/tick");
                needs_correction = true;
            }
        }

        // 3. Y position check (void falling)
        if !needs_correction && input.position.y < Self::MIN_Y_POSITION {
            debug!(
                "Player {addr} below world: {:.2} < {}",
                input.position.y,
                Self::MIN_Y_POSITION
            );
            needs_correction = true;
        }

        // 4. Vertical speed check (terminal velocity)
        if !needs_correction {
            let dy = (input.position.y - prev_position.y).abs();
            if dy > MAX_FALL_PER_TICK {
                debug!("Vertical speed too fast from {addr}: {dy:.2} blocks/tick");
                needs_correction = true;
            }
        }

        // 5. No-clip detection (survival/adventure only)
        // Check that the player's AABB does not overlap any solid block.
        if !needs_correction && gamemode != 1 && gamemode != 3 {
            let aabb =
                PlayerAabb::from_eye_position(input.position.x, input.position.y, input.position.z);
            for (bx, by, bz) in aabb.intersecting_blocks() {
                if let Some(hash) = self.get_block(bx, by, bz) {
                    if self.block_registry.is_solid(hash) {
                        debug!("No-clip detected at ({bx},{by},{bz}) from {addr}");
                        needs_correction = true;
                        break;
                    }
                }
            }
        }

        if needs_correction {
            let conn = match self.connections.get(&addr) {
                Some(c) => c,
                None => return,
            };
            let correction = MovePlayer::reset(
                entity_runtime_id,
                conn.position,
                conn.pitch,
                conn.yaw,
                conn.head_yaw,
                conn.on_ground,
                input.tick,
            );
            self.send_packet(addr, packets::id::MOVE_PLAYER, &correction)
                .await;
            return;
        }

        // Plugin event: PlayerMove (cancellable)
        if let Some(conn) = self.connections.get(&addr) {
            let player = Self::make_plugin_player(conn);
            let from = (prev_position.x, prev_position.y, prev_position.z);
            let to = (input.position.x, input.position.y, input.position.z);
            let event = PluginEvent::PlayerMove { player, from, to };
            let snapshot = self.build_snapshot();
            let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
            if result == EventResult::Cancelled {
                let conn = match self.connections.get(&addr) {
                    Some(c) => c,
                    None => return,
                };
                let correction = MovePlayer::reset(
                    entity_runtime_id,
                    conn.position,
                    conn.pitch,
                    conn.yaw,
                    conn.head_yaw,
                    conn.on_ground,
                    input.tick,
                );
                self.send_packet(addr, packets::id::MOVE_PLAYER, &correction)
                    .await;
                return;
            }
        }

        // --- Accept the movement ---

        // On-ground detection: check block below player's feet.
        // In Bedrock, position.y is the eye position (1.62 above feet).
        const PLAYER_EYE_HEIGHT: f32 = 1.62;
        let feet_y = input.position.y - PLAYER_EYE_HEIGHT;
        let check_y = (feet_y - 0.01).floor() as i32;
        let check_x = input.position.x.floor() as i32;
        let check_z = input.position.z.floor() as i32;
        let on_ground = self
            .get_block(check_x, check_y, check_z)
            .map(|hash| self.block_registry.is_solid(hash))
            .unwrap_or(true); // Default true for unloaded chunks

        // Anti-fly: track consecutive airborne ticks (survival only)
        let anti_fly_correction = if gamemode == 0 && !on_ground {
            let ticks = self
                .connections
                .get(&addr)
                .map(|c| c.airborne_ticks)
                .unwrap_or(0);
            let dy = input.position.y - prev_position.y;
            // If airborne too long AND not falling → fly hack
            ticks + 1 > MAX_AIRBORNE_TICKS && dy >= 0.0
        } else {
            false
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.position = input.position;
            conn.pitch = input.pitch;
            conn.yaw = input.yaw;
            conn.head_yaw = input.head_yaw;
            conn.client_tick = input.tick;
            conn.on_ground = on_ground;
            conn.last_position_delta_y = input.position_delta.y;
            conn.is_sprinting =
                input.has_flag(mc_rs_proto::packets::player_auth_input::input_flags::SPRINTING);
            // Swimming tracking
            if input.has_flag(mc_rs_proto::packets::player_auth_input::input_flags::START_SWIMMING)
            {
                conn.is_swimming = true;
            }
            if input.has_flag(mc_rs_proto::packets::player_auth_input::input_flags::STOP_SWIMMING) {
                conn.is_swimming = false;
            }
            if on_ground {
                conn.airborne_ticks = 0;
            } else {
                conn.airborne_ticks = conn.airborne_ticks.saturating_add(1);
            }

            // Sync position to ECS mirror entity
            let uid = conn.entity_unique_id;
            self.game_world.update_player_position(
                uid,
                input.position.x,
                input.position.y,
                input.position.z,
            );
        }

        if anti_fly_correction {
            debug!("Anti-fly: {addr} airborne too long without falling (gamemode=survival)");
            let conn = match self.connections.get(&addr) {
                Some(c) => c,
                None => return,
            };
            let correction = MovePlayer::reset(
                entity_runtime_id,
                conn.position,
                conn.pitch,
                conn.yaw,
                conn.head_yaw,
                conn.on_ground,
                input.tick,
            );
            self.send_packet(addr, packets::id::MOVE_PLAYER, &correction)
                .await;
            return;
        }

        // --- Broadcast position to other players ---
        let move_pkt = MovePlayer::normal(
            entity_runtime_id,
            input.position,
            input.pitch,
            input.yaw,
            input.head_yaw,
            on_ground,
            input.tick,
        );
        self.broadcast_packet_except(addr, packets::id::MOVE_PLAYER, &move_pkt)
            .await;

        // --- Fall distance tracking + fall damage (survival only) ---
        if gamemode == 0 {
            if !on_ground && input.position_delta.y < 0.0 {
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.fall_distance += (-input.position_delta.y).abs();
                }
            }
            if on_ground {
                let fall_dist = self
                    .connections
                    .get(&addr)
                    .map(|c| c.fall_distance)
                    .unwrap_or(0.0);
                if fall_dist > 3.0 {
                    let mut damage = (fall_dist - 3.0).ceil();
                    // Feather Falling reduction from boots (armor slot 3)
                    let ff_reduction = self
                        .connections
                        .get(&addr)
                        .map(|c| {
                            game_combat::feather_falling_reduction(&c.inventory.armor[3].nbt_data)
                        })
                        .unwrap_or(0.0);
                    if ff_reduction > 0.0 {
                        damage *= 1.0 - ff_reduction;
                    }
                    let conn = match self.connections.get_mut(&addr) {
                        Some(c) => c,
                        None => return,
                    };
                    conn.health = (conn.health - damage).max(0.0);
                    conn.exhaustion += 0.0; // fall damage doesn't cause exhaustion
                    conn.fall_distance = 0.0;
                    let rid = conn.entity_runtime_id;
                    let hp = conn.health;
                    let tick = conn.client_tick;

                    self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                        .await;
                    self.send_packet(
                        addr,
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(rid, hp, tick),
                    )
                    .await;

                    if hp <= 0.0 {
                        let name = self
                            .connections
                            .get(&addr)
                            .and_then(|c| c.login_data.as_ref())
                            .map(|d| d.display_name.clone())
                            .unwrap_or_default();
                        self.handle_player_death_with_message(
                            addr,
                            &format!("{name} fell from a high place"),
                        )
                        .await;
                        return;
                    }
                } else if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.fall_distance = 0.0;
                }
            }

            // --- Exhaustion accumulation ---
            if let Some(conn) = self.connections.get_mut(&addr) {
                let hdist = (input.position_delta.x * input.position_delta.x
                    + input.position_delta.z * input.position_delta.z)
                    .sqrt();

                if conn.is_sprinting && hdist > 0.0 {
                    conn.exhaustion += hdist * 0.1;
                } else if conn.is_swimming && hdist > 0.0 {
                    conn.exhaustion += hdist * 0.01;
                }

                if input.has_flag(mc_rs_proto::packets::player_auth_input::input_flags::JUMPING) {
                    if conn.is_sprinting {
                        conn.exhaustion += 0.2;
                    } else {
                        conn.exhaustion += 0.05;
                    }
                }
            }
        }

        // --- Dynamic chunk loading ---
        let prev_chunk = (
            Self::chunk_coord(prev_position.x),
            Self::chunk_coord(prev_position.z),
        );
        let curr_chunk = (
            Self::chunk_coord(input.position.x),
            Self::chunk_coord(input.position.z),
        );

        if prev_chunk != curr_chunk {
            self.send_new_chunks(addr).await;
            self.cleanup_sent_chunks(addr);
        }
    }
}
