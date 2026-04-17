use tracing::info;

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::item_entities::{item_stack_debug_summary, PendingItemEntitySpawn};
use crate::world::block_registry::BLOCKS;

use super::Connection;

impl Connection {
    pub(super) fn handle_player_auth_input(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = PlayerAuthInput::decode(reader) else {
            return Vec::new();
        };

        // Validate position (anti-cheat basics)
        if !pkt.position[0].is_finite()
            || !pkt.position[1].is_finite()
            || !pkt.position[2].is_finite()
        {
            return Vec::new(); // ignore invalid position
        }

        // Void kill check
        if pkt.position[1] < -128.0 {
            self.position = self.spawn_position;
            let reset = MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
                mode: 1, // reset
                on_ground: true,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Anti-fly: if player is moving UP by more than 1.5 blocks per tick,
        // clamp their Y to prevent fly hacking
        let dy = pkt.position[1] - self.position[1];
        if dy > 1.5 {
            // Player is rising too fast -- clamp Y to max jump height
            let clamped_y = self.position[1] + 1.5;
            self.position = [pkt.position[0], clamped_y, pkt.position[2]];
            // Send correction
            let reset = MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: pkt.pitch,
                yaw: pkt.yaw,
                head_yaw: pkt.head_yaw,
                mode: 1, // reset
                on_ground: false,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Update player position
        self.position = pkt.position;
        self.pitch = pkt.pitch;
        self.yaw = pkt.yaw;
        self.head_yaw = pkt.head_yaw;
        self.tick += 1;

        // Broadcast MovePlayer to all other players
        let move_pkt = MovePlayer {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            mode: 0, // normal
            on_ground: true,
            riding_runtime_id: 0,
            tick: self.tick,
        };
        self.broadcasts
            .push(self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode()));

        // Check if player moved to a new chunk -- queue chunks for tick-based sending
        let mut responses = Vec::new();
        let chunk_x = (self.position[0] as i32) >> 4;
        let chunk_z = (self.position[2] as i32) >> 4;

        if chunk_x != self.last_chunk_x || chunk_z != self.last_chunk_z {
            self.last_chunk_x = chunk_x;
            self.last_chunk_z = chunk_z;

            // CRITICAL: Tell client the new center of the chunk render area.
            let ncpu = NetworkChunkPublisherUpdate {
                position: [
                    self.position[0] as i32,
                    self.position[1] as i32,
                    self.position[2] as i32,
                ],
                radius: (self.view_distance * 16) as u32,
            };
            responses.push(self.encode_compressed_packet(
                packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                &ncpu.encode(),
            ));

            info!(
                "[{}] NCPU sent: pos=({},{},{}), radius={} blocks, chunk=({},{})",
                self.addr,
                self.position[0] as i32,
                self.position[1] as i32,
                self.position[2] as i32,
                self.view_distance * 16,
                chunk_x,
                chunk_z,
            );

            // PMMP: nextChunkOrderRun = 0 on chunk change
            self.chunk_order_countdown = 0;
        } else {
            // PMMP: nextChunkOrderRun = min(current, 20) on normal movement
            if self.chunk_order_countdown > 20 {
                self.chunk_order_countdown = 20;
            }
        }

        // Handle block actions (breaking/placing)
        for action in &pkt.block_actions {
            let bx = action.position[0];
            let by = action.position[1];
            let bz = action.position[2];
            // PMMP uses integer block position (cast to float), NOT +0.5 center
            let block_pos = [bx as f32, by as f32, bz as f32];
            let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

            match action.action_type {
                // START_BREAK (0) or CONTINUE_DESTROY_BLOCK (27)
                0 | 27 => {
                    let break_speed: f32 = {
                        let block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                            cache.get_block(bx, by, bz)
                        } else {
                            0
                        };
                        match block_id {
                            13079 => 0.0,         // bedrock -- unbreakable
                            12421 | 11669 => 1.0, // short grass/tall grass -- instant
                            _ => 1.0 / 30.0,      // default ~1.5s with hand
                        }
                    };

                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_START_BREAK,
                        position: block_pos,
                        event_data: (65535.0 * break_speed) as i32,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // ABORT_BREAK (1) or STOP_BREAK (2)
                1 | 2 => {
                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
                        event_data: 0,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // PREDICT_DESTROY_BLOCK (26)
                26 => {
                    let air_id = BLOCKS.air;

                    // Send BLOCK_STOP_BREAK to clear crack animation
                    let stop_event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
                        event_data: 0,
                    };
                    let stop_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &stop_event.encode());
                    responses.push(stop_bytes.clone());
                    self.broadcasts.push(stop_bytes);

                    // Get the old block ID and set to air
                    let old_block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                        let old = cache.get_block(bx, by, bz);
                        cache.set_block(bx, by, bz, air_id);
                        cache.save_chunk_now(bx.div_euclid(16), bz.div_euclid(16));
                        old
                    } else {
                        air_id
                    };

                    // Send UpdateBlock
                    let update = UpdateBlock {
                        position: action.position,
                        runtime_id: air_id,
                        flags: 3, // FLAG_NEIGHBORS | FLAG_NETWORK
                        layer: 0,
                    };
                    let update_bytes =
                        self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
                    responses.push(update_bytes.clone());
                    self.broadcasts.push(update_bytes);

                    // Send block destroy particles + sound
                    if old_block_id != air_id {
                        let level_event = LevelEvent {
                            event_id: LevelEvent::PARTICLE_DESTROY,
                            position: block_center,
                            event_data: old_block_id as i32,
                        };
                        let event_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_EVENT,
                            &level_event.encode(),
                        );
                        responses.push(event_bytes.clone());
                        self.broadcasts.push(event_bytes);

                        let sound = LevelSoundEvent::block_sound(
                            LevelSoundEvent::BREAK,
                            block_center,
                            old_block_id as i32,
                        );
                        let sound_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_SOUND_EVENT,
                            &sound.encode(),
                        );
                        responses.push(sound_bytes.clone());
                        self.broadcasts.push(sound_bytes);
                    }

                    // Spawn a dropped item entity
                    if old_block_id != air_id {
                        if let Some(drop_item) = crate::inventory::block_drop(old_block_id) {
                            let item_id = drop_item.id;
                            let drop_summary = item_stack_debug_summary(&drop_item);
                            self.pending_item_spawns
                                .push(PendingItemEntitySpawn::with_scatter(
                                    drop_item,
                                    [bx as f32 + 0.5, by as f32 + 0.25, bz as f32 + 0.5],
                                ));
                            info!(
                                "[{}] Queued dropped item entity: item_id={} at ({}, {}, {}) :: {}",
                                self.addr, item_id, bx, by, bz, drop_summary
                            );
                        } else {
                            info!("[{}] No drop for block {}", self.addr, old_block_id);
                        }
                    }

                    info!(
                        "[{}] Block broken at ({}, {}, {}) old_id={}",
                        self.addr, bx, by, bz, old_block_id
                    );

                    // PMMP `BlockBreakEvent` (post-break, pour monitoring plugins).
                    if let Ok(mut ev_mgr) = self.events.lock() {
                        let mut ev = crate::event::block::BlockBreakEvent {
                            player_addr: self.addr,
                            position: [bx, by, bz],
                            old_block_runtime_id: old_block_id,
                            new_block_runtime_id: air_id,
                            drops: Vec::new(),
                            xp_drop: 0,
                            cancelled: false,
                        };
                        ev_mgr.call(&mut ev);
                    }

                    // PMMP `Block::onAttackedByTool` / `Durable::applyDamage` :
                    // si le held item est un outil durable, on décrémente sa
                    // durabilité de 1 (PMMP standard pour chaque bloc cassé).
                    // Si l'outil casse, on le remplace par air.
                    if self.gamemode != 1 {
                        let held_slot = self.inventory.held_slot as usize;
                        let held = &mut self.inventory.slots[held_slot];
                        if let Some(info) = crate::durability::durable_info(held.item.id) {
                            let broken = crate::durability::apply_damage(&mut held.item, 1);
                            if broken {
                                info!(
                                    "[{}] Tool broken (id={}, tier={:?})",
                                    self.addr, held.item.id, info.tier
                                );
                                // Replace par air + track via manager.
                                let new_item = mc_rs_proto::packets::player::ItemStack::AIR;
                                self.inventory_manager.set_slot(
                                    &mut self.inventory,
                                    crate::inventory_manager::InvKey::Main,
                                    held_slot,
                                    new_item,
                                );
                            } else {
                                // Slot changé (meta a bougé) : push pending_sync.
                                let current_item = held.item.clone();
                                self.inventory_manager.set_slot(
                                    &mut self.inventory,
                                    crate::inventory_manager::InvKey::Main,
                                    held_slot,
                                    current_item,
                                );
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        // Handle block placement (item interaction with ACTION_CLICK_BLOCK)
        if let Some(ref interaction) = pkt.item_interaction {
            info!(
                "[{}] item_interaction: action_type={} face={} hotbar_slot={}",
                self.addr, interaction.action_type, interaction.face, interaction.hotbar_slot
            );
            if interaction.action_type == 0 {
                // ACTION_CLICK_BLOCK
                self.handle_block_place(interaction, &mut responses);
            }
        }

        // Handle inventory stack requests (slot movements)
        if let Some(ref request) = pkt.item_stack_request {
            info!(
                "[{}] item_stack_request id={} actions={}",
                self.addr, request.request_id, request.actions.len()
            );
            self.handle_item_stack_request(request, &mut responses);
        }

        responses
    }

    pub(super) fn handle_block_place(
        &mut self,
        interaction: &ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let held = &self.inventory.slots[self.inventory.held_slot as usize];
        if held.item.is_air() {
            return;
        }

        // Check if held item is a placeable block
        let block_runtime_id = match crate::inventory::item_to_block(held.item.id) {
            Some(id) => id,
            None => return, // Not a block item
        };

        // Calculate target position from face offset
        let (dx, dy, dz) = match interaction.face {
            0 => (0, -1, 0), // down
            1 => (0, 1, 0),  // up
            2 => (0, 0, -1), // north
            3 => (0, 0, 1),  // south
            4 => (-1, 0, 0), // west
            5 => (1, 0, 0),  // east
            _ => return,
        };

        let tx = interaction.block_position[0] + dx;
        let ty = interaction.block_position[1] + dy;
        let tz = interaction.block_position[2] + dz;

        // Set the block
        if let Ok(mut cache) = self.chunk_cache.lock() {
            let existing = cache.get_block(tx, ty, tz);
            if existing != BLOCKS.air {
                return; // Can't place on a non-air block
            }
            cache.set_block(tx, ty, tz, block_runtime_id);
            cache.save_chunk_now(tx.div_euclid(16), tz.div_euclid(16));
        }

        // Send UpdateBlock
        let update = UpdateBlock {
            position: [tx, ty, tz],
            runtime_id: block_runtime_id,
            flags: 3,
            layer: 0,
        };
        let update_bytes = self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
        responses.push(update_bytes.clone());
        self.broadcasts.push(update_bytes);

        // Send place sound
        let block_center = [tx as f32 + 0.5, ty as f32 + 0.5, tz as f32 + 0.5];
        let sound = LevelSoundEvent::block_sound(
            LevelSoundEvent::PLACE,
            block_center,
            block_runtime_id as i32,
        );
        let sound_bytes =
            self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &sound.encode());
        responses.push(sound_bytes.clone());
        self.broadcasts.push(sound_bytes);

        // Decrement item count in inventory via manager (track + queue sync).
        let slot = self.inventory.held_slot as usize;
        let new_item = {
            let cur = &self.inventory.slots[slot].item;
            if cur.count > 1 {
                let mut n = cur.clone();
                n.count -= 1;
                n
            } else {
                mc_rs_proto::packets::player::ItemStack::AIR
            }
        };
        self.inventory_manager.set_slot(
            &mut self.inventory,
            crate::inventory_manager::InvKey::Main,
            slot,
            new_item,
        );
        // Le sync sera émis à la fin du tick via flush_pending_updates (boucle
        // principale). Pas d'envoi inline ici.

        info!(
            "[{}] Block placed at ({}, {}, {}) block_id={}",
            self.addr, tx, ty, tz, block_runtime_id
        );

        // PMMP `BlockPlaceEvent` (post-place).
        if let Ok(mut ev_mgr) = self.events.lock() {
            let mut ev = crate::event::block::BlockPlaceEvent {
                player_addr: self.addr,
                position: [tx, ty, tz],
                block_runtime_id,
                replaced_block_runtime_id: BLOCKS.air,
                cancelled: false,
            };
            ev_mgr.call(&mut ev);
        }
    }
}
