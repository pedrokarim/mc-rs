//! Cassage de blocs par un joueur.
//!
//! Port de PMMP : `InGamePacketHandler::handlePlayerActionFromData` (START_BREAK, CONTINUE_DESTROY_BLOCK,
//! ABORT_BREAK, STOP_BREAK, CRACK_BREAK, PREDICT_DESTROY_BLOCK) et `syncBlocksNearby`,
//! `Player::attackBlock` / `continueBreakBlock` / `stopBreakBlock` / `breakBlock` / `canInteract`,
//! `SurvivalBlockBreakHandler`, `World::useBreakOn`. Règles et données : `crate::block_break`.

use tracing::{debug, info};

use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::world::{LevelEvent, LevelSoundEvent, UpdateBlock};

use crate::block_break::{self, BreakProfile, HeldTool, MinerState};
use crate::enchantments::EnchantmentKind;
use crate::item_entities::PendingItemEntitySpawn;
use crate::world::block_registry::BLOCKS;

use super::Connection;

/// Actions de `PlayerBlockActions` traitées ici (PMMP `PlayerAction`).
pub(super) mod player_action {
    pub const START_BREAK: i32 = 0;
    pub const ABORT_BREAK: i32 = 1;
    pub const STOP_BREAK: i32 = 2;
    pub const CRACK_BREAK: i32 = 18;
    pub const CREATIVE_PLAYER_DESTROY_BLOCK: i32 = 13;
    pub const PREDICT_DESTROY_BLOCK: i32 = 26;
    pub const CONTINUE_DESTROY_BLOCK: i32 = 27;
}

/// PMMP `Player::MAX_REACH_DISTANCE_CREATIVE` / `MAX_REACH_DISTANCE_SURVIVAL`.
const MAX_REACH_DISTANCE_CREATIVE: f32 = 13.0;
const MAX_REACH_DISTANCE_SURVIVAL: f32 = 7.0;
/// Distance au-delà de laquelle le cassage en cours s'arrête (PMMP `attackBlock` : 16).
const MAX_BREAK_DISTANCE: f32 = 16.0;
/// PMMP `SurvivalBlockBreakHandler::DEFAULT_FX_INTERVAL_TICKS`.
const FX_INTERVAL_TICKS: u32 = 5;
/// PMMP `Entity::getEyeHeight` pour un joueur : la position reçue est celle des yeux.
const EYE_HEIGHT: f32 = 1.62;

/// Cassage en cours en survie (PMMP `SurvivalBlockBreakHandler`).
pub(crate) struct BlockBreakHandler {
    pos: [i32; 3],
    runtime_id: u32,
    face: i32,
    break_speed: f32,
    progress: f32,
    fx_ticker: u32,
}

fn level_event_position(pos: [i32; 3]) -> [f32; 3] {
    [pos[0] as f32, pos[1] as f32, pos[2] as f32]
}

fn block_center(pos: [i32; 3]) -> [f32; 3] {
    [
        pos[0] as f32 + 0.5,
        pos[1] as f32 + 0.5,
        pos[2] as f32 + 0.5,
    ]
}

/// Position voisine selon une face (PMMP `Facing` : 0 bas, 1 haut, 2 nord, 3 sud, 4 ouest, 5 est).
fn side(pos: [i32; 3], face: i32) -> [i32; 3] {
    let [x, y, z] = pos;
    match face {
        0 => [x, y - 1, z],
        1 => [x, y + 1, z],
        2 => [x, y, z - 1],
        3 => [x, y, z + 1],
        4 => [x - 1, y, z],
        _ => [x + 1, y, z],
    }
}

