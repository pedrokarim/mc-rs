use std::collections::HashMap;
use std::net::SocketAddr;

use mc_rs_proto::packets::player::{
    entity_flags, AddItemActor, ItemStack, ItemStackWrapper, MetadataValue, RemoveEntity,
};

use crate::player_registry::{self, PlayerRegistry};

// The server ticks at 100 TPS (10ms), so Bedrock-like timings need to be scaled up
// from the usual 20 TPS expectations.
const SERVER_TICKS_PER_SECOND: u32 = 100;
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
}

fn item_entity_metadata() -> Vec<(u32, u32, MetadataValue)> {
    let flags = entity_flags::HAS_GRAVITY | entity_flags::HAS_COLLISION;
    vec![
        (0, 7, MetadataValue::Long(flags)),
        (3, 0, MetadataValue::Byte(0)),
        (4, 4, MetadataValue::String(String::new())),
        (5, 7, MetadataValue::Long(-1)),
        (6, 7, MetadataValue::Long(0)),
        (37, 7, MetadataValue::Long(-1)),
        (38, 3, MetadataValue::Float(1.0)),
        (53, 3, MetadataValue::Float(0.25)),
        (54, 3, MetadataValue::Float(0.25)),
        (81, 0, MetadataValue::Byte(0)),
        (84, 4, MetadataValue::String(String::new())),
    ]
}

pub struct PickupCandidate {
    pub player_addr: SocketAddr,
    pub entity_runtime_id: u64,
}

pub struct TickResult {
    pub despawned: Vec<ItemEntity>,
    pub pickup_candidates: Vec<PickupCandidate>,
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

    pub fn tick(&mut self, registry: &PlayerRegistry) -> TickResult {
        let mut despawned = Vec::new();
        let mut pickup_candidates = Vec::new();
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
        }
    }
}
