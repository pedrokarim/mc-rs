use std::collections::HashMap;
use std::net::SocketAddr;

use rand::Rng;

use mc_rs_proto::packets::player::{
    AddItemActor, ItemStack, ItemStackWrapper, MetadataValue, MoveActorAbsolute, RemoveEntity,
    SetActorMotion,
};

use crate::entity;
use crate::mob_entities::is_supporting_block;
use crate::player_registry::{self, PlayerRegistry};
use crate::world::chunk_cache::ChunkCache;

// The server ticks at 100 TPS (10ms), so Bedrock-like timings need to be scaled
// from the usual 20 TPS expectations.
const SERVER_TICKS_PER_SECOND: u32 = 100;
const BASELINE_TICKS_PER_SECOND: f32 = 20.0;
const SERVER_TICKS_PER_SECOND_F: f32 = 100.0;
const PICKUP_DELAY_TICKS: u32 = SERVER_TICKS_PER_SECOND * 2;
const DESPAWN_AFTER_TICKS: u32 = SERVER_TICKS_PER_SECOND * 300;
const PICKUP_RADIUS_SQ: f32 = 1.5 * 1.5;

// PMMP ItemEntity physics constants (per game tick @ 20 TPS):
//   gravity      = 0.04 blocks / tick² (half of mobs, ItemEntity::getInitialGravity)
//   drag         = 0.02 (applyDragBeforeGravity → v *= (1 - 0.02))
// Scaled to the server's 100 TPS tick rate.
const GRAVITY_PER_TICK: f32 = 0.04 * (BASELINE_TICKS_PER_SECOND / SERVER_TICKS_PER_SECOND_F);
const AIR_DRAG: f32 = 0.996; // ~= 0.98^(1/5)
const HORIZONTAL_DRAG: f32 = 0.992; // slightly stronger, simulates friction
const MAX_FALL_SPEED: f32 = -0.784;
const MOVEMENT_EPSILON: f32 = 1.0 / 128.0;

#[derive(Clone)]
pub struct PendingItemEntitySpawn {
    pub item: ItemStack,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

impl PendingItemEntitySpawn {
    /// Spawn with zero velocity (used for items that appear already at rest,
    /// e.g. persistence restore).
    pub fn stationary(item: ItemStack, position: [f32; 3]) -> Self {
        Self {
            item,
            position,
            velocity: [0.0, 0.0, 0.0],
        }
    }

    /// Spawn with PMMP-style scatter:
    ///   motion = (rand * 0.2 - 0.1, 0.2, rand * 0.2 - 0.1)
    /// See `World::dropItem()` in PMMP.
    pub fn with_scatter(item: ItemStack, position: [f32; 3]) -> Self {
        let mut rng = rand::thread_rng();
        let vx: f32 = rng.gen::<f32>() * 0.2 - 0.1;
        let vz: f32 = rng.gen::<f32>() * 0.2 - 0.1;
        Self {
            item,
            position,
            velocity: [vx, 0.2, vz],
        }
    }

