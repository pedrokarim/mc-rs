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

    /// Tick all active furnaces: burn fuel, cook items, swap lit/unlit blocks.
    pub(super) async fn tick_furnaces(&mut self) {
        // Collect furnace positions that need ticking:
        // - currently lit (burning fuel), OR
        // - has smeltable input + available fuel (could ignite)
        let positions: Vec<(i32, i32, i32)> = self
            .block_entities
            .iter()
            .filter_map(|(&pos, be)| match be {
                BlockEntityData::Furnace {
                    lit_time,
                    input,
                    fuel,
                    furnace_type,
                    ..
                } => {
                    if *lit_time > 0 {
                        return Some(pos);
                    }
                    // Check if we could start burning: non-empty input with a recipe + fuel
                    if !input.is_empty() && !fuel.is_empty() {
                        let input_name = self
                            .item_registry
                            .get_by_id(input.runtime_id as i16)
                            .map(|i| i.name.as_str())
                            .unwrap_or("");
                        let fuel_name = self
                            .item_registry
                            .get_by_id(fuel.runtime_id as i16)
                            .map(|i| i.name.as_str())
                            .unwrap_or("");
                        if self
                            .smelting_registry
                            .find_recipe(input_name, input.metadata as i16, *furnace_type)
                            .is_some()
                            && self.smelting_registry.fuel_burn_time(fuel_name).is_some()
                        {
                            return Some(pos);
                        }
                    }
                    None
                }
                _ => None,
            })
            .collect();

        for pos in positions {
            self.tick_single_furnace(pos).await;
        }
    }

    /// Tick a single furnace at the given position.
    async fn tick_single_furnace(&mut self, pos: (i32, i32, i32)) {
        // Extract current furnace state
        let (
            furnace_type,
            mut input,
            mut fuel,
            mut output,
            mut cook_time,
            cook_time_total,
            mut lit_time,
            mut lit_duration,
            mut stored_xp,
        ) = match self.block_entities.get(&pos) {
            Some(BlockEntityData::Furnace {
                furnace_type,
                input,
                fuel,
                output,
                cook_time,
                cook_time_total,
                lit_time,
                lit_duration,
                stored_xp,
            }) => (
                *furnace_type,
                input.clone(),
                fuel.clone(),
                output.clone(),
                *cook_time,
                *cook_time_total,
                *lit_time,
                *lit_duration,
                *stored_xp,
            ),
            _ => return,
        };

        let was_lit = lit_time > 0;
        let mut changed = false;

        // Look up item names for recipe/fuel matching
        let input_name = self
            .item_registry
            .get_by_id(input.runtime_id as i16)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        let fuel_name = self
            .item_registry
            .get_by_id(fuel.runtime_id as i16)
            .map(|i| i.name.clone())
            .unwrap_or_default();

        // Find matching recipe for current input
        let recipe = self
            .smelting_registry
            .find_recipe(&input_name, input.metadata as i16, furnace_type)
            .cloned();

        // --- Try to ignite if not burning and has recipe + fuel ---
        if lit_time <= 0 && recipe.is_some() && !fuel.is_empty() {
            if let Some(burn) = self.smelting_registry.fuel_burn_time(&fuel_name) {
                lit_time = burn as i16;
                lit_duration = burn as i16;
                // Consume one fuel item
                fuel.count -= 1;
                if fuel.count == 0 {
                    fuel = mc_rs_proto::item_stack::ItemStack::empty();
                }
                changed = true;
            }
        }

        // --- Burn fuel ---
        if lit_time > 0 {
            lit_time -= 1;
            changed = true;

            // --- Cook progress ---
            if let Some(ref rec) = recipe {
                cook_time += 1;
                if cook_time >= cook_time_total {
                    // Cooking complete — produce output
                    cook_time = 0;
                    stored_xp += rec.xp;

                    // Find output item info
                    let out_info = self.item_registry.get_by_name(&rec.output_name);
                    if let Some(info) = out_info {
                        if output.is_empty() {
                            output = mc_rs_proto::item_stack::ItemStack {
                                runtime_id: info.numeric_id as i32,
                                count: rec.output_count as u16,
                                metadata: rec.output_metadata,
                                block_runtime_id: 0,
                                nbt_data: Vec::new(),
                                can_place_on: Vec::new(),
                                can_destroy: Vec::new(),
                                stack_network_id: 0,
                            };
                        } else if output.runtime_id == info.numeric_id as i32
                            && output.metadata == rec.output_metadata
                        {
                            let max_stack = self.item_registry.max_stack_size(info.numeric_id);
                            if output.count + rec.output_count as u16 <= max_stack as u16 {
                                output.count += rec.output_count as u16;
                            }
                            // If stack full, just don't produce (input stays)
                        }
                    }

                    // Consume one input item
                    input.count -= 1;
                    if input.count == 0 {
                        input = mc_rs_proto::item_stack::ItemStack::empty();
                    }
                }
            } else {
                // No valid recipe — reset cook progress
                if cook_time > 0 {
                    cook_time = 0;
                }
            }
        } else {
            // Not burning — reset cook progress
            if cook_time > 0 {
                cook_time = 0;
                changed = true;
            }
        }

        // --- Swap lit/unlit block ---
        let is_now_lit = lit_time > 0;
        if was_lit != is_now_lit {
            let current_rid = self.get_block(pos.0, pos.1, pos.2);
            if let Some(current_rid) = current_rid {
                let new_rid = if is_now_lit {
                    self.block_entity_hashes.lit_hash_for(current_rid)
                } else {
                    self.block_entity_hashes.unlit_hash_for(current_rid)
                };
                if let Some(new_rid) = new_rid {
                    self.set_block_and_broadcast(pos.0, pos.1, pos.2, new_rid)
                        .await;
                }
            }
        }

        // --- Write back furnace state ---
        if changed {
            if let Some(BlockEntityData::Furnace {
                input: ref mut be_input,
                fuel: ref mut be_fuel,
                output: ref mut be_output,
                cook_time: ref mut be_cook,
                lit_time: ref mut be_lit,
                lit_duration: ref mut be_dur,
                stored_xp: ref mut be_xp,
                ..
            }) = self.block_entities.get_mut(&pos)
            {
                *be_input = input;
                *be_fuel = fuel;
                *be_output = output;
                *be_cook = cook_time;
                *be_lit = lit_time;
                *be_dur = lit_duration;
                *be_xp = stored_xp;
            }

            // --- Send ContainerSetData to players viewing this furnace ---
            let viewers: Vec<(SocketAddr, u8)> = self
                .connections
                .iter()
                .filter_map(|(&addr, conn)| {
                    if let Some(ref oc) = conn.open_container {
                        if oc.position.x == pos.0
                            && oc.position.y == pos.1
                            && oc.position.z == pos.2
                        {
                            return Some((addr, oc.window_id));
                        }
                    }
                    None
                })
                .collect();

            for (addr, window_id) in &viewers {
                // Cook progress
                self.send_packet(
                    *addr,
                    packets::id::CONTAINER_SET_DATA,
                    &ContainerSetData {
                        window_id: *window_id,
                        property: 0,
                        value: cook_time as i32,
                    },
                )
                .await;
                // Lit time remaining
                self.send_packet(
                    *addr,
                    packets::id::CONTAINER_SET_DATA,
                    &ContainerSetData {
                        window_id: *window_id,
                        property: 1,
                        value: lit_time as i32,
                    },
                )
                .await;
                // Lit duration (total burn time)
                self.send_packet(
                    *addr,
                    packets::id::CONTAINER_SET_DATA,
                    &ContainerSetData {
                        window_id: *window_id,
                        property: 2,
                        value: lit_duration as i32,
                    },
                )
                .await;
                // Cook time total
                self.send_packet(
                    *addr,
                    packets::id::CONTAINER_SET_DATA,
                    &ContainerSetData {
                        window_id: *window_id,
                        property: 3,
                        value: cook_time_total as i32,
                    },
                )
                .await;

                // Also update inventory content for viewers
                let items = match self.block_entities.get(&pos) {
                    Some(BlockEntityData::Furnace {
                        input,
                        fuel,
                        output,
                        ..
                    }) => vec![input.clone(), fuel.clone(), output.clone()],
                    _ => continue,
                };
                self.send_packet(
                    *addr,
                    packets::id::INVENTORY_CONTENT,
                    &InventoryContent {
                        window_id: *window_id as u32,
                        items,
                    },
                )
                .await;
            }
        }
    }
}
