use tracing::{debug, info, warn};

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::item_entities::PendingItemEntitySpawn;

use super::{Connection, PLAYER_INVENTORY_SCREEN_ID, PLAYER_INVENTORY_WINDOW_TYPE};

/// Compute the unit forward vector from the player's yaw+pitch (Bedrock angles,
/// in degrees). Matches PMMP `Entity::getDirectionVector`.
///   yaw = 0   → facing +Z (south)
///   yaw = 90  → facing -X (west)
///   yaw = 180 → facing -Z (north)
///   yaw = 270 → facing +X (east)
fn direction_vector(yaw_deg: f32, pitch_deg: f32) -> [f32; 3] {
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let xz = pitch.cos();
    [-yaw.sin() * xz, -pitch.sin(), yaw.cos() * xz]
}

impl Connection {
    pub(super) fn handle_item_stack_request(
        &mut self,
        request: &ItemStackRequest,
        responses: &mut Vec<Vec<u8>>,
    ) {
        use mc_rs_proto::packets::player::StackRequestAction;
        use mc_rs_proto::packets::world::{ItemStackResponse, ItemStackResponseContainer};

        debug!(
            "[{}] ItemStackRequest id={} actions={}",
            self.addr,
            request.request_id,
            request.actions.len()
        );

        let mut changed_containers: Vec<ItemStackResponseContainer> = Vec::new();

        for action in &request.actions {
            match action {
                StackRequestAction::Take {
                    count,
                    source,
                    destination,
                }
                | StackRequestAction::Place {
                    count,
                    source,
                    destination,
                } => {
                    let src_slot = self.resolve_slot(source.container_id, source.slot_id);
                    let dst_slot = self.resolve_slot(destination.container_id, destination.slot_id);

                    if let (Some(src_idx), Some(dst_idx)) = (src_slot, dst_slot) {
                        let take_count = *count;

                        // Take from source
                        let src_item = self.inventory.slots[src_idx].item.clone();
                        if src_item.is_air() || src_item.count < take_count as u16 {
                            continue;
                        }

                        // Place to destination
                        let dst_item = &self.inventory.slots[dst_idx].item;
                        if dst_item.is_air() {
                            // Move to empty slot
                            let mut new_item = src_item.clone();
                            new_item.count = take_count as u16;
                            let stack_id = self.inventory.next_stack_id();
                            self.inventory.slots[dst_idx] =
                                ItemStackWrapper::new(new_item, stack_id);
                        } else if dst_item.id == src_item.id && dst_item.meta == src_item.meta {
                            // Stack on same item
                            self.inventory.slots[dst_idx].item.count += take_count as u16;
                        } else {
                            continue; // Can't place here
                        }

                        // Reduce source
                        if self.inventory.slots[src_idx].item.count <= take_count as u16 {
                            self.inventory.slots[src_idx] = ItemStackWrapper::air();
                        } else {
                            self.inventory.slots[src_idx].item.count -= take_count as u16;
                        }

                        // Track changes for response
                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            src_idx,
                        );
                        self.add_slot_to_response(
                            &mut changed_containers,
                            destination.container_id,
                            destination.slot_id,
                            dst_idx,
                        );
                    }
                }
                StackRequestAction::Swap {
                    source,
                    destination,
                    ..
                } => {
                    let src_slot = self.resolve_slot(source.container_id, source.slot_id);
                    let dst_slot = self.resolve_slot(destination.container_id, destination.slot_id);

                    if let (Some(src_idx), Some(dst_idx)) = (src_slot, dst_slot) {
                        self.inventory.slots.swap(src_idx, dst_idx);

                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            src_idx,
                        );
                        self.add_slot_to_response(
                            &mut changed_containers,
                            destination.container_id,
                            destination.slot_id,
                            dst_idx,
                        );
                    }
                }
                StackRequestAction::Destroy { source, .. } => {
                    if let Some(slot_idx) = self.resolve_slot(source.container_id, source.slot_id) {
                        self.inventory.slots[slot_idx] = ItemStackWrapper::air();
                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            slot_idx,
                        );
                    }
                }
                StackRequestAction::Drop { count, source } => {
                    if let Some(slot_idx) = self.resolve_slot(source.container_id, source.slot_id) {
                        let current = self.inventory.slots[slot_idx].item.clone();
                        if !current.is_air() && current.count > 0 {
                            let drop_count = (*count as u16).min(current.count);
                            if drop_count > 0 {
                                // Build the dropped stack from the held slot.
                                let mut dropped = current.clone();
                                dropped.count = drop_count;

                                // Decrement (or clear) the source slot.
                                if current.count <= drop_count {
                                    self.inventory.slots[slot_idx] = ItemStackWrapper::air();
                                } else {
                                    self.inventory.slots[slot_idx].item.count -= drop_count;
                                }

                                // Queue the dropped item entity using the
                                // player's eye level + forward throw. PMMP:
                                // position = location + (0, 1.3, 0),
                                // motion   = directionVector * 0.4.
                                let dir = direction_vector(self.yaw, self.pitch);
                                let spawn_pos = [
                                    self.position[0] + dir[0] * 0.3,
                                    self.position[1] + 1.3,
                                    self.position[2] + dir[2] * 0.3,
                                ];
                                self.pending_item_spawns
                                    .push(PendingItemEntitySpawn::with_throw(
                                        dropped, spawn_pos, dir,
                                    ));
                                info!(
                                    "[{}] Dropped {} x item_id={} from slot {}",
                                    self.addr, drop_count, current.id, slot_idx
                                );
                            }
                        }
                        self.add_slot_to_response(
                            &mut changed_containers,
                            source.container_id,
                            source.slot_id,
                            slot_idx,
                        );
                    }
                }
                StackRequestAction::Unknown(_) => {}
            }
        }

        // Send response
        let response = ItemStackResponse::ok(request.request_id, changed_containers);
        responses.push(
            self.encode_compressed_packet(packet_id::ITEM_STACK_RESPONSE, &response.encode()),
        );
    }

    pub(super) fn inventory_screen_container_name(&self) -> FullContainerName {
        // Dragonfly (protocol 944) always uses container_id=0 in
        // FullContainerName for InventoryContent packets — NOT a dynamic ID.
        FullContainerName::new(0)
    }

    fn advance_player_inventory_window_id(&mut self) -> u8 {
        self.player_inventory_window_id = if self.player_inventory_window_id >= 99 {
            PLAYER_INVENTORY_SCREEN_ID
        } else {
            self.player_inventory_window_id + 1
        };
        self.player_inventory_window_id
    }

    /// Sync ALL inventories to the client. Matches dragonfly (gophertunnel 944):
    ///   sendInv(inv, WindowIDInventory=0)
    ///   sendInv(ui, WindowIDUI=124)          ← cursor + 2x2 crafting grid (54 slots)
    ///   sendInv(offHand, WindowIDOffHand=119)
    ///   sendInv(armour, WindowIDArmour=120)
    ///
    /// The UI inventory is critical — without it, the client has no cursor or
    /// crafting grid state, and opening the inventory UI (E key) crashes.
    pub(super) fn push_inventory_sync(&self, responses: &mut Vec<Vec<u8>>) {
        let fcn = self.inventory_screen_container_name();

        // Main inventory (36 slots)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(0, &self.inventory.slots, &fcn),
        ));

        // UI inventory (54 slots) — all air. Includes cursor, 2x2 crafting grid,
        // crafting output, and other UI slots. Dragonfly always sends this at
        // spawn; without it the client crashes when opening the inventory UI.
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_empty(124, 54, &fcn),
        ));

        // Off-hand (1 slot)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(
                119,
                std::slice::from_ref(&self.inventory.offhand),
                &fcn,
            ),
        ));

        // Armor (4 slots)
        responses.push(self.encode_compressed_packet(
            packet_id::INVENTORY_CONTENT,
            &InventoryContent::encode_items(120, &self.inventory.armor, &fcn),
        ));

        // MobEquipment (held item / hotbar slot)
        responses.push(self.encode_compressed_packet(
            packet_id::MOB_EQUIPMENT,
            &MobEquipment::encode_item(
                self.entity_runtime_id,
                self.inventory.held_item(),
                self.inventory.held_slot,
            ),
        ));
    }

    pub fn prepared_inventory_sync_packets(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        self.push_inventory_sync(&mut responses);
        responses
            .into_iter()
            .map(|response| self.prepare_for_send(response))
            .collect()
    }

    fn push_open_inventory_window_sync(&self, responses: &mut Vec<Vec<u8>>) {
        self.push_inventory_sync(responses);
    }

    pub(super) fn handle_inventory_transaction(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::player::InventoryTransactionData;

        let Ok(transaction) = InventoryTransaction::decode(reader) else {
            warn!("[{}] Failed to decode InventoryTransaction", self.addr);
            let mut responses = Vec::new();
            self.push_inventory_sync(&mut responses);
            return responses;
        };
        let InventoryTransaction {
            request_id: _request_id,
            changed_slots,
            data,
        } = transaction;

        let mut responses = Vec::new();

        match data {
            InventoryTransactionData::Normal { actions } => {
                // Legacy drop flow. PMMP: a drop transaction is a pair of
                // NetworkInventoryAction — one with source_type=SOURCE_CONTAINER
                // (the inventory change) and one with source_type=SOURCE_WORLD
                // whose new_item carries the dropped stack. We trust the world
                // entry, apply the container-side change authoritatively, then
                // spawn the item entity server-side.
                let mut dropped_any = false;
                for action in &actions {
                    if action.source_type != 2 || action.source_flags != Some(0) {
                        continue;
                    }
                    let dropped = action.new_item.item.clone();
                    if dropped.is_air() || dropped.count == 0 {
                        continue;
                    }
                    dropped_any = true;

                    // Remove the corresponding stack from the authoritative
                    // inventory. We match the first container action whose
                    // old_item equals the dropped item; if not found, we fall
                    // back to decrementing the held slot.
                    let consumed = actions.iter().find(|a| {
                        a.source_type == 0
                            && !a.old_item.item.is_air()
                            && a.old_item.item.id == dropped.id
                            && a.old_item.item.count >= dropped.count
                    });
                    let slot_idx = if let Some(container_action) = consumed {
                        self.resolve_slot(0, container_action.inventory_slot as u8)
                    } else {
                        Some(self.inventory.held_slot as usize)
                    };
                    if let Some(idx) = slot_idx {
                        if idx < self.inventory.slots.len()
                            && !self.inventory.slots[idx].item.is_air()
                        {
                            let remaining = self.inventory.slots[idx]
                                .item
                                .count
                                .saturating_sub(dropped.count);
                            if remaining == 0 {
                                self.inventory.slots[idx] = ItemStackWrapper::air();
                            } else {
                                self.inventory.slots[idx].item.count = remaining;
                            }
                        }
                    }

                    let dir = direction_vector(self.yaw, self.pitch);
                    let spawn_pos = [
                        self.position[0] + dir[0] * 0.3,
                        self.position[1] + 1.3,
                        self.position[2] + dir[2] * 0.3,
                    ];
                    self.pending_item_spawns
                        .push(PendingItemEntitySpawn::with_throw(dropped, spawn_pos, dir));
                    info!(
                        "[{}] Legacy drop: {} x item_id={}",
                        self.addr, action.new_item.item.count, action.new_item.item.id
                    );
                }
                if dropped_any {
                    self.push_inventory_sync(&mut responses);
                }
            }
            InventoryTransactionData::Mismatch { .. } => {
                self.push_inventory_sync(&mut responses);
            }
            InventoryTransactionData::UseItem { .. }
            | InventoryTransactionData::ReleaseItem { .. }
            | InventoryTransactionData::Unknown { .. } => {}
            InventoryTransactionData::UseItemOnEntity {
                actor_runtime_id,
                action_type,
                hotbar_slot,
                ..
            } => {
                if (0..=8).contains(&hotbar_slot) {
                    self.inventory.held_slot = hotbar_slot as u8;
                }
                self.pending_entity_attacks
                    .push(super::PendingEntityAttack {
                        target_runtime_id: actor_runtime_id,
                        action_type,
                    });
                debug!(
                    "[{}] Queued entity interaction: target_runtime_id={} action_type={}",
                    self.addr, actor_runtime_id, action_type
                );
            }
        }

        if !changed_slots.is_empty() && responses.is_empty() {
            self.push_inventory_sync(&mut responses);
        }

        responses
    }

    /// Resolve a container_id + slot_id to an index in self.inventory.slots.
    fn resolve_slot(&self, container_id: u8, slot_id: u8) -> Option<usize> {
        match container_id {
            0 | 12 | 28 | 29 => {
                // Inventory / hotbar / combined inventory UI containers.
                let idx = slot_id as usize;
                if idx < 36 {
                    Some(idx)
                } else {
                    None
                }
            }
            _ => None, // Armor, offhand, etc. not handled yet
        }
    }

    /// Add a slot to the response containers.
    fn add_slot_to_response(
        &self,
        containers: &mut Vec<ItemStackResponseContainer>,
        container_id: u8,
        slot_id: u8,
        inventory_idx: usize,
    ) {
        let item = &self.inventory.slots[inventory_idx];
        let response_slot = ItemStackResponseSlot {
            slot: slot_id,
            hotbar_slot: slot_id,
            count: item.item.count as u8,
            stack_id: if item.item.is_air() { 0 } else { 1 },
            custom_name: String::new(),
            filtered_custom_name: String::new(),
            durability_correction: 0,
        };

        // Find or create the container
        if let Some(container) = containers
            .iter_mut()
            .find(|c| c.container_id == container_id)
        {
            container.slots.push(response_slot);
        } else {
            containers.push(ItemStackResponseContainer {
                container_id,
                slots: vec![response_slot],
            });
        }
    }

    pub(super) fn handle_interact(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(action) = reader.read_u8() else {
            return Vec::new();
        };
        let _actor_runtime_id = reader.read_var_u64().unwrap_or(0);

        info!("[{}] InteractPacket action={}", self.addr, action);

        if action == 6 {
            if self.player_inventory_open {
                return Vec::new();
            }
            self.player_inventory_open = true;

            // PMMP InventoryManager::onClientOpenMainInventory (line 394-408):
            //   windowId = getNewWindowId()           — dynamic 1-99
            //   ContainerOpenPacket::entityInv(windowId, WindowTypes::INVENTORY, player->getId())
            //     → windowType = -1 (0xFF)
            //     → blockPosition = BlockPosition(0, 0, 0)
            //     → actorUniqueId = player entity unique ID
            let window_id = self.advance_player_inventory_window_id();
            let container_open = ContainerOpen::entity_inventory(
                window_id,
                self.entity_runtime_id as i64,
            );
            info!(
                "[{}] Opening player inventory (PMMP entityInv: window_id={}, entity={})",
                self.addr, window_id, self.entity_runtime_id,
            );
            return vec![
                self.encode_compressed_packet(packet_id::CONTAINER_OPEN, &container_open.encode()),
            ];
        }

        Vec::new()
    }

    pub(super) fn handle_container_close(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let raw_window_id = reader.read_u8().unwrap_or(0);
        let window_type = reader.read_u8().unwrap_or(0);
        let _server = reader.read_bool().unwrap_or(false);

        let window_id = if raw_window_id == u8::MAX {
            self.player_inventory_window_id
        } else {
            raw_window_id
        };

        info!(
            "[{}] ContainerClose window_id={} window_type={}",
            self.addr, window_id, window_type
        );

        if window_id == self.player_inventory_window_id {
            self.player_inventory_open = false;
        }

        // Echo back the close
        let close = ContainerClose {
            window_id,
            window_type: if window_id == self.player_inventory_window_id {
                PLAYER_INVENTORY_WINDOW_TYPE
            } else {
                window_type
            },
            server: false,
        };
        vec![self.encode_compressed_packet(packet_id::CONTAINER_CLOSE, &close.encode())]
    }

    pub(super) fn handle_mob_equipment(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let _runtime_entity_id = reader.read_var_u64().unwrap_or(0);
        let _item_id = reader.read_var_i32().unwrap_or(0);
        let remaining = reader.read_remaining();
        if remaining.len() >= 3 {
            let hotbar_slot = remaining[remaining.len() - 2];
            if hotbar_slot < 9 {
                self.inventory.held_slot = hotbar_slot;
                debug!("[{}] Held slot changed to {}", self.addr, hotbar_slot);
            }
        }

        Vec::new()
    }
}
