use super::*;

impl ConnectionHandler {
    /// Send the full inventory contents to a player.
    pub(super) async fn send_inventory(&mut self, addr: SocketAddr) {
        let items = match self.connections.get(&addr) {
            Some(c) => c.inventory.main.clone(),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 0,
                items,
            },
        )
        .await;

        let armor = match self.connections.get(&addr) {
            Some(c) => c.inventory.armor.clone(),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 119,
                items: armor,
            },
        )
        .await;
    }

    pub(super) async fn handle_mob_equipment(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => c.state,
            _ => return,
        };
        let _ = state;

        let equipment = match MobEquipment::proto_decode(buf) {
            Ok(e) => e,
            Err(e) => {
                debug!("Bad MobEquipment from {addr}: {e}");
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.inventory.held_slot = equipment.hotbar_slot;
        }

        // Broadcast to other players
        let entity_runtime_id = match self.connections.get(&addr) {
            Some(c) => c.entity_runtime_id,
            None => return,
        };

        let broadcast_pkt = MobEquipment {
            entity_runtime_id,
            item: equipment.item,
            inventory_slot: equipment.inventory_slot,
            hotbar_slot: equipment.hotbar_slot,
            window_id: equipment.window_id,
        };
        self.broadcast_packet_except(addr, packets::id::MOB_EQUIPMENT, &broadcast_pkt)
            .await;
    }

    pub(super) async fn handle_item_stack_request(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => c.state,
            _ => return,
        };
        let _ = state;

        let request = match ItemStackRequest::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                debug!("Bad ItemStackRequest from {addr}: {e}");
                return;
            }
        };

        let mut responses = Vec::new();
        for req in &request.requests {
            // Check if player has an open container (chest)
            let container_info = self
                .connections
                .get(&addr)
                .and_then(|c| c.open_container.as_ref())
                .map(|oc| (oc.window_id, oc.position));

            let response = if let Some((window_id, pos)) = container_info {
                // Extract container items based on block entity type
                let mut container_items = match self.block_entities.get(&(pos.x, pos.y, pos.z)) {
                    Some(BlockEntityData::Chest { items }) => items.clone(),
                    Some(BlockEntityData::Furnace {
                        input,
                        fuel,
                        output,
                        ..
                    }) => vec![input.clone(), fuel.clone(), output.clone()],
                    _ => vec![mc_rs_proto::item_stack::ItemStack::empty(); 3],
                };

                let resp = match self.connections.get_mut(&addr) {
                    Some(conn) => conn.inventory.process_request_with_container(
                        req,
                        &self.item_registry,
                        window_id,
                        &mut container_items,
                    ),
                    None => return,
                };

                // Write back modified items to the block entity
                match self.block_entities.get_mut(&(pos.x, pos.y, pos.z)) {
                    Some(BlockEntityData::Chest { items }) => {
                        *items = container_items;
                    }
                    Some(BlockEntityData::Furnace {
                        input,
                        fuel,
                        output,
                        ..
                    }) => {
                        if container_items.len() >= 3 {
                            *input = container_items[0].clone();
                            *fuel = container_items[1].clone();
                            *output = container_items[2].clone();
                        }
                    }
                    _ => {}
                }

                resp
            } else {
                match self.connections.get_mut(&addr) {
                    Some(conn) => conn.inventory.process_request(
                        req,
                        &self.item_registry,
                        &self.recipe_registry,
                    ),
                    None => return,
                }
            };
            responses.push(response);
        }

        self.send_packet(
            addr,
            packets::id::ITEM_STACK_RESPONSE,
            &ItemStackResponse { responses },
        )
        .await;
    }

    /// Build a CraftingData packet from the recipe registry.
    pub(super) fn build_crafting_data(&self) -> mc_rs_proto::packets::crafting_data::CraftingData {
        use mc_rs_proto::packets::crafting_data::{
            CraftingOutputItem, RecipeIngredient, ShapedRecipeEntry, ShapelessRecipeEntry,
        };

        let mut shaped_entries = Vec::new();
        for recipe in self.recipe_registry.shaped_recipes() {
            let input: Vec<RecipeIngredient> = recipe
                .input
                .iter()
                .map(|i| {
                    if i.item_name.is_empty() {
                        RecipeIngredient {
                            network_id: 0,
                            metadata: 0,
                            count: 0,
                        }
                    } else {
                        let nid = self
                            .item_registry
                            .get_by_name(&i.item_name)
                            .map(|e| e.numeric_id)
                            .unwrap_or(0);
                        let meta = if i.metadata == -1 { 0x7FFF } else { i.metadata };
                        RecipeIngredient {
                            network_id: nid,
                            metadata: meta,
                            count: i.count as i32,
                        }
                    }
                })
                .collect();
            let output: Vec<CraftingOutputItem> = recipe
                .output
                .iter()
                .map(|o| {
                    let nid = self
                        .item_registry
                        .get_by_name(&o.item_name)
                        .map(|e| e.numeric_id as i32)
                        .unwrap_or(0);
                    CraftingOutputItem {
                        network_id: nid,
                        count: o.count as u16,
                        metadata: o.metadata,
                        block_runtime_id: 0,
                    }
                })
                .collect();
            let mut uuid = [0u8; 16];
            uuid[0..4].copy_from_slice(&recipe.network_id.to_le_bytes());
            uuid[4] = 0x01;
            shaped_entries.push(ShapedRecipeEntry {
                recipe_id: recipe.id.clone(),
                width: recipe.width as i32,
                height: recipe.height as i32,
                input,
                output,
                uuid,
                tag: recipe.tag.clone(),
                network_id: recipe.network_id,
            });
        }

        let mut shapeless_entries = Vec::new();
        for recipe in self.recipe_registry.shapeless_recipes() {
            let input: Vec<RecipeIngredient> = recipe
                .inputs
                .iter()
                .map(|i| {
                    let nid = self
                        .item_registry
                        .get_by_name(&i.item_name)
                        .map(|e| e.numeric_id)
                        .unwrap_or(0);
                    let meta = if i.metadata == -1 { 0x7FFF } else { i.metadata };
                    RecipeIngredient {
                        network_id: nid,
                        metadata: meta,
                        count: i.count as i32,
                    }
                })
                .collect();
            let output: Vec<CraftingOutputItem> = recipe
                .output
                .iter()
                .map(|o| {
                    let nid = self
                        .item_registry
                        .get_by_name(&o.item_name)
                        .map(|e| e.numeric_id as i32)
                        .unwrap_or(0);
                    CraftingOutputItem {
                        network_id: nid,
                        count: o.count as u16,
                        metadata: o.metadata,
                        block_runtime_id: 0,
                    }
                })
                .collect();
            let mut uuid = [0u8; 16];
            uuid[0..4].copy_from_slice(&recipe.network_id.to_le_bytes());
            uuid[4] = 0x00;
            shapeless_entries.push(ShapelessRecipeEntry {
                recipe_id: recipe.id.clone(),
                input,
                output,
                uuid,
                tag: recipe.tag.clone(),
                network_id: recipe.network_id,
            });
        }

        // Furnace recipes (type 3 = FurnaceDataRecipe)
        use mc_rs_proto::packets::crafting_data::FurnaceRecipeEntry;
        let mut furnace_entries = Vec::new();
        for recipe in self.smelting_registry.recipes() {
            let input_nid = self
                .item_registry
                .get_by_name(&recipe.input_name)
                .map(|e| e.numeric_id as i32)
                .unwrap_or(0);
            if input_nid == 0 {
                continue;
            }
            let output_nid = self
                .item_registry
                .get_by_name(&recipe.output_name)
                .map(|e| e.numeric_id as i32)
                .unwrap_or(0);
            if output_nid == 0 {
                continue;
            }
            let input_meta = if recipe.input_metadata == -1 {
                0x7FFF
            } else {
                recipe.input_metadata as i32
            };
            // Emit one entry per tag
            for tag in &recipe.tags {
                furnace_entries.push(FurnaceRecipeEntry {
                    input_id: input_nid,
                    input_metadata: input_meta,
                    output: CraftingOutputItem {
                        network_id: output_nid,
                        count: recipe.output_count as u16,
                        metadata: recipe.output_metadata,
                        block_runtime_id: 0,
                    },
                    tag: tag.clone(),
                });
            }
        }

        mc_rs_proto::packets::crafting_data::CraftingData {
            shaped: shaped_entries,
            shapeless: shapeless_entries,
            furnace: furnace_entries,
            clear_recipes: true,
        }
    }

    pub(super) async fn handle_player_action(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame => {}
            _ => return,
        }

        let action = match PlayerAction::proto_decode(buf) {
            Ok(a) => a,
            Err(e) => {
                warn!("Bad PlayerAction from {addr}: {e}");
                return;
            }
        };

        match action.action {
            PlayerActionType::StartBreak => {
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.breaking_block = Some((action.block_position, Instant::now()));
                }
                debug!("StartBreak at {} by {addr}", action.block_position);
            }
            PlayerActionType::AbortBreak => {
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.breaking_block = None;
                }
                debug!("AbortBreak by {addr}");
            }
            other => {
                debug!("PlayerAction {:?} from {addr}", other);
            }
        }
    }

    pub(super) async fn handle_inventory_transaction(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::InGame && !c.is_dead => {}
            _ => return,
        }

        let transaction = match InventoryTransaction::proto_decode(buf) {
            Ok(t) => t,
            Err(e) => {
                warn!("Bad InventoryTransaction from {addr}: {e}");
                return;
            }
        };

        // Handle UseItemOnEntity (attack/interact) first
        if let Some(entity_data) = transaction.use_item_on_entity {
            if entity_data.action == UseItemOnEntityAction::Attack {
                self.handle_attack(addr, entity_data.entity_runtime_id)
                    .await;
            }
            return;
        }

        let use_item = match transaction.use_item {
            Some(data) => data,
            None => return,
        };

        match use_item.action {
            UseItemAction::BreakBlock => {
                let pos = use_item.block_position;

                // Check bounds
                if pos.y < OVERWORLD_MIN_Y || pos.y > Self::MAX_Y {
                    return;
                }

                // Get old block before replacing
                let old_runtime_id = match self.get_block(pos.x, pos.y, pos.z) {
                    Some(id) => id,
                    None => return,
                };

                let air_hash = self.flat_world_blocks.air;

                // Skip if already air
                if old_runtime_id == air_hash {
                    return;
                }

                // Don't allow breaking unbreakable blocks (bedrock, barriers, etc.)
                if let Some(hardness) = self.block_registry.hardness(old_runtime_id) {
                    if hardness < 0.0 {
                        debug!("Rejected break of unbreakable block at {pos} by {addr}");
                        return;
                    }
                }

                // Mining time validation for survival mode
                let gamemode = self.connections.get(&addr).map(|c| c.gamemode).unwrap_or(0);

                if gamemode == 0 {
                    // Survival mode: validate mining time
                    let breaking_info = self.connections.get(&addr).and_then(|c| c.breaking_block);

                    if let Some(expected_secs) =
                        self.block_registry.expected_mining_secs(old_runtime_id)
                    {
                        // Apply Efficiency enchantment to expected mining time
                        let eff_level = self
                            .connections
                            .get(&addr)
                            .map(|c| {
                                game_combat::efficiency_level(&c.inventory.held_item().nbt_data)
                            })
                            .unwrap_or(0);
                        let adjusted_secs = if eff_level > 0 {
                            expected_secs / (1.0 + (eff_level * eff_level) as f32)
                        } else {
                            expected_secs
                        };
                        if adjusted_secs > 0.0 {
                            match breaking_info {
                                Some((break_pos, start_time)) if break_pos == pos => {
                                    let elapsed = start_time.elapsed().as_secs_f32();
                                    let min_allowed = adjusted_secs * 0.8;
                                    if elapsed < min_allowed {
                                        debug!(
                                            "Mining too fast at {pos} by {addr}: {elapsed:.2}s < {min_allowed:.2}s (expected {adjusted_secs:.2}s)"
                                        );
                                        return;
                                    }
                                }
                                _ => {
                                    // No StartBreak recorded for this position
                                    debug!("No StartBreak for {pos} from {addr}, rejecting break");
                                    return;
                                }
                            }
                        }
                    }

                    // Clear breaking state
                    if let Some(conn) = self.connections.get_mut(&addr) {
                        conn.breaking_block = None;
                    }
                }

                // Plugin event: BlockBreak (cancellable)
                if let Some(conn) = self.connections.get(&addr) {
                    let player = Self::make_plugin_player(conn);
                    let event = PluginEvent::BlockBreak {
                        player,
                        position: PluginBlockPos {
                            x: pos.x,
                            y: pos.y,
                            z: pos.z,
                        },
                        block_id: old_runtime_id,
                    };
                    let snapshot = self.build_snapshot();
                    let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                    self.apply_plugin_actions(actions).await;
                    if result == EventResult::Cancelled {
                        return;
                    }
                }

                // Set to air
                if !self.set_block(pos.x, pos.y, pos.z, air_hash) {
                    return;
                }

                // Send UpdateBlock to all players
                let update = UpdateBlock::new(pos, air_hash);
                self.broadcast_packet(packets::id::UPDATE_BLOCK, &update)
                    .await;

                // Send LevelEvent (destroy particles) to all players
                let event = LevelEvent::destroy_block(pos.x, pos.y, pos.z, old_runtime_id);
                self.broadcast_packet(packets::id::LEVEL_EVENT, &event)
                    .await;

                // Trigger fluid updates for neighbors (water/lava may flow into the gap)
                self.schedule_fluid_neighbors(pos.x, pos.y, pos.z);

                // Trigger redstone updates if broken near wire
                self.update_redstone_from(pos.x, pos.y, pos.z).await;

                // Remove block entity if any
                self.block_entities.remove(&(pos.x, pos.y, pos.z));
                // Close any open containers at this position
                self.close_container_at(pos).await;

                // Award XP for ore mining (survival only)
                if gamemode == 0 {
                    if let Some(info) = self.block_registry.get(old_runtime_id) {
                        let ore_xp = xp::ore_xp_random(info.name);
                        if ore_xp > 0 {
                            self.award_xp(addr, ore_xp).await;
                        }
                    }
                }

                debug!("Block broken at {pos} by {addr}");
            }
            UseItemAction::ClickBlock => {
                // Check if the clicked block is interactive (lever, repeater)
                let click_pos = use_item.block_position;
                if let Some(rid) = self.get_block(click_pos.x, click_pos.y, click_pos.z) {
                    if let Some(toggled) = self.tick_blocks.toggle_lever(rid) {
                        self.set_block_and_broadcast(
                            click_pos.x,
                            click_pos.y,
                            click_pos.z,
                            toggled,
                        )
                        .await;
                        self.update_redstone_from(click_pos.x, click_pos.y, click_pos.z)
                            .await;
                        return;
                    }
                    if let Some(cycled) = self.tick_blocks.cycle_repeater_delay(rid) {
                        self.set_block_and_broadcast(click_pos.x, click_pos.y, click_pos.z, cycled)
                            .await;
                        return;
                    }
                    // Check if clicking on a chest → open it
                    if self.block_entity_hashes.is_chest(rid) {
                        self.open_chest(addr, click_pos).await;
                        return;
                    }
                    if self.block_entity_hashes.is_furnace(rid) {
                        self.open_furnace(addr, click_pos).await;
                        return;
                    }
                }

                let target = Self::face_offset(use_item.block_position, use_item.face);

                // Check bounds
                if target.y < OVERWORLD_MIN_Y || target.y > Self::MAX_Y {
                    return;
                }

                // Get the block runtime ID from the held item
                let block_runtime_id = use_item.held_item_block_runtime_id;
                if block_runtime_id <= 0 {
                    return;
                }
                let block_runtime_id = block_runtime_id as u32;

                // Don't place air
                let air_hash = self.flat_world_blocks.air;
                if block_runtime_id == air_hash {
                    return;
                }

                // Check that target is in a loaded chunk
                let cx = target.x >> 4;
                let cz = target.z >> 4;
                if !self.world_chunks.contains_key(&(cx, cz)) {
                    return;
                }

                // Plugin event: BlockPlace (cancellable)
                if let Some(conn) = self.connections.get(&addr) {
                    let player = Self::make_plugin_player(conn);
                    let event = PluginEvent::BlockPlace {
                        player,
                        position: PluginBlockPos {
                            x: target.x,
                            y: target.y,
                            z: target.z,
                        },
                        block_id: block_runtime_id,
                    };
                    let snapshot = self.build_snapshot();
                    let (result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                    self.apply_plugin_actions(actions).await;
                    if result == EventResult::Cancelled {
                        return;
                    }
                }

                // Remap sign/chest block hashes to include correct state (direction)
                let yaw = self.connections.get(&addr).map(|c| c.yaw).unwrap_or(0.0);
                let final_rid = if self.block_entity_hashes.is_sign(block_runtime_id) {
                    if use_item.face == 1 {
                        // Standing sign: direction from player yaw
                        let (hash, _) = self.block_entity_hashes.standing_sign_direction(yaw);
                        hash
                    } else if (2..=5).contains(&use_item.face) {
                        // Wall sign: facing from clicked face
                        self.block_entity_hashes
                            .wall_sign_face(use_item.face)
                            .unwrap_or(block_runtime_id)
                    } else {
                        block_runtime_id
                    }
                } else if self.block_entity_hashes.is_chest(block_runtime_id) {
                    // Chest faces the player
                    self.block_entity_hashes.chest_from_yaw(yaw)
                } else if let Some(variant) =
                    self.block_entity_hashes.furnace_variant(block_runtime_id)
                {
                    // Furnace faces the player
                    self.block_entity_hashes
                        .furnace_from_yaw(variant, yaw, false)
                } else {
                    block_runtime_id
                };

                // Set the block
                if !self.set_block(target.x, target.y, target.z, final_rid) {
                    return;
                }

                // Send UpdateBlock to all players
                let update = UpdateBlock::new(target, final_rid);
                self.broadcast_packet(packets::id::UPDATE_BLOCK, &update)
                    .await;

                // Create block entity if sign or chest
                if self.block_entity_hashes.is_sign(final_rid) {
                    let be = BlockEntityData::new_sign();
                    let nbt = be.to_network_nbt(target.x, target.y, target.z);
                    self.block_entities
                        .insert((target.x, target.y, target.z), be);
                    // Send BlockActorData to open the sign editor
                    self.send_packet(
                        addr,
                        packets::id::BLOCK_ACTOR_DATA,
                        &BlockActorData {
                            position: target,
                            nbt_data: nbt,
                        },
                    )
                    .await;
                } else if self.block_entity_hashes.is_chest(final_rid) {
                    self.block_entities
                        .insert((target.x, target.y, target.z), BlockEntityData::new_chest());
                } else if let Some(variant) = self.block_entity_hashes.furnace_variant(final_rid) {
                    use mc_rs_game::smelting::FurnaceType;
                    let ft = match variant {
                        mc_rs_world::block_hash::FurnaceVariant::Furnace => FurnaceType::Furnace,
                        mc_rs_world::block_hash::FurnaceVariant::BlastFurnace => {
                            FurnaceType::BlastFurnace
                        }
                        mc_rs_world::block_hash::FurnaceVariant::Smoker => FurnaceType::Smoker,
                    };
                    self.block_entities.insert(
                        (target.x, target.y, target.z),
                        BlockEntityData::new_furnace(ft),
                    );
                }

                // Trigger fluid updates: if placed block is fluid, schedule self;
                // also schedule neighboring fluids that may be affected
                self.schedule_fluid_neighbors(target.x, target.y, target.z);

                // Trigger redstone updates if placed near wire
                self.update_redstone_from(target.x, target.y, target.z)
                    .await;

                debug!("Block placed at {target} by {addr}");
            }
            UseItemAction::ClickAir => {
                // Food consumption
                let (item_rid, food_level) = match self.connections.get(&addr) {
                    Some(c) => (c.inventory.held_item().runtime_id, c.food),
                    None => return,
                };
                if item_rid == 0 {
                    return;
                }
                let item_name = self
                    .item_registry
                    .get_by_id(item_rid as i16)
                    .map(|info| info.name.clone());
                if let Some(name) = item_name {
                    if let Some(fd) = mc_rs_game::food::food_data(&name) {
                        if food_level < 20 {
                            let conn = match self.connections.get_mut(&addr) {
                                Some(c) => c,
                                None => return,
                            };
                            conn.food = (conn.food + fd.hunger).min(20);
                            conn.saturation =
                                (conn.saturation + fd.saturation).min(conn.food as f32);

                            // Decrement item count
                            let slot = conn.inventory.held_slot as usize;
                            let stack = &mut conn.inventory.main[slot];
                            if stack.count > 1 {
                                stack.count -= 1;
                            } else {
                                conn.inventory.main[slot] =
                                    mc_rs_proto::item_stack::ItemStack::empty();
                            }

                            let rid = conn.entity_runtime_id;
                            let food = conn.food;
                            let sat = conn.saturation;
                            let exh = conn.exhaustion;
                            let tick = conn.client_tick;
                            let updated_item = conn.inventory.main[slot].clone();

                            // Send updated inventory slot
                            self.send_packet(
                                addr,
                                packets::id::INVENTORY_SLOT,
                                &InventorySlot {
                                    window_id: 0,
                                    slot: slot as u32,
                                    item: updated_item,
                                },
                            )
                            .await;

                            // Send updated hunger attributes
                            self.send_packet(
                                addr,
                                packets::id::UPDATE_ATTRIBUTES,
                                &UpdateAttributes::hunger(rid, food as f32, sat, exh, tick),
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    /// Open a chest container for a player.
    async fn open_chest(&mut self, addr: SocketAddr, pos: BlockPos) {
        let window_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                let wid = conn.next_window_id;
                conn.next_window_id = conn.next_window_id.wrapping_add(1);
                if conn.next_window_id == 0 {
                    conn.next_window_id = 1;
                }
                conn.open_container = Some(OpenContainer {
                    window_id: wid,
                    container_type: 0,
                    position: pos,
                });
                wid
            }
            None => return,
        };

        // Send ContainerOpen
        self.send_packet(
            addr,
            packets::id::CONTAINER_OPEN,
            &ContainerOpen {
                window_id,
                container_type: 0, // Container
                position: pos,
                entity_unique_id: -1,
            },
        )
        .await;

        // Get chest items
        let items = self
            .block_entities
            .get(&(pos.x, pos.y, pos.z))
            .and_then(|be| match be {
                BlockEntityData::Chest { items } => Some(items.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                (0..27)
                    .map(|_| mc_rs_proto::item_stack::ItemStack::empty())
                    .collect()
            });

        // Send InventoryContent with the chest's items
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: window_id as u32,
                items,
            },
        )
        .await;

        debug!("Opened chest at {pos} for {addr} (window_id={window_id})");
    }

    /// Open a furnace container UI for a player.
    async fn open_furnace(&mut self, addr: SocketAddr, pos: BlockPos) {
        // Determine furnace type from block entity
        let furnace_type = match self.block_entities.get(&(pos.x, pos.y, pos.z)) {
            Some(BlockEntityData::Furnace { furnace_type, .. }) => *furnace_type,
            _ => return,
        };

        let container_type = furnace_type.container_type();

        let window_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                let wid = conn.next_window_id;
                conn.next_window_id = conn.next_window_id.wrapping_add(1);
                if conn.next_window_id == 0 {
                    conn.next_window_id = 1;
                }
                conn.open_container = Some(OpenContainer {
                    window_id: wid,
                    container_type,
                    position: pos,
                });
                wid
            }
            None => return,
        };

        // Send ContainerOpen
        self.send_packet(
            addr,
            packets::id::CONTAINER_OPEN,
            &ContainerOpen {
                window_id,
                container_type,
                position: pos,
                entity_unique_id: -1,
            },
        )
        .await;

        // Get furnace items
        let items = match self.block_entities.get(&(pos.x, pos.y, pos.z)) {
            Some(BlockEntityData::Furnace {
                input,
                fuel,
                output,
                ..
            }) => vec![input.clone(), fuel.clone(), output.clone()],
            _ => vec![mc_rs_proto::item_stack::ItemStack::empty(); 3],
        };

        // Send InventoryContent
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: window_id as u32,
                items,
            },
        )
        .await;

        // Send ContainerSetData for furnace progress bars
        let furnace_data = match self.block_entities.get(&(pos.x, pos.y, pos.z)) {
            Some(BlockEntityData::Furnace {
                cook_time,
                cook_time_total,
                lit_time,
                lit_duration,
                ..
            }) => Some((*cook_time, *cook_time_total, *lit_time, *lit_duration)),
            _ => None,
        };
        if let Some((ct, ctt, lt, ld)) = furnace_data {
            self.send_packet(
                addr,
                packets::id::CONTAINER_SET_DATA,
                &ContainerSetData {
                    window_id,
                    property: 0,
                    value: ct as i32,
                },
            )
            .await;
            self.send_packet(
                addr,
                packets::id::CONTAINER_SET_DATA,
                &ContainerSetData {
                    window_id,
                    property: 1,
                    value: lt as i32,
                },
            )
            .await;
            self.send_packet(
                addr,
                packets::id::CONTAINER_SET_DATA,
                &ContainerSetData {
                    window_id,
                    property: 2,
                    value: ld as i32,
                },
            )
            .await;
            self.send_packet(
                addr,
                packets::id::CONTAINER_SET_DATA,
                &ContainerSetData {
                    window_id,
                    property: 3,
                    value: ctt as i32,
                },
            )
            .await;
        }

        debug!(
            "Opened {:?} at {pos} for {addr} (window_id={window_id})",
            furnace_type
        );
    }

    /// Close any open containers at the given position for all players.
    async fn close_container_at(&mut self, pos: BlockPos) {
        let to_close: Vec<(SocketAddr, u8)> = self
            .connections
            .iter()
            .filter_map(|(&a, c)| {
                c.open_container
                    .as_ref()
                    .filter(|oc| oc.position == pos)
                    .map(|oc| (a, oc.window_id))
            })
            .collect();

        for (a, wid) in to_close {
            if let Some(conn) = self.connections.get_mut(&a) {
                conn.open_container = None;
            }
            self.send_packet(
                a,
                packets::id::CONTAINER_CLOSE,
                &ContainerClose {
                    window_id: wid,
                    server_initiated: true,
                },
            )
            .await;
        }
    }

    /// Handle a client-initiated container close.
    pub(super) async fn handle_container_close(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let pkt = match ContainerClose::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                debug!("Bad ContainerClose from {addr}: {e}");
                return;
            }
        };

        let matches = self
            .connections
            .get(&addr)
            .and_then(|c| c.open_container.as_ref())
            .map(|oc| oc.window_id == pkt.window_id)
            .unwrap_or(false);

        if matches {
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.open_container = None;
            }
        }

        // Echo back ContainerClose
        self.send_packet(
            addr,
            packets::id::CONTAINER_CLOSE,
            &ContainerClose {
                window_id: pkt.window_id,
                server_initiated: false,
            },
        )
        .await;
    }

    /// Handle a block actor data update from the client (sign text editing).
    pub(super) async fn handle_block_actor_data(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let pkt = match BlockActorData::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                debug!("Bad BlockActorData from {addr}: {e}");
                return;
            }
        };

        let pos = pkt.position;

        // Only handle sign edits for editable signs
        let is_editable_sign = self
            .block_entities
            .get(&(pos.x, pos.y, pos.z))
            .map(|be| {
                matches!(
                    be,
                    BlockEntityData::Sign {
                        is_editable: true,
                        ..
                    }
                )
            })
            .unwrap_or(false);

        if !is_editable_sign {
            return;
        }

        // Parse the sign text from the client's NBT
        let (front, back) = match BlockEntityData::sign_from_network_nbt(&pkt.nbt_data) {
            Some(t) => t,
            None => return,
        };

        // Update the sign and mark as no longer editable
        if let Some(BlockEntityData::Sign {
            front_text,
            back_text,
            is_editable,
        }) = self.block_entities.get_mut(&(pos.x, pos.y, pos.z))
        {
            *front_text = front;
            *back_text = back;
            *is_editable = false;
        }

        // Broadcast updated sign to all players
        if let Some(be) = self.block_entities.get(&(pos.x, pos.y, pos.z)) {
            let nbt = be.to_network_nbt(pos.x, pos.y, pos.z);
            self.broadcast_packet(
                packets::id::BLOCK_ACTOR_DATA,
                &BlockActorData {
                    position: pos,
                    nbt_data: nbt,
                },
            )
            .await;
        }

        debug!("Sign edited at {pos} by {addr}");
    }
}
