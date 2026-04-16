use tracing::{debug, info, warn};

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::player::*;

use crate::item_entities::PendingItemEntitySpawn;

use super::Connection;

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
        debug!(
            "[{}] ItemStackRequest id={} actions={}",
            self.addr,
            request.request_id,
            request.actions.len()
        );

        // Délégué au manager (port complet ItemStackRequestExecutor + ResponseBuilder).
        let outcome = self
            .inventory_manager
            .process_item_stack_request(&mut self.inventory, request);

        // Items à drop physiquement → spawn item entities.
        for dropped in outcome.drops {
            let dir = direction_vector(self.yaw, self.pitch);
            let spawn_pos = [
                self.position[0] + dir[0] * 0.3,
                self.position[1] + 1.3,
                self.position[2] + dir[2] * 0.3,
            ];
            info!(
                "[{}] Drop via ItemStackRequest: {} x item_id={}",
                self.addr, dropped.count, dropped.id
            );
            self.pending_item_spawns
                .push(PendingItemEntitySpawn::with_throw(
                    dropped, spawn_pos, dir,
                ));
        }

        for (pkt_id, payload) in outcome.packets {
            responses.push(self.encode_compressed_packet(pkt_id, &payload));
        }
    }

    pub(super) fn inventory_screen_container_name(&self) -> FullContainerName {
        FullContainerName::new(self.inventory_manager.last_inventory_network_id)
    }

    /// Sync ALL inventories to the client via `InventoryManager`.
    /// Ordre dragonfly : Main → UI(124, 54 slots) → Offhand → Armor.
    /// Pas de MobEquipment au spawn (dragonfly ne l'envoie qu'au changement
    /// hotbar côté client).
    pub(super) fn push_inventory_sync(&mut self, responses: &mut Vec<Vec<u8>>) {
        let mut out: Vec<(u32, Vec<u8>)> = Vec::new();
        self.inventory_manager.sync_all(&self.inventory, &mut out);
        for (pkt_id, payload) in out {
            // DEBUG spawn dump
            let preview: String = payload
                .iter()
                .take(48)
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            info!(
                "[{}] spawn inv pkt 0x{:03X} len={} hex={}{}",
                self.addr,
                pkt_id,
                payload.len(),
                preview,
                if payload.len() > 48 { " ..." } else { "" },
            );
            responses.push(self.encode_compressed_packet(pkt_id, &payload));
        }
    }

    pub fn prepared_inventory_sync_packets(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        self.push_inventory_sync(&mut responses);
        responses
            .into_iter()
            .map(|response| self.prepare_for_send(response))
            .collect()
    }

    #[allow(dead_code)]
    fn push_open_inventory_window_sync(&mut self, responses: &mut Vec<Vec<u8>>) {
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
            request_id,
            changed_slots,
            data,
        } = transaction;

        let responses = Vec::new();

        // PMMP : `setCurrentItemStackRequestId` + `addRawPredictedSlotChanges`
        // doivent encadrer toute transaction legacy. Sans ça, les set_slot
        // server-side fired pendant le handle ne sont pas associés au requestId
        // et provoquent des resync inutiles.
        self.inventory_manager
            .set_current_item_stack_request_id(Some(request_id));
        let actions_for_pred: Vec<_> = match &data {
            InventoryTransactionData::Normal { actions }
            | InventoryTransactionData::Mismatch { actions }
            | InventoryTransactionData::UseItem { actions, .. }
            | InventoryTransactionData::ReleaseItem { actions, .. }
            | InventoryTransactionData::UseItemOnEntity { actions, .. }
            | InventoryTransactionData::Unknown { actions, .. } => actions.clone(),
        };
        self.inventory_manager
            .add_raw_predicted_slot_changes(&actions_for_pred);

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
                        let idx = container_action.inventory_slot as usize;
                        (idx < 36).then_some(idx)
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
                            let new_item = if remaining == 0 {
                                mc_rs_proto::packets::player::ItemStack::AIR
                            } else {
                                let mut n = self.inventory.slots[idx].item.clone();
                                n.count = remaining;
                                n
                            };
                            // Via manager : track + listener (matchera la prédiction
                            // posée plus haut, donc pas de pending_sync).
                            self.inventory_manager.set_slot(
                                &mut self.inventory,
                                crate::inventory_manager::InvKey::Main,
                                idx,
                                new_item,
                            );
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
                    // Le manager queue les pending_syncs ; le flush a lieu en
                    // fin de tick. Pas besoin d'un push_inventory_sync explicite.
                }
            }
            InventoryTransactionData::Mismatch { .. } => {
                self.inventory_manager.request_sync_all();
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

        // PMMP : pour chaque requestChangedSlots, force resync du slot.
        for cs in &changed_slots {
            for net_slot in &cs.changed_slots {
                if let Some((key, core)) = self
                    .inventory_manager
                    .locate_window_and_slot(cs.container_id, *net_slot as u32)
                {
                    if let Some(item) = self.inventory.slot_ref(key, core).cloned() {
                        if let Some(entry) = self.inventory_manager.inventories.get_mut(&key) {
                            entry.pending_syncs.insert(core, item.item);
                        }
                    }
                }
            }
        }

        // PMMP : finalisation transaction.
        self.inventory_manager
            .sync_mismatched_predicted_slot_changes(&self.inventory);
        self.inventory_manager
            .set_current_item_stack_request_id(None);

        responses
    }

    pub(super) fn handle_interact(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(action) = reader.read_u8() else {
            return Vec::new();
        };
        let _actor_runtime_id = reader.read_var_u64().unwrap_or(0);

        info!("[{}] InteractPacket action={}", self.addr, action);

        if action == 6 {
            // Protocol 944 dragonfly : WindowID=0, EntityUniqueID=-1, Position=player.
            // Voir inventory_manager::on_client_open_main_inventory.
            let mut out: Vec<(u32, Vec<u8>)> = Vec::new();
            self.inventory_manager
                .on_client_open_main_inventory(self.position, &mut out);
            info!(
                "[{}] on_client_open_main_inventory (dragonfly 944): {} packet(s)",
                self.addr,
                out.len(),
            );
            // DEBUG : dump hex des paquets bruts pour diagnostic client-side crash.
            for (pkt_id, payload) in &out {
                let preview: String = payload
                    .iter()
                    .take(64)
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                info!(
                    "[{}] E-key packet 0x{:03X} len={} hex={}{}",
                    self.addr,
                    pkt_id,
                    payload.len(),
                    preview,
                    if payload.len() > 64 { " ..." } else { "" },
                );
            }
            return out
                .into_iter()
                .map(|(id, p)| self.encode_compressed_packet(id, &p))
                .collect();
        }

        Vec::new()
    }

    pub(super) fn handle_container_close(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let raw_window_id = reader.read_u8().unwrap_or(0);
        let _window_type = reader.read_u8().unwrap_or(0);
        let _server = reader.read_bool().unwrap_or(false);

        info!(
            "[{}] ContainerClose raw_window_id={}",
            self.addr, raw_window_id
        );

        let mut out: Vec<(u32, Vec<u8>)> = Vec::new();
        self.inventory_manager
            .on_client_remove_window(raw_window_id, &mut out);
        self.player_inventory_open = false;

        out.into_iter()
            .map(|(id, p)| self.encode_compressed_packet(id, &p))
            .collect()
    }

    pub(super) fn handle_mob_equipment(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::player::MobEquipment;

        let _runtime_entity_id = reader.read_var_u64().unwrap_or(0);
        let _item_id = reader.read_var_i32().unwrap_or(0);
        let remaining = reader.read_remaining();
        if remaining.len() >= 3 {
            let hotbar_slot = remaining[remaining.len() - 2];
            let window_id = remaining[remaining.len() - 1];
            // PMMP: ignorer le windowId=OFFHAND (119), c'est juste un placement offhand.
            if window_id == 119 {
                return Vec::new();
            }
            if hotbar_slot < 9 {
                self.inventory.held_slot = hotbar_slot;
                self.inventory_manager
                    .on_client_select_hotbar_slot(hotbar_slot as i32);
                debug!("[{}] Held slot changed to {}", self.addr, hotbar_slot);

                // Broadcast aux autres viewers : ils doivent voir l'item tenu.
                let stack_id = self
                    .inventory_manager
                    .stack_id_of(crate::inventory_manager::InvKey::Main, hotbar_slot as usize);
                let wrapper = mc_rs_proto::packets::player::ItemStackWrapper {
                    stack_id,
                    item: self.inventory.slots[hotbar_slot as usize].item.clone(),
                };
                let bcast = MobEquipment::encode_item(self.entity_runtime_id, &wrapper, hotbar_slot);
                let bcast_pkt = self.encode_compressed_packet(
                    mc_rs_proto::packets::packet_id::MOB_EQUIPMENT,
                    &bcast,
                );
                self.broadcasts.push(bcast_pkt);
            }
        }

        Vec::new()
    }

    /// Vide les pending sync slots du manager. À appeler à chaque fin de tick
    /// pour propager au client toute mutation server-side (pickup, block place,
    /// command /give, etc.). PMMP `flushPendingUpdates`.
    pub fn tick_inventory_flush(&mut self) -> Vec<Vec<u8>> {
        let out = self
            .inventory_manager
            .flush_pending_updates(&self.inventory);
        out.into_iter()
            .map(|(id, p)| self.encode_compressed_packet(id, &p))
            .collect()
    }

    /// Tick game state : hunger, combat timers, attribute desync sync.
    /// À appeler à chaque game-tick (20 TPS = 1 tick / 5 server-ticks).
    /// Retourne les paquets à envoyer au joueur (UpdateAttributes si desync).
    pub fn tick_game_state(&mut self) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::packet_id;

        // Combat timers (attack_time, no_damage_ticks).
        self.combat.tick();
        // Hunger (exhaustion drain + regen/starvation).
        self.hunger
            .tick(&mut self.attributes, self.current_difficulty);

        // Drain désync → UpdateAttributesPacket si non-vide.
        let desync = self.attributes.drain_desync();
        let mut out = Vec::new();
        if !desync.is_empty() {
            let payload = encode_update_attributes(self.entity_runtime_id, &desync);
            out.push(self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &payload));
        }
        out
    }
}