fn all_sides(pos: [i32; 3]) -> [[i32; 3]; 6] {
    [0, 1, 2, 3, 4, 5].map(|face| side(pos, face))
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

impl Connection {
    /// Traite une action de cassage ; `false` si l'action n'en relève pas.
    pub(super) fn handle_block_break_action(
        &mut self,
        action: i32,
        pos: [i32; 3],
        face: i32,
        responses: &mut Vec<Vec<u8>>,
    ) -> bool {
        use player_action::*;
        // PMMP `validateFacing` : une face hors de 0..=5 est une erreur de paquet.
        if matches!(
            action,
            START_BREAK | CONTINUE_DESTROY_BLOCK | CRACK_BREAK | PREDICT_DESTROY_BLOCK
        ) && !(0..=5).contains(&face)
        {
            debug!(
                "[{}] action {action} ignorée : face {face} invalide",
                self.addr
            );
            return true;
        }
        match action {
            START_BREAK | CONTINUE_DESTROY_BLOCK => {
                // Le client renvoie CONTINUE_DESTROY_BLOCK pour le bloc déjà ciblé juste avant
                // PREDICT_DESTROY_BLOCK : PMMP l'ignore, sinon l'animation repart de zéro.
                if self.last_block_attacked == Some(pos) {
                    debug!(
                        "[{}] action {action} ignorée sur {pos:?} : déjà en cours",
                        self.addr
                    );
                    return true;
                }
                if !self.attack_block(pos, face, responses) {
                    self.sync_blocks_nearby(pos, Some(face), responses);
                }
                self.last_block_attacked = Some(pos);
            }
            ABORT_BREAK | STOP_BREAK => {
                self.stop_break_block(pos, responses);
                self.last_block_attacked = None;
            }
            CRACK_BREAK => {
                if let Some(handler) = &mut self.block_break_handler {
                    if handler.pos == pos {
                        handler.face = face;
                    }
                }
                self.last_block_attacked = Some(pos);
            }
            // En cassage autoritatif serveur, PREDICT_DESTROY_BLOCK arrive aussi en créatif.
            CREATIVE_PLAYER_DESTROY_BLOCK => {}
            PREDICT_DESTROY_BLOCK => {
                if !self.break_block(pos, responses) {
                    self.sync_blocks_nearby(pos, Some(face), responses);
                }
                self.last_block_attacked = None;
            }
            _ => return false,
        }
        true
    }

    fn held_tool(&self) -> HeldTool {
        HeldTool::from_stack(&self.inventory.slots[self.inventory.held_slot as usize].item)
    }

    /// Conditions de cassage du joueur (PMMP `calculateBreakProgressPerTick`).
    fn miner_state(&self) -> MinerState {
        let helmet = self
            .inventory
            .armor
            .first()
            .map(|w| {
                crate::item_extra_data::enchantment_level(&w.item, EnchantmentKind::AquaAffinity)
            })
            .unwrap_or(0);
        MinerState {
            on_ground: self.on_ground,
            // PMMP n'autorise le vol (`allowFlight`) qu'en créatif et spectateur, modes où aucun
            // cassage progressif n'a lieu.
            flying: false,
            underwater_without_aqua_affinity: helmet == 0 && self.is_underwater(),
            // mc-rs ne tient pas encore d'effets par joueur : Célérité et Fatigue de minage restent
            // sans effet ici (dev-docs/36-CASSAGE-DE-BLOCS.md).
            haste: 0,
            mining_fatigue: 0,
        }
    }

    /// PMMP `Entity::isUnderwater` : yeux dans un bloc d'eau, sous la surface du liquide.
    fn is_underwater(&self) -> bool {
        let eye = self.position;
        let block_y = eye[1].floor();
        let block = match self.chunk_cache.lock() {
            Ok(mut cache) => {
                cache.get_block(eye[0].floor() as i32, block_y as i32, eye[2].floor() as i32)
            }
            Err(_) => return false,
        };
        match crate::liquids::water_height_percent(block) {
            Some(height) => eye[1] < block_y + 1.0 - (height - 0.111_111_1),
            None => false,
        }
    }

    fn break_progress_per_tick(&self, runtime_id: u32) -> f32 {
        block_break::profile(runtime_id).map_or(0.0, |profile| {
            block_break::break_progress_per_tick(profile, &self.held_tool(), &self.miner_state())
        })
    }

    /// PMMP `Player::canInteract` : distance aux yeux et cible devant le joueur.
    fn can_interact(&self, target: [f32; 3], max_distance: f32) -> bool {
        let eye = self.position;
        if distance_squared(eye, target) > max_distance * max_distance {
            return false;
        }
        let (pitch, yaw) = (self.pitch.to_radians(), self.yaw.to_radians());
        let xz = pitch.cos();
        let dir = [-xz * yaw.sin(), -pitch.sin(), xz * yaw.cos()];
        let dot = |v: [f32; 3]| dir[0] * v[0] + dir[1] * v[1] + dir[2] * v[2];
        dot(target) - dot(eye) >= -(3f32.sqrt() / 2.0)
    }

    /// Envoie un LevelEvent au joueur et aux autres (PMMP `broadcastPacketToViewers`).
    fn broadcast_level_event(
        &mut self,
        event_id: i32,
        position: [f32; 3],
        event_data: i32,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let payload = LevelEvent {
            event_id,
            position,
            event_data,
        }
        .encode();
        responses.push(self.encode_compressed_packet(packet_id::LEVEL_EVENT, &payload));
        self.broadcasts.push((packet_id::LEVEL_EVENT, payload));
    }

    fn broadcast_sound(&mut self, sound: LevelSoundEvent, responses: &mut Vec<Vec<u8>>) {
        let payload = sound.encode();
        responses.push(self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &payload));
        self.broadcasts
            .push((packet_id::LEVEL_SOUND_EVENT, payload));
    }

    fn set_block_and_broadcast(
        &mut self,
        pos: [i32; 3],
        runtime_id: u32,
        responses: &mut Vec<Vec<u8>>,
    ) {
        if let Ok(mut cache) = self.chunk_cache.lock() {
            cache.set_block(pos[0], pos[1], pos[2], runtime_id);
        }
        let payload = UpdateBlock {
            position: pos,
            runtime_id,
            flags: 3, // FLAG_NEIGHBORS | FLAG_NETWORK
            layer: 0,
        }
        .encode();
        responses.push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &payload));
        self.broadcasts.push((packet_id::UPDATE_BLOCK, payload));
    }

    /// Remplace le cassage en cours ; l'ancien envoie BLOCK_STOP_BREAK (PMMP
    /// `SurvivalBlockBreakHandler::__destruct`).
    fn replace_block_break_handler(
        &mut self,
        handler: Option<BlockBreakHandler>,
        responses: &mut Vec<Vec<u8>>,
    ) {
        if let Some(old) = std::mem::replace(&mut self.block_break_handler, handler) {
            self.broadcast_level_event(
                LevelEvent::BLOCK_STOP_BREAK,
                level_event_position(old.pos),
                0,
                responses,
            );
        }
    }

    /// PMMP `Player::attackBlock` : clic gauche sur un bloc ; `false` si l'action est refusée.
    fn attack_block(&mut self, pos: [i32; 3], face: i32, responses: &mut Vec<Vec<u8>>) -> bool {
        let feet = [
            self.position[0],
            self.position[1] - EYE_HEIGHT,
            self.position[2],
        ];
        if distance_squared(level_event_position(pos), feet) > 10_000.0 {
            return false;
        }
        // PlayerInteractEvent (LEFT_CLICK_BLOCK) : annulé en spectateur.
        if self.gamemode == 3 {
            return false;
        }
        let swing = crate::combat_packets::arm_swing(self.entity_runtime_id);
        self.broadcasts.push((packet_id::ANIMATE, swing));

        // Feu sur la face visée : éteint (PMMP `BlockTypeTags::FIRE`).
        let side_pos = side(pos, face);
        let side_id = self
            .chunk_cache
            .lock()
            .map(|mut cache| cache.get_block(side_pos[0], side_pos[1], side_pos[2]))
            .unwrap_or(BLOCKS.air);
        if matches!(
            BLOCKS.name_for(side_id),
            Some("minecraft:fire" | "minecraft:soul_fire")
        ) {
            self.set_block_and_broadcast(side_pos, BLOCKS.air, responses);
            self.broadcast_sound(
                LevelSoundEvent::block_sound(
                    LevelSoundEvent::EXTINGUISH_FIRE,
                    block_center(side_pos),
                    -1,
                ),
                responses,
            );
            return true;
        }

        let target = self
            .chunk_cache
            .lock()
            .map(|mut cache| cache.get_block(pos[0], pos[1], pos[2]))
            .unwrap_or(BLOCKS.air);
        let instant = block_break::profile(target).is_some_and(BreakProfile::breaks_instantly);
        if self.gamemode != 1 && !instant {
            let break_speed = self.break_progress_per_tick(target);
            if break_speed > 0.0 {
                self.broadcast_level_event(
                    LevelEvent::BLOCK_START_BREAK,
                    level_event_position(pos),
                    (65535.0 * break_speed) as i32,
                    responses,
                );
            }
            self.replace_block_break_handler(
                Some(BlockBreakHandler {
                    pos,
                    runtime_id: target,
                    face,
                    break_speed,
                    progress: 0.0,
                    fx_ticker: 0,
                }),
                responses,
            );
        }
        true
    }

    /// PMMP `Player::stopBreakBlock`.
    fn stop_break_block(&mut self, pos: [i32; 3], responses: &mut Vec<Vec<u8>>) {
        if self
            .block_break_handler
            .as_ref()
            .is_some_and(|h| h.pos == pos)
        {
            self.replace_block_break_handler(None, responses);
        }
    }

    /// Avance le cassage en cours d'un tick de jeu (PMMP `SurvivalBlockBreakHandler::update`, appelé
    /// par `Player::onUpdate`).
    pub(super) fn tick_block_break(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        let Some(handler) = &self.block_break_handler else {
            return responses;
        };
        let (pos, runtime_id, face, old_speed) = (
            handler.pos,
            handler.runtime_id,
            handler.face,
            handler.break_speed,
        );
        let feet = [
            self.position[0],
            self.position[1] - EYE_HEIGHT,
            self.position[2],
        ];
        if distance_squared(feet, block_center(pos)) > MAX_BREAK_DISTANCE * MAX_BREAK_DISTANCE {
            self.replace_block_break_handler(None, &mut responses);
            return responses;
        }

        let new_speed = self.break_progress_per_tick(runtime_id);
        if (new_speed - old_speed).abs() > 0.0001 {
            self.broadcast_level_event(
                LevelEvent::BLOCK_BREAK_SPEED,
                level_event_position(pos),
                (65535.0 * new_speed) as i32,
                &mut responses,
            );
        }
        let Some(handler) = &mut self.block_break_handler else {
            return responses;
        };
        handler.break_speed = new_speed;
        handler.progress += new_speed;
        let fx = handler.fx_ticker % FX_INTERVAL_TICKS == 0 && handler.progress < 1.0;
        handler.fx_ticker += 1;
        let done = handler.progress >= 1.0;

        if fx {
            // BlockPunchParticle, BlockPunchSound, ArmSwingAnimation aux autres joueurs.
            self.broadcast_level_event(
                LevelEvent::PARTICLE_PUNCH_BLOCK,
                level_event_position(pos),
                runtime_id as i32 | (face << 24),
                &mut responses,
            );
            self.broadcast_sound(
                LevelSoundEvent::block_sound(
                    LevelSoundEvent::HIT,
                    level_event_position(pos),
                    runtime_id as i32,
                ),
                &mut responses,
            );
            let swing = crate::combat_packets::arm_swing(self.entity_runtime_id);
            self.broadcasts.push((packet_id::ANIMATE, swing));
        }
        if done {
            self.replace_block_break_handler(None, &mut responses);
        }
        responses
    }

    /// PMMP `Player::breakBlock` : le client a cassé le bloc ; `false` s'il faut le resynchroniser.
    fn break_block(&mut self, pos: [i32; 3], responses: &mut Vec<Vec<u8>>) -> bool {
        let reach = if self.gamemode == 1 {
            MAX_REACH_DISTANCE_CREATIVE
        } else {
            MAX_REACH_DISTANCE_SURVIVAL
        };
        if !self.can_interact(block_center(pos), reach) {
            debug!("[{}] cassage refusé en {pos:?} : hors de portée", self.addr);
            return false;
        }
        let swing = crate::combat_packets::arm_swing(self.entity_runtime_id);
        self.broadcasts.push((packet_id::ANIMATE, swing));
        self.stop_break_block(pos, responses);
        if self.use_break_on(pos, responses) {
            // PMMP `HungerManager::exhaust(0.005, CAUSE_MINING)`.
            if self.gamemode == 0 || self.gamemode == 2 {
                self.hunger.exhaust(&mut self.attributes, 0.005);
            }
            return true;
        }
        false
    }

    /// PMMP `World::useBreakOn` avec un joueur : validation, destruction, drops, XP, usure de l'outil.
    fn use_break_on(&mut self, pos: [i32; 3], responses: &mut Vec<Vec<u8>>) -> bool {
        let air = BLOCKS.air;
        let target = match self.chunk_cache.lock() {
            Ok(mut cache) => cache.get_block(pos[0], pos[1], pos[2]),
            Err(_) => return false,
        };
        let Some(profile) = block_break::profile(target) else {
            return false;
        };
        let tool = self.held_tool();
        let creative = self.gamemode == 1;
        let finite_resources = !creative;

        // BlockBreakEvent annulé : air, bloc incassable en survie, spectateur, aventure sans CanDestroy.
        if target == air || self.gamemode == 3 || (self.gamemode == 0 && !profile.is_breakable()) {
            return false;
        }
        if self.gamemode == 2 && !self.held_item_can_destroy(target) {
            return false;
        }

        let drops = if finite_resources {
            block_break::drop_stacks(target, &tool, &mut rand::thread_rng())
        } else {
            Vec::new()
        };
        let xp = if finite_resources {
            profile.xp_drop(&tool, &mut rand::thread_rng())
        } else {
            0
        };

        let mut ev = crate::event::block::BlockBreakEvent {
            player_addr: self.addr,
            position: pos,
            old_block_runtime_id: target,
            new_block_runtime_id: air,
            drops,
            xp_drop: xp,
            cancelled: false,
        };
        if let Ok(mut events) = self.events.lock() {
            events.call(&mut ev);
        }
        if ev.cancelled {
            return false;
        }

        // `destroyBlockInternal` : particules de destruction, bloc remplacé par de l'air.
        self.broadcast_level_event(
            LevelEvent::PARTICLE_DESTROY,
            block_center(pos),
            target as i32,
            responses,
        );
        self.broadcast_sound(
            LevelSoundEvent::block_sound(LevelSoundEvent::BREAK, block_center(pos), target as i32),
            responses,
        );
        self.set_block_and_broadcast(pos, air, responses);
        if crate::furnace::FurnaceKind::from_block_id(target).is_some() {
            self.pending_furnace_events
                .push(super::PendingFurnaceEvent::Unregister {
                    pos: (pos[0], pos[1], pos[2]),
                });
        }

        // `Item::onDestroyBlock` : usure de l'outil.
        if finite_resources {
            self.wear_held_tool(block_break::destroy_damage(profile, &tool), tool.unbreaking);
        }

        let drop_pos = block_center(pos);
        for drop in ev.drops {
            self.pending_item_spawns
                .push(PendingItemEntitySpawn::with_scatter(drop, drop_pos));
        }
        if ev.xp_drop > 0 {
            self.pending_xp_drops.push((drop_pos, ev.xp_drop));
        }
        info!(
            "[{}] bloc cassé en {pos:?} : {} ({})",
            self.addr,
            BLOCKS.name_for(target).unwrap_or("?"),
            profile.class.as_deref().unwrap_or(&profile.source)
        );

        self.pop_unsupported_neighbors(pos, responses);
        true
    }

    /// Mode aventure : l'item tenu doit lister le bloc dans `CanDestroy` (PMMP `useBreakOn`, qui
    /// compare le type de bloc ; ici par identifiant, avec ou sans espace de noms).
    fn held_item_can_destroy(&self, target: u32) -> bool {
        let Some(name) = BLOCKS.name_for(target) else {
            return false;
        };
        let item = &self.inventory.slots[self.inventory.held_slot as usize].item;
        crate::item_extra_data::ItemExtraData::parse(&item.extra_data).is_some_and(|data| {
            data.can_destroy.iter().any(|entry| {
                entry == name || name.strip_prefix("minecraft:") == Some(entry.as_str())
            })
        })
    }

    /// Applique l'usure à l'item tenu (PMMP `Durable::applyDamage` avec Solidité).
    fn wear_held_tool(&mut self, amount: u32, unbreaking: u8) {
        if amount == 0 {
            return;
        }
        let amount =
            amount - block_break::unbreaking_reduction(amount, unbreaking, &mut rand::thread_rng());
        if amount == 0 {
            return;
        }
        let held_slot = self.inventory.held_slot as usize;
        let mut item = self.inventory.slots[held_slot].item.clone();
        if crate::durability::durable_info(item.id).is_none() {
            return;
        }
        let broken = crate::durability::apply_damage(&mut item, amount);
        let new_item = if broken {
            info!("[{}] outil cassé (id={})", self.addr, item.id);
            mc_rs_proto::packets::player::ItemStack::AIR
        } else {
            item
        };
        self.inventory_manager.set_slot(
            &mut self.inventory,
            crate::inventory_manager::InvKey::Main,
            held_slot,
            new_item,
        );
    }

    /// Blocs voisins qui ont perdu leur support (plantes, torches…) : cassés en cascade.
    fn pop_unsupported_neighbors(&mut self, pos: [i32; 3], responses: &mut Vec<Vec<u8>>) {
        let mut pop_queue = Vec::new();
        for [nx, ny, nz] in all_sides(pos) {
            if let Ok(mut cache) = self.chunk_cache.lock() {
                let nid = cache.get_block(nx, ny, nz);
                if nid == BLOCKS.air {
                    continue;
                }
                let Some(rule) = BLOCKS
                    .name_for(nid)
                    .and_then(crate::block_attachment::attachment_rule)
                else {
                    continue;
                };
                if !crate::block_attachment::check_support(&mut cache, nx, ny, nz, rule) {
                    pop_queue.push((nx, ny, nz));
                }
            }
        }
        for (nx, ny, nz) in pop_queue {
            self.pop_attached_block(nx, ny, nz, responses);
        }
    }

    /// PMMP `InGamePacketHandler::syncBlocksNearby` : renvoie les blocs autour d'une action refusée.
    fn sync_blocks_nearby(
        &mut self,
        pos: [i32; 3],
        face: Option<i32>,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let feet = [
            self.position[0],
            self.position[1] - EYE_HEIGHT,
            self.position[2],
        ];
        if distance_squared(level_event_position(pos), feet) >= 10_000.0 {
            return;
        }
        let mut blocks: Vec<[i32; 3]> = all_sides(pos).to_vec();
        match face {
            Some(face) => blocks.extend(all_sides(side(pos, face))),
            None => blocks.push(pos),
        }
        blocks.sort_unstable();
        blocks.dedup();
        let ids: Vec<u32> = match self.chunk_cache.lock() {
            Ok(mut cache) => blocks
                .iter()
                .map(|b| cache.get_block(b[0], b[1], b[2]))
                .collect(),
            Err(_) => return,
        };
        for (block, runtime_id) in blocks.into_iter().zip(ids) {
            let payload = UpdateBlock {
                position: block,
                runtime_id,
                flags: 3,
                layer: 0,
            }
            .encode();
            responses.push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &payload));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sides_follow_pmmp_facing() {
        assert_eq!(side([0, 0, 0], 0), [0, -1, 0]);
        assert_eq!(side([0, 0, 0], 1), [0, 1, 0]);
        assert_eq!(side([0, 0, 0], 2), [0, 0, -1]);
        assert_eq!(side([0, 0, 0], 3), [0, 0, 1]);
        assert_eq!(side([0, 0, 0], 4), [-1, 0, 0]);
        assert_eq!(side([0, 0, 0], 5), [1, 0, 0]);
    }
}