    /// Spawn with a directed throw (drop from inventory / player toss).
    /// PMMP `Player::dropItem` uses motion = directionVector * 0.4 exactly.
    pub fn with_throw(item: ItemStack, position: [f32; 3], direction: [f32; 3]) -> Self {
        const THROW_FORCE: f32 = 0.4;
        Self {
            item,
            position,
            velocity: [
                direction[0] * THROW_FORCE,
                direction[1] * THROW_FORCE,
                direction[2] * THROW_FORCE,
            ],
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

    pub fn debug_summary(&self) -> String {
        format!(
            "entity_uid={} entity_rid={} item_id={} count={} meta={} block_rid={} extra_len={} extra_hex={} pos=({:.3},{:.3},{:.3}) vel=({:.3},{:.3},{:.3}) metadata={}",
            self.entity_unique_id,
            self.entity_runtime_id,
            self.item.id,
            self.item.count,
            self.item.meta,
            self.item.block_runtime_id,
            self.item.extra_data.len(),
            hex_preview(&self.item.extra_data, 32),
            self.position[0],
            self.position[1],
            self.position[2],
            self.velocity[0],
            self.velocity[1],
            self.velocity[2],
            metadata_debug_summary(&item_entity_metadata()),
        )
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
            tick: 0,
        }
        .encode()
    }
}

fn item_entity_metadata() -> Vec<(u32, u32, MetadataValue)> {
    entity::item_metadata()
}

pub fn item_stack_debug_summary(item: &ItemStack) -> String {
    format!(
        "item_id={} count={} meta={} block_rid={} extra_len={} extra_hex={}",
        item.id,
        item.count,
        item.meta,
        item.block_runtime_id,
        item.extra_data.len(),
        hex_preview(&item.extra_data, 32),
    )
}

pub fn hex_preview(bytes: &[u8], limit: usize) -> String {
    let shown = bytes
        .iter()
        .take(limit)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > limit {
        format!("{shown} ... (+{} bytes)", bytes.len() - limit)
    } else {
        shown
    }
}

fn metadata_debug_summary(metadata: &[(u32, u32, MetadataValue)]) -> String {
    metadata
        .iter()
        .map(|(key, ty, value)| match value {
            MetadataValue::Byte(v) => format!("{key}:{ty}=byte({v})"),
            MetadataValue::Short(v) => format!("{key}:{ty}=short({v})"),
            MetadataValue::Int(v) => format!("{key}:{ty}=int({v})"),
            MetadataValue::Float(v) => format!("{key}:{ty}=float({v:.3})"),
            MetadataValue::String(v) => format!("{key}:{ty}=string({v:?})"),
            MetadataValue::Long(v) => format!("{key}:{ty}=long({v})"),
        })
        .collect::<Vec<_>>()
        .join(", ")
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
        let mut movement_updates: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
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

            // ── Physics (PMMP ItemEntity::tryChangeMovement): drag, gravity, ground ──
            let old_position = entity.position;
            let old_velocity = entity.velocity;

            // Apply horizontal drag + vertical drag BEFORE gravity (PMMP
            // applyDragBeforeGravity = true for items).
            entity.velocity[0] *= HORIZONTAL_DRAG;
            entity.velocity[2] *= HORIZONTAL_DRAG;
            entity.velocity[1] *= AIR_DRAG;
            entity.velocity[1] = (entity.velocity[1] - GRAVITY_PER_TICK).max(MAX_FALL_SPEED);

            // Integrate position
            let mut next_x = entity.position[0] + entity.velocity[0];
            let mut next_y = entity.position[1] + entity.velocity[1];
            let mut next_z = entity.position[2] + entity.velocity[2];

            // Vertical collision against the supporting block directly below.
            let world_x = next_x.floor() as i32;
            let world_z = next_z.floor() as i32;
            let support_y = (next_y - 0.01).floor() as i32;
            let support_block = chunk_cache.get_block(world_x, support_y, world_z);
            let on_ground = is_supporting_block(support_block);
            if on_ground {
                let floor_y = support_y as f32 + 1.0;
                if next_y <= floor_y {
                    next_y = floor_y;
                    entity.velocity[1] = 0.0;
                    // Items lose horizontal momentum quickly once they settle.
                    entity.velocity[0] *= 0.6;
                    entity.velocity[2] *= 0.6;
                }
            }

            // Very small residual velocities are clamped so we don't jitter
            // forever broadcasting MoveActorAbsolute packets.
            if entity.velocity[0].abs() < 0.003 {
                entity.velocity[0] = 0.0;
                next_x = entity.position[0];
            }
            if entity.velocity[2].abs() < 0.003 {
                entity.velocity[2] = 0.0;
                next_z = entity.position[2];
            }

            entity.position[0] = next_x;
            entity.position[1] = next_y;
            entity.position[2] = next_z;

            let position_changed = (entity.position[0] - old_position[0]).abs() > MOVEMENT_EPSILON
                || (entity.position[1] - old_position[1]).abs() > MOVEMENT_EPSILON
                || (entity.position[2] - old_position[2]).abs() > MOVEMENT_EPSILON;
            let velocity_changed = (entity.velocity[0] - old_velocity[0]).abs() > MOVEMENT_EPSILON
                || (entity.velocity[1] - old_velocity[1]).abs() > MOVEMENT_EPSILON
                || (entity.velocity[2] - old_velocity[2]).abs() > MOVEMENT_EPSILON;

            if position_changed || velocity_changed {
                movement_updates.push((
                    entity.move_absolute_packet(on_ground && entity.velocity[1] == 0.0),
                    entity.motion_packet(),
                ));
            }

            // Pickup detection — only once the pickup delay has elapsed.
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

#[cfg(test)]
mod tests {
    use super::{ItemEntityManager, PendingItemEntitySpawn};
    use crate::item_registry;
    use crate::player_registry::PlayerRegistry;
    use crate::world::block_registry::BLOCKS;
    use crate::world::chunk_cache::ChunkCache;

    fn fresh_cache(name: &str) -> (std::path::PathBuf, ChunkCache) {
        let test_dir = std::env::temp_dir().join(format!(
            "mc-rs-item-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&test_dir);
        let cache = ChunkCache::new(&test_dir, 42, "normal");
        (test_dir, cache)
    }

    fn test_item() -> mc_rs_proto::packets::player::ItemStack {
        let item_id = item_registry::required_item_id("minecraft:dirt");
        mc_rs_proto::packets::player::ItemStack::new(item_id, 1, 9853)
    }

    #[test]
    fn dropped_items_fall_under_gravity_and_settle_on_the_ground() {
        let (test_dir, mut cache) = fresh_cache("gravity");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut items = ItemEntityManager::new();
        let entity = items.spawn(PendingItemEntitySpawn::stationary(
            test_item(),
            [0.5, 70.0, 0.5],
        ));
        let id = entity.entity_runtime_id;

        let registry = PlayerRegistry::new();
        let mut saw_movement = false;
        for _ in 0..500 {
            let tick = items.tick(&registry, &mut cache);
            if !tick.movement_updates.is_empty() {
                saw_movement = true;
            }
        }

        let settled = items
            .all()
            .find(|e| e.entity_runtime_id == id)
            .expect("item exists");
        assert!(
            saw_movement,
            "expected at least one movement update during the fall"
        );
        assert!(
            settled.position[1] < 70.0,
            "expected item to fall, got y={}",
            settled.position[1]
        );
        assert_eq!(
            settled.velocity[1], 0.0,
            "item should come to rest vertically"
        );
        let support_y = (settled.position[1] - 0.01).floor() as i32;
        assert!(
            super::is_supporting_block(cache.get_block(0, support_y, 0)),
            "expected item to rest on a supporting block (y={}, support_y={})",
            settled.position[1],
            support_y
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn resting_items_stop_broadcasting_movement_updates() {
        let (test_dir, mut cache) = fresh_cache("rest");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut items = ItemEntityManager::new();
        let _ = items.spawn(PendingItemEntitySpawn::stationary(
            test_item(),
            [0.5, 65.0, 0.5],
        ));

        let registry = PlayerRegistry::new();
        // Run long enough for any residual velocity to decay.
        for _ in 0..400 {
            let _ = items.tick(&registry, &mut cache);
        }
        // After settling, subsequent ticks should produce no movement packets.
        let quiet = items.tick(&registry, &mut cache);
        assert!(
            quiet.movement_updates.is_empty(),
            "expected no movement updates once the item has settled"
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn scatter_spawn_sends_an_initial_motion_packet() {
        let (test_dir, mut cache) = fresh_cache("scatter");
        cache.set_block(0, 64, 0, BLOCKS.stone);

        let mut items = ItemEntityManager::new();
        let _ = items.spawn(PendingItemEntitySpawn::with_scatter(
            test_item(),
            [0.5, 68.0, 0.5],
        ));

        let registry = PlayerRegistry::new();
        let tick = items.tick(&registry, &mut cache);
        assert!(
            !tick.movement_updates.is_empty(),
            "scattered spawn should produce a movement update immediately"
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
