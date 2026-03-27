use std::collections::HashMap;
use std::net::SocketAddr;

use mc_rs_proto::packets::player::{
    AddItemActor, ItemStack, ItemStackWrapper, MetadataValue, MoveActorAbsolute, RemoveEntity,
    SetActorMotion,
};

use crate::entity;
use crate::player_registry::{self, PlayerRegistry};
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;

// The server ticks at 100 TPS (10ms), so Bedrock-like timings need to be scaled up
// from the usual 20 TPS expectations.
const SERVER_TICKS_PER_SECOND: u32 = 100;
const BASELINE_TICKS_PER_SECOND: f32 = 20.0;
const GRAVITY_PER_TICK: f32 = 0.04 * (BASELINE_TICKS_PER_SECOND / SERVER_TICKS_PER_SECOND as f32);
const AIR_DRAG: f32 = 0.98;
const MAX_FALL_SPEED: f32 = -0.49;
const PICKUP_DELAY_TICKS: u32 = SERVER_TICKS_PER_SECOND * 2;
const DESPAWN_AFTER_TICKS: u32 = SERVER_TICKS_PER_SECOND * 300;
const PICKUP_RADIUS_SQ: f32 = 1.5 * 1.5;

#[derive(Clone)]
pub struct PendingItemEntitySpawn {
    pub item: ItemStack,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

impl PendingItemEntitySpawn {
    pub fn stationary(item: ItemStack, position: [f32; 3]) -> Self {
        Self {
            item,
            position,
            velocity: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Clone)]
pub struct ItemEntity {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub item: ItemStack,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub age_ticks: u32,
    pub pickup_delay_ticks: u32,
}

impl ItemEntity {
    pub fn add_actor_packet(&self) -> Vec<u8> {
        AddItemActor {
            entity_unique_id: self.entity_unique_id,
            entity_runtime_id: self.entity_runtime_id,
            item: ItemStackWrapper::legacy(self.item.clone()),
            position: self.position,
            velocity: self.velocity,
            metadata: item_entity_metadata(),
            is_from_fishing: false,
        }
        .encode()
    }

    pub fn remove_packet(&self) -> Vec<u8> {
        RemoveEntity {
            entity_unique_id: self.entity_unique_id,
        }
        .encode()
    }

    pub fn move_absolute_packet(&self, on_ground: bool) -> Vec<u8> {
        let mut flags = 0;
        if on_ground {
            flags |= MoveActorAbsolute::FLAG_GROUND;
        }
        MoveActorAbsolute {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            flags,
        }
        .encode()
    }

    pub fn motion_packet(&self) -> Vec<u8> {
        SetActorMotion {
            runtime_entity_id: self.entity_runtime_id,
            motion: self.velocity,
        }
        .encode()
    }
}

fn item_entity_metadata() -> Vec<(u32, u32, MetadataValue)> {
    entity::item_metadata()
}

pub struct PickupCandidate {
    pub player_addr: SocketAddr,
    pub entity_runtime_id: u64,
}

pub struct TickResult {
    pub despawned: Vec<ItemEntity>,
    pub pickup_candidates: Vec<PickupCandidate>,
    pub movement_updates: Vec<(Vec<u8>, Vec<u8>)>,
}

pub struct ItemEntityManager {
    entities: HashMap<u64, ItemEntity>,
}

impl Default for ItemEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemEntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, spawn: PendingItemEntitySpawn) -> ItemEntity {
        let entity_unique_id = player_registry::next_entity_id();
        let entity = ItemEntity {
            entity_unique_id,
            entity_runtime_id: entity_unique_id as u64,
            item: spawn.item,
            position: spawn.position,
            velocity: spawn.velocity,
            age_ticks: 0,
            pickup_delay_ticks: PICKUP_DELAY_TICKS,
        };
        self.entities
            .insert(entity.entity_runtime_id, entity.clone());
        entity
    }