/// Helper : encode un `UpdateAttributesPacket` à partir d'une liste d'attributs
/// désynchronisés. PMMP `UpdateAttributesPacket` format :
/// VarU64 actorRuntimeId, VarU32 count, { f32 min, f32 max, f32 current, f32 default,
/// string id, VarU32 modCount (=0) } × count, VarU64 tick.
fn encode_update_attributes(
    entity_runtime_id: u64,
    attrs: &[crate::attribute::Attribute],
) -> Vec<u8> {
    use mc_rs_proto::io::ProtoWriter;
    let mut w = ProtoWriter::with_capacity(128);
    w.write_var_u64(entity_runtime_id);
    w.write_var_u32(attrs.len() as u32);
    for a in attrs {
        // Protocol 944 : 6 float32 par attribut (Min, Max, Value,
        // DefaultMin, DefaultMax, Default). PMMP 924 n'avait que 4.
        w.write_f32_le(a.min_value);
        w.write_f32_le(a.max_value);
        w.write_f32_le(a.current_value);
        w.write_f32_le(a.min_value);  // DefaultMin
        w.write_f32_le(a.max_value);  // DefaultMax
        w.write_f32_le(a.default_value);
        w.write_string(&a.id);
        w.write_var_u32(0); // mod count
    }
    w.write_var_u64(0); // tick
    w.into_bytes()
}
