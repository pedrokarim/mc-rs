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
                .push(PendingItemEntitySpawn::with_throw(dropped, spawn_pos, dir));
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
        info!(
            "[{}] InventoryTransaction id={} variant={} changed_slots_count={}",
            self.addr,
            request_id,
            match &data {
                InventoryTransactionData::Normal { .. } => "Normal",
                InventoryTransactionData::Mismatch { .. } => "Mismatch",
                InventoryTransactionData::UseItem { .. } => "UseItem",
                InventoryTransactionData::ReleaseItem { .. } => "ReleaseItem",
                InventoryTransactionData::UseItemOnEntity { .. } => "UseItemOnEntity",
                InventoryTransactionData::Unknown { .. } => "Unknown",
            },
            changed_slots.len(),
        );

        let mut responses = Vec::new();

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
            InventoryTransactionData::UseItem { data, .. } => {
                // PMMP handleUseItemTransaction — placement de bloc arrive ici
                // avec data.action_type = 0 (ACTION_CLICK_BLOCK). C'est le
                // SEUL endroit où le client envoie l'info placement en proto
                // 944 (le bit 34 de PlayerAuthInput n'est jamais set).
                info!(
                    "[{}] UseItem action_type={} block_pos=({},{},{}) face={} hotbar={}",
                    self.addr,
                    data.action_type,
                    data.block_position[0],
                    data.block_position[1],
                    data.block_position[2],
                    data.face,
                    data.hotbar_slot,
                );
                match data.action_type {
                    0 => {
                        // ACTION_CLICK_BLOCK : placement ou right-click sur bloc
                        // (bed, crafting table, coffre, etc.).
                        self.handle_block_place(&data, &mut responses);
                    }
                    1 => {
                        // ACTION_CLICK_AIR : use held item (food, potion, eat).
                        // Pas implémenté pour l'instant.
                    }
                    2 => {
                        // ACTION_BREAK_BLOCK : legacy break (avant server-auth
                        // block breaking). Normalement passe par PREDICT_DESTROY_BLOCK
                        // en proto 944. On ignore ici.
                    }
                    _ => {}
                }
            }
            InventoryTransactionData::ReleaseItem {
                action_type,
                hotbar_slot,
                ..
            } => {
                // PMMP `handleReleaseItemTransaction` :
                //   action_type 0 = RELEASE (shoot bow / throw trident)
                //   action_type 1 = CONSUME (finish eating)
                if (0..=8).contains(&hotbar_slot) {
                    self.inventory.held_slot = hotbar_slot as u8;
                }
                if action_type == 1 {
                    self.handle_consume_item(&mut responses);
                } else {
                    info!(
                        "[{}] ReleaseItem action_type={} hotbar={} (not implemented)",
                        self.addr, action_type, hotbar_slot
                    );
                }
            }
            InventoryTransactionData::Unknown { .. } => {}
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
            // PMMP 5.42.1 (protocol 944) `InventoryManager::onClientOpenMainInventory`
            // (référence confirmée fonctionnelle avec Bedrock 1.26.10) :
            //   entityInv(windowId=getNewWindowId(), type=INVENTORY=0xFF,
            //             actorUniqueId=player.getId(), pos=(0,0,0))
            // avec garde `player_inventory_open` (dragonfly confirme que double
            // ContainerOpen crashe le client).
            if self.player_inventory_open {
                return Vec::new();
            }
            self.player_inventory_open = true;

            let mut out: Vec<(u32, Vec<u8>)> = Vec::new();
            self.inventory_manager
                .on_client_open_main_inventory(self.entity_runtime_id as i64, &mut out);
            info!(
                "[{}] on_client_open_main_inventory (PMMP entityInv): {} packet(s)",
                self.addr,
                out.len(),
            );
            for (pkt_id, payload) in &out {
                let preview: String = payload
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                info!(
                    "[{}] E-key pkt 0x{:03X} len={} hex={}",
                    self.addr,
                    pkt_id,
                    payload.len(),
                    preview
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
                let bcast =
                    MobEquipment::encode_item(self.entity_runtime_id, &wrapper, hotbar_slot);
                self.broadcasts
                    .push((mc_rs_proto::packets::packet_id::MOB_EQUIPMENT, bcast));
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

    /// Tick game state : hunger, combat timers, attribute desync sync,
    /// dégâts environnementaux (drowning/lava), détection de mort.
    /// À appeler à chaque game-tick (20 TPS = 1 tick / 5 server-ticks).
    /// Retourne les paquets à envoyer au joueur (UpdateAttributes si desync).
    pub fn tick_game_state(&mut self) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::packet_id;

        // Combat timers (attack_time, no_damage_ticks).
        self.combat.tick();
        // Hunger (exhaustion drain + regen/starvation).
        self.hunger
            .tick(&mut self.attributes, self.current_difficulty);

        // Dégâts environnementaux (survival only).
        if self.gamemode == 0 && !self.dead {
            self.tick_environment_damage();
        }

        // Détection de mort — quand HEALTH tombe à 0, bascule en état "dead"
        // et envoie Respawn(SEARCHING_FOR_SPAWN) + DeathAnimation. Le joueur
        // doit alors cliquer Respawn côté client (PlayerAction::RESPAWN=7).
        let mut out = Vec::new();
        if !self.dead
            && self
                .attributes
                .must_get(crate::attribute::ids::HEALTH)
                .current_value
                <= 0.0
        {
            self.dead = true;
            let spawn = self.spawn_position;
            let runtime_id = self.entity_runtime_id;
            let death_anim = crate::combat_packets::death_animation(runtime_id);
            out.push(self.encode_compressed_packet(packet_id::ACTOR_EVENT, &death_anim));
            let respawn = crate::combat_packets::encode_respawn(
                spawn,
                crate::combat_packets::respawn_state::SEARCHING_FOR_SPAWN,
                runtime_id,
            );
            out.push(self.encode_compressed_packet(packet_id::RESPAWN, &respawn));
            info!("[{}] Player died — awaiting respawn action", self.addr);
        }

        // Drain désync → UpdateAttributesPacket si non-vide.
        let desync = self.attributes.drain_desync();
        if !desync.is_empty() {
            let payload = encode_update_attributes(self.entity_runtime_id, &desync);
            out.push(self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &payload));
        }
        out
    }

    /// Tick des dégâts environnementaux : drowning (eyes in water),
    /// lava (feet in lava), fire (on_fire ticks). Port simplifié PMMP
    /// `Living::entityBaseTick` + `Living::updateAir`.
    ///
    /// Fréquence : 20 TPS (appel par tick_game_state). Air supply max=300
    /// (15s). Sous l'eau : décrément d'air, dégâts 2 HP/s quand air≤0.
    /// Dans la lave : 4 HP toutes les 10 ticks (0.5s) + met le joueur en feu.
    fn tick_environment_damage(&mut self) {
        self.environment_tick = self.environment_tick.wrapping_add(1);

        let feet_x = self.position[0].floor() as i32;
        let feet_y = self.position[1].floor() as i32;
        let feet_z = self.position[2].floor() as i32;
        // Eyes ≈ feet + 1.62. Block contenant les yeux = feet_y + 1.
        let eye_block_y = (self.position[1] + 1.62).floor() as i32;

        let (eye_block, feet_block) = {
            let Ok(mut cache) = self.chunk_cache.lock() else {
                return;
            };
            (
                cache.get_block(feet_x, eye_block_y, feet_z),
                cache.get_block(feet_x, feet_y, feet_z),
            )
        };

        // ── Drowning ──
        let water_id = crate::world::block_registry::BLOCKS.water;
        if eye_block == water_id {
            self.air_supply = (self.air_supply - 1).max(-20);
            if self.air_supply <= 0 && self.environment_tick % 20 == 0 {
                // 2 HP de dégâts toutes les secondes.
                let hp = self
                    .attributes
                    .must_get(crate::attribute::ids::HEALTH)
                    .current_value;
                let new_hp = (hp - 2.0).max(0.0);
                self.attributes
                    .must_get_mut(crate::attribute::ids::HEALTH)
                    .set_value(new_hp, true);
                info!("[{}] Drowning damage: hp={:.1}", self.addr, new_hp);
            }
        } else {
            // Hors de l'eau → recharge rapide de l'air.
            if self.air_supply < 300 {
                self.air_supply = (self.air_supply + 4).min(300);
            }
        }

        // ── Lava ──
        let lava_id = crate::world::block_registry::BLOCKS.lava;
        if feet_block == lava_id && self.environment_tick % 10 == 0 {
            // 4 HP toutes les 10 ticks (0.5s) — PMMP `EntityCombustEvent` damage.
            let hp = self
                .attributes
                .must_get(crate::attribute::ids::HEALTH)
                .current_value;
            let new_hp = (hp - 4.0).max(0.0);
            self.attributes
                .must_get_mut(crate::attribute::ids::HEALTH)
                .set_value(new_hp, true);
            info!("[{}] Lava damage: hp={:.1}", self.addr, new_hp);
        }
    }

    /// Consomme l'item tenu (eat / drink). Restaure faim+saturation via
    /// `items_vanilla::nutrition/saturation` (PMMP `Player::eatFoodInHand`).
    /// Décrémente le stack de 1 (sauf en créatif).
    pub fn handle_consume_item(&mut self, _responses: &mut Vec<Vec<u8>>) {
        let slot = self.inventory.held_slot as usize;
        let item = self.inventory.slots[slot].item.clone();
        if item.is_air() {
            return;
        }
        let item_name = match crate::item_registry::item_name_by_id(item.id) {
            Some(n) => n,
            None => return,
        };
        // Récupère nutrition + saturation depuis items_vanilla
        let nutrition = crate::items_vanilla::nutrition(item_name).unwrap_or(0);
        let saturation = crate::items_vanilla::saturation(item_name).unwrap_or(0.0);
        if nutrition == 0 && !crate::items_vanilla::is_food(item_name) {
            return; // pas de la nourriture
        }

        let cur_hunger = self
            .attributes
            .must_get(crate::attribute::ids::HUNGER)
            .current_value;
        let cur_sat = self
            .attributes
            .must_get(crate::attribute::ids::SATURATION)
            .current_value;
        let new_hunger = (cur_hunger + nutrition as f32).min(20.0);
        let new_sat = (cur_sat + saturation * 2.0).min(new_hunger);
        self.attributes
            .must_get_mut(crate::attribute::ids::HUNGER)
            .set_value(new_hunger, true);
        self.attributes
            .must_get_mut(crate::attribute::ids::SATURATION)
            .set_value(new_sat, true);

        info!(
            "[{}] Consumed {} : hunger {:.0}→{:.0} sat {:.1}→{:.1}",
            self.addr, item_name, cur_hunger, new_hunger, cur_sat, new_sat
        );

        // Décrémente le stack de 1 sauf en créatif.
        if self.gamemode != 1 {
            let new_count = item.count.saturating_sub(1);
            let new_item = if new_count == 0 {
                mc_rs_proto::packets::player::ItemStack::AIR
            } else {
                let mut n = item.clone();
                n.count = new_count;
                n
            };
            self.inventory_manager.set_slot(
                &mut self.inventory,
                crate::inventory_manager::InvKey::Main,
                slot,
                new_item,
            );
        }
    }

    /// Respawn : restaure HEALTH/hunger/position, envoie Respawn(READY) au
    /// client + re-sync complet (attributes, SetActorData, abilities, inventory)
    /// comme PMMP `NetworkSession::onServerRespawn`. À appeler sur PlayerAction::RESPAWN.
    pub fn handle_respawn_request(&mut self) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::packet_id;
        use mc_rs_proto::packets::player::{MovePlayer, SetActorData, UpdateAbilities};

        if !self.dead {
            return Vec::new();
        }
        self.dead = false;

        // Restauration attributs (force desync pour resync client).
        for id in [
            crate::attribute::ids::HEALTH,
            crate::attribute::ids::HUNGER,
            crate::attribute::ids::SATURATION,
            crate::attribute::ids::EXHAUSTION,
            crate::attribute::ids::ABSORPTION,
        ] {
            let a = self.attributes.must_get_mut(id);
            a.reset_to_default();
            a.desynchronized = true;
        }

        // Téléportation au spawn + reset état environnemental.
        self.position = self.spawn_position;
        self.fall_peak_y = None;
        self.air_supply = 300;
        self.environment_tick = 0;

        let mut out = Vec::new();

        // PMMP onServerRespawn : syncAttributes.
        let desync = self.attributes.drain_desync();
        if !desync.is_empty() {
            let payload = encode_update_attributes(self.entity_runtime_id, &desync);
            out.push(self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &payload));
        }

        // PMMP onServerRespawn : sendData (SetActorData) — reset metadata (remove
        // DEAD flag si on en avait un). player_in_game() rétablit les flags vivants.
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_in_game(self.entity_runtime_id, &player_name);
        out.push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // PMMP onServerRespawn : syncAbilities — réapplique les abilities
        // selon le gamemode actuel (créatif = fly, survival = no fly, etc.).
        let is_op = self.is_op;
        let abilities = match self.gamemode {
            1 => UpdateAbilities::default_creative(self.entity_runtime_id as i64, is_op),
            3 => UpdateAbilities::default_spectator(self.entity_runtime_id as i64, is_op),
            _ => UpdateAbilities::default_survival(self.entity_runtime_id as i64, is_op),
        };
        out.push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // PMMP onServerRespawn : invManager.syncAll() — resync tous les
        // inventaires (utile si mort a modifié ou vidé l'inventaire).
        self.push_inventory_sync(&mut out);

        // Téléportation.
        let move_pkt = MovePlayer {
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
        out.push(self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode()));

        // Respawn(READY_TO_SPAWN) → le client se téléporte.
        let respawn = crate::combat_packets::encode_respawn(
            self.spawn_position,
            crate::combat_packets::respawn_state::READY_TO_SPAWN,
            self.entity_runtime_id,
        );
        out.push(self.encode_compressed_packet(packet_id::RESPAWN, &respawn));

        info!("[{}] Player respawned at spawn", self.addr);
        out
    }

    /// RespawnPacket C→S : reçu quand le client a terminé sa téléportation et
    /// envoie CLIENT_READY_TO_SPAWN (state=2). PMMP `DeathPacketHandler::handleRespawn`
    /// répond par un autre Respawn(READY_TO_SPAWN) pour confirmer la transition
    /// et débloquer l'écran de réapparition.
    pub(super) fn handle_client_respawn(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        use mc_rs_proto::packets::packet_id;

        // Payload : Vec3 position (f32×3), u8 state, VarU64 runtime_id.
        let _px = reader.read_f32_le().unwrap_or(0.0);
        let _py = reader.read_f32_le().unwrap_or(0.0);
        let _pz = reader.read_f32_le().unwrap_or(0.0);
        let state = reader.read_u8().unwrap_or(0);
        let _rid = reader.read_var_u64().unwrap_or(0);

        if state != crate::combat_packets::respawn_state::CLIENT_READY_TO_SPAWN {
            return Vec::new();
        }

        // Confirme au client qu'il peut reprendre la main.
        let respawn = crate::combat_packets::encode_respawn(
            self.spawn_position,
            crate::combat_packets::respawn_state::READY_TO_SPAWN,
            self.entity_runtime_id,
        );
        info!(
            "[{}] CLIENT_READY_TO_SPAWN ACK → sending READY_TO_SPAWN",
            self.addr
        );
        vec![self.encode_compressed_packet(packet_id::RESPAWN, &respawn)]
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
        w.write_f32_le(a.min_value); // DefaultMin
        w.write_f32_le(a.max_value); // DefaultMax
        w.write_f32_le(a.default_value);
        w.write_string(&a.id);
        w.write_var_u32(0); // mod count
    }
    w.write_var_u64(0); // tick
    w.into_bytes()
}