    pub fn remove(&mut self, entity_runtime_id: u64) -> Option<ItemEntity> {
        self.entities.remove(&entity_runtime_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &ItemEntity> {
        self.entities.values()
    }

    pub fn tick(&mut self, registry: &PlayerRegistry, chunk_cache: &mut ChunkCache) -> TickResult {
        let mut despawned = Vec::new();
        let mut pickup_candidates = Vec::new();
        let mut movement_updates = Vec::new();
        let mut to_despawn = Vec::new();

        let ids: Vec<u64> = self.entities.keys().copied().collect();
        for entity_runtime_id in ids {
            let Some(entity) = self.entities.get_mut(&entity_runtime_id) else {
                continue;
            };

            entity.age_ticks = entity.age_ticks.saturating_add(1);
            if entity.pickup_delay_ticks > 0 {
                entity.pickup_delay_ticks -= 1;
            }

            let old_position = entity.position;
            let old_velocity = entity.velocity;
            entity.velocity[1] = (entity.velocity[1] - GRAVITY_PER_TICK).max(MAX_FALL_SPEED);
            entity.velocity[1] *= AIR_DRAG;

            let mut next_y = entity.position[1] + entity.velocity[1];
            let world_x = entity.position[0].floor() as i32;
            let world_z = entity.position[2].floor() as i32;
            let support_y = (next_y - 0.01).floor() as i32;
            let on_ground = is_supporting_block(chunk_cache.get_block(world_x, support_y, world_z));
            if on_ground {
                let floor_y = support_y as f32 + 1.0;
                if next_y <= floor_y {
                    next_y = floor_y;
                    entity.velocity[1] = 0.0;
                }
            }
            entity.position[1] = next_y;

            let position_changed = (entity.position[1] - old_position[1]).abs() > 0.0001;
            let velocity_changed = (entity.velocity[1] - old_velocity[1]).abs() > 0.0001;
            if position_changed || velocity_changed {
                movement_updates.push((
                    entity.move_absolute_packet(on_ground && entity.velocity[1] == 0.0),
                    entity.motion_packet(),
                ));
            }

            if entity.age_ticks >= DESPAWN_AFTER_TICKS {
                to_despawn.push(entity_runtime_id);
                continue;
            }

            if entity.pickup_delay_ticks > 0 {
                continue;
            }

            let mut picked_by = None;
            for player in registry.players.values() {
                let dx = player.position[0] - entity.position[0];
                let dy = (player.position[1] - 1.62) - entity.position[1];
                let dz = player.position[2] - entity.position[2];
                let dist_sq = dx * dx + dy * dy + dz * dz;
                if dist_sq <= PICKUP_RADIUS_SQ {
                    picked_by = Some(player.addr);
                    break;
                }
            }

            if let Some(player_addr) = picked_by {
                pickup_candidates.push(PickupCandidate {
                    player_addr,
                    entity_runtime_id,
                });
            }
        }

        for entity_runtime_id in to_despawn {
            if let Some(entity) = self.entities.remove(&entity_runtime_id) {
                despawned.push(entity);
            }
        }

        TickResult {
            despawned,
            pickup_candidates,
            movement_updates,
        }
    }
}

fn is_supporting_block(runtime_id: u32) -> bool {
    runtime_id != BLOCKS.air
        && runtime_id != BLOCKS.water
        && runtime_id != BLOCKS.lava
        && runtime_id != BLOCKS.short_grass
        && runtime_id != BLOCKS.tall_grass
        && runtime_id != BLOCKS.fern
        && runtime_id != BLOCKS.large_fern
        && runtime_id != BLOCKS.dandelion
        && runtime_id != BLOCKS.poppy
        && runtime_id != BLOCKS.blue_orchid
        && runtime_id != BLOCKS.allium
        && runtime_id != BLOCKS.azure_bluet
        && runtime_id != BLOCKS.oxeye_daisy
        && runtime_id != BLOCKS.cornflower
        && runtime_id != BLOCKS.waterlily
        && runtime_id != BLOCKS.seagrass
        && runtime_id != BLOCKS.brown_mushroom
        && runtime_id != BLOCKS.red_mushroom
        && runtime_id != BLOCKS.reeds
}

#[cfg(test)]
mod tests {
    use super::{is_supporting_block, ItemEntityManager, PendingItemEntitySpawn};
    use crate::item_registry;
    use crate::player_registry::PlayerRegistry;
    use crate::world::block_registry::BLOCKS;
    use crate::world::chunk_cache::ChunkCache;

    #[test]
    fn item_entities_fall_to_the_ground() {
        let test_dir =
            std::env::temp_dir().join(format!("mc-rs-item-physics-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&test_dir);
        let mut cache = ChunkCache::new(&test_dir, 42, "normal");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut items = ItemEntityManager::new();
        let item_id = item_registry::required_item_id("minecraft:dirt");
        let item = mc_rs_proto::packets::player::ItemStack::new(item_id, 1, BLOCKS.dirt as i32);
        let entity = items.spawn(PendingItemEntitySpawn::stationary(item, [0.5, 70.0, 0.5]));

        let registry = PlayerRegistry::new();
        for _ in 0..200 {
            let _ = items.tick(&registry, &mut cache);
        }

        let settled = items
            .all()
            .find(|item| item.entity_runtime_id == entity.entity_runtime_id)
            .expect("item exists");
        assert!(settled.position[1] < 70.0, "expected item to fall");
        let support_y = (settled.position[1] - 0.01).floor() as i32;
        assert!(is_supporting_block(cache.get_block(0, support_y, 0)));
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
