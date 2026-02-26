use super::*;

impl ConnectionHandler {
    /// Process random ticks and scheduled ticks for loaded chunks.
    /// Advance world time and weather each tick.
    pub(super) async fn tick_time_and_weather(&mut self) {
        let tick = self.game_world.current_tick();

        // --- Day/night cycle ---
        if self.do_daylight_cycle {
            self.world_time += 1;
            // Broadcast SetTime every 200 ticks (10 seconds)
            if tick.is_multiple_of(200) {
                let pkt = SetTime {
                    time: self.world_time as i32,
                };
                self.broadcast_packet(packets::id::SET_TIME, &pkt).await;
            }
        }

        // --- Weather cycle ---
        if self.do_weather_cycle {
            self.weather_duration -= 1;
            if self.weather_duration <= 0 {
                self.pick_next_weather().await;
            }

            // Smooth rain transition (±0.01 per tick)
            if (self.rain_level - self.rain_target).abs() > 0.001 {
                if self.rain_level < self.rain_target {
                    self.rain_level = (self.rain_level + 0.01).min(self.rain_target);
                } else {
                    self.rain_level = (self.rain_level - 0.01).max(self.rain_target);
                }
            }

            // Smooth lightning transition (±0.01 per tick)
            if (self.lightning_level - self.lightning_target).abs() > 0.001 {
                if self.lightning_level < self.lightning_target {
                    self.lightning_level = (self.lightning_level + 0.01).min(self.lightning_target);
                } else {
                    self.lightning_level = (self.lightning_level - 0.01).max(self.lightning_target);
                }
            }
        }

        // --- Lightning strikes (rare, during thunderstorms) ---
        if self.is_thundering && self.lightning_level > 0.5 && tick.is_multiple_of(100) {
            // Scope RNG before any .await
            let strike_pos = {
                let mut rng = rand::thread_rng();
                // ~1% chance per check (every 5 seconds) = roughly 1 strike per 500 seconds
                if rng.gen_range(0..100) < 1 {
                    let chunk_keys: Vec<(i32, i32)> = self.world_chunks.keys().copied().collect();
                    if !chunk_keys.is_empty() {
                        let &(cx, cz) = chunk_keys.choose(&mut rng).unwrap();
                        let lx = rng.gen_range(0..16);
                        let lz = rng.gen_range(0..16);
                        let world_x = cx * 16 + lx;
                        let world_z = cz * 16 + lz;
                        let world_y = if let Some(col) = self.world_chunks.get(&(cx, cz)) {
                            let mut y = 319;
                            while y > -64 {
                                if col
                                    .get_block_world(lx as usize, y, lz as usize)
                                    .unwrap_or(0)
                                    != 0
                                {
                                    break;
                                }
                                y -= 1;
                            }
                            y + 1
                        } else {
                            64
                        };
                        Some((world_x, world_y, world_z))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some((wx, wy, wz)) = strike_pos {
                let pkt = LevelEvent {
                    event_id: mc_rs_proto::packets::level_event::START_THUNDER,
                    position: Vec3::new(wx as f32 + 0.5, wy as f32, wz as f32 + 0.5),
                    data: 0,
                };
                self.broadcast_packet(packets::id::LEVEL_EVENT, &pkt).await;
            }
        }
    }

    /// Pick the next weather state randomly.
    async fn pick_next_weather(&mut self) {
        // Scope RNG before any .await
        let (new_raining, new_thundering, duration) = {
            let mut rng = rand::thread_rng();
            let roll: u32 = rng.gen_range(0..100);

            let (nr, nt) = if self.is_raining {
                if roll < 70 {
                    (false, false)
                } else {
                    (!self.is_thundering, !self.is_thundering)
                }
            } else if roll < 80 {
                (false, false)
            } else if roll < 95 {
                (true, false)
            } else {
                (true, true)
            };

            let dur = rng.gen_range(12000..24000);
            (nr, nt, dur)
        };

        // Plugin event: WeatherChange (cancellable)
        {
            let event = PluginEvent::WeatherChange {
                raining: new_raining,
                thundering: new_thundering,
            };
            let snapshot = self.build_snapshot();
            let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
            if result == EventResult::Cancelled {
                return;
            }
        }

        self.weather_duration = duration;

        // Update targets
        if new_raining != self.is_raining {
            self.rain_target = if new_raining { 1.0 } else { 0.0 };
            let pkt = if new_raining {
                LevelEvent::start_rain()
            } else {
                LevelEvent::stop_rain()
            };
            self.broadcast_packet(packets::id::LEVEL_EVENT, &pkt).await;
        }
        if new_thundering != self.is_thundering {
            self.lightning_target = if new_thundering { 1.0 } else { 0.0 };
            let pkt = if new_thundering {
                LevelEvent::start_thunder()
            } else {
                LevelEvent::stop_thunder()
            };
            self.broadcast_packet(packets::id::LEVEL_EVENT, &pkt).await;
        }

        self.is_raining = new_raining;
        self.is_thundering = new_thundering;
    }

    pub(super) async fn tick_block_updates(&mut self) {
        let current_tick = self.game_world.current_tick();

        // 1. Random ticks: 1 random block per non-empty sub-chunk for chunks near players
        // Collect all changes first (RNG must be scoped before any .await)
        let all_changes = {
            let sim_chunks = self.get_simulation_chunks();
            let mut rng = thread_rng();
            let mut changes: Vec<(i32, i32, i32, u32)> = Vec::new();

            for (cx, cz) in &sim_chunks {
                if let Some(column) = self.world_chunks.get(&(*cx, *cz)) {
                    for sub_idx in 0..OVERWORLD_SUB_CHUNK_COUNT {
                        // Skip empty sub-chunks (palette = [air] only)
                        if column.sub_chunks[sub_idx].palette.len() <= 1 {
                            continue;
                        }

                        let bx = rng.gen_range(0..16usize);
                        let by = rng.gen_range(0..16usize);
                        let bz = rng.gen_range(0..16usize);
                        let rid = column.sub_chunks[sub_idx].get_block(bx, by, bz);

                        if rid == self.tick_blocks.air {
                            continue;
                        }

                        let wx = cx * 16 + bx as i32;
                        let wy = OVERWORLD_MIN_Y + sub_idx as i32 * 16 + by as i32;
                        let wz = cz * 16 + bz as i32;

                        let result = process_random_tick(
                            rid,
                            wx,
                            wy,
                            wz,
                            &self.tick_blocks,
                            |x, y, z| self.get_block(x, y, z),
                            |rid| self.block_registry.is_solid(rid),
                        );
                        changes.extend(result);
                    }
                }
            }
            changes
        }; // rng dropped here, before any .await

        // Apply all random tick changes
        for (x, y, z, new_rid) in all_changes {
            self.set_block_and_broadcast(x, y, z, new_rid).await;
        }

        // 2. Scheduled ticks (fluid flow, gravity, redstone)
        let ready = self.tick_scheduler.drain_ready(current_tick);
        let scheduled_results: Vec<_> = ready
            .iter()
            .map(|tick| {
                process_scheduled_tick(
                    tick.x,
                    tick.y,
                    tick.z,
                    &self.tick_blocks,
                    |x, y, z| self.get_block(x, y, z),
                    |rid| self.block_registry.is_solid(rid),
                )
            })
            .collect();
        for result in scheduled_results {
            for (x, y, z, rid) in result.changes {
                self.set_block_and_broadcast(x, y, z, rid).await;
            }
            for (x, y, z, delay, prio) in result.schedule {
                self.tick_scheduler
                    .schedule(x, y, z, delay, current_tick, prio);
            }
        }
    }

    /// Get the set of chunk coordinates within simulation distance (4 chunks) of any player.
    fn get_simulation_chunks(&self) -> HashSet<(i32, i32)> {
        let mut chunks = HashSet::new();
        let sim_radius = 4i32;

        for conn in self.connections.values() {
            if conn.state != LoginState::InGame {
                continue;
            }
            let pcx = (conn.position.x as i32) >> 4;
            let pcz = (conn.position.z as i32) >> 4;

            for dx in -sim_radius..=sim_radius {
                for dz in -sim_radius..=sim_radius {
                    let key = (pcx + dx, pcz + dz);
                    if self.world_chunks.contains_key(&key) {
                        chunks.insert(key);
                    }
                }
            }
        }
        chunks
    }

    /// Set a block and broadcast the change to all players.
    pub(super) async fn set_block_and_broadcast(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        runtime_id: u32,
    ) {
        if self.set_block(x, y, z, runtime_id) {
            let pos = BlockPos::new(x, y, z);
            self.broadcast_packet(
                packets::id::UPDATE_BLOCK,
                &UpdateBlock::new(pos, runtime_id),
            )
            .await;
        }
    }

    /// Schedule fluid and gravity ticks for neighbors of a changed position.
    pub(super) fn schedule_fluid_neighbors(&mut self, x: i32, y: i32, z: i32) {
        let current_tick = self.game_world.current_tick();
        let positions = [
            (x, y, z),
            (x - 1, y, z),
            (x + 1, y, z),
            (x, y - 1, z),
            (x, y + 1, z),
            (x, y, z - 1),
            (x, y, z + 1),
        ];
        for (nx, ny, nz) in positions {
            if let Some(rid) = self.get_block(nx, ny, nz) {
                if let Some(ft) = self.tick_blocks.fluid_type(rid) {
                    let delay = fluid::tick_delay(ft);
                    self.tick_scheduler
                        .schedule(nx, ny, nz, delay, current_tick, 0);
                } else if self.tick_blocks.is_gravity_block(rid) {
                    self.tick_scheduler.schedule(
                        nx,
                        ny,
                        nz,
                        gravity::GRAVITY_TICK_DELAY,
                        current_tick,
                        0,
                    );
                }
            }
        }
    }

    /// Recalculate redstone wire near a position and apply changes.
    ///
    /// Called after block break/place or lever toggle to propagate signal changes.
    pub(super) async fn update_redstone_from(&mut self, x: i32, y: i32, z: i32) {
        let current_tick = self.game_world.current_tick();
        let result = redstone::recalculate_wire_from(
            x,
            y,
            z,
            &self.tick_blocks,
            |bx, by, bz| self.get_block(bx, by, bz),
            |rid| self.block_registry.is_solid(rid),
        );
        for (cx, cy, cz, rid) in result.changes {
            self.set_block_and_broadcast(cx, cy, cz, rid).await;
        }
        for (sx, sy, sz, delay, prio) in result.schedule {
            self.tick_scheduler
                .schedule(sx, sy, sz, delay, current_tick, prio);
        }
    }

    /// Send XP attributes to a player.
    pub(super) async fn send_xp_attributes(&mut self, addr: SocketAddr) {
        let (rid, level, total, tick) = match self.connections.get(&addr) {
            Some(c) => (c.entity_runtime_id, c.xp_level, c.xp_total, c.client_tick),
            None => return,
        };
        let progress = xp::xp_progress(level, total);
        self.send_packet(
            addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::xp(rid, level, progress, tick),
        )
        .await;
    }

    /// Award XP to a player (survival mode only) and send attribute update.
    pub(super) async fn award_xp(&mut self, addr: SocketAddr, amount: i32) {
        if amount <= 0 {
            return;
        }
        let gamemode = match self.connections.get(&addr) {
            Some(c) if !c.is_dead => c.gamemode,
            _ => return,
        };
        if gamemode != 0 {
            return; // survival only
        }
        if let Some(conn) = self.connections.get_mut(&addr) {
            let (nl, nt) = xp::add_xp(conn.xp_level, conn.xp_total, amount);
            conn.xp_level = nl;
            conn.xp_total = nt;
        }
        self.send_xp_attributes(addr).await;
    }
}
