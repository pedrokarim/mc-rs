//! Natural mob spawning and despawning systems.

use bevy_ecs::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::game_world::{OutgoingEvents, TickCounter};
use crate::mob_registry::{MobCategory, MobRegistry};

use super::brain::BehaviorList;
use super::mob_behaviors;

/// Configuration for natural mob spawning.
pub struct SpawnConfig {
    /// Maximum hostile mobs in the world.
    pub hostile_cap: u32,
    /// Maximum passive mobs in the world.
    pub passive_cap: u32,
    /// Minimum distance from any player to spawn (blocks).
    pub min_distance: f32,
    /// Maximum distance from any player to spawn (blocks).
    pub max_distance: f32,
    /// Ticks between spawn attempts.
    pub spawn_interval: u64,
    /// Ticks between despawn checks.
    pub despawn_interval: u64,
    /// Distance beyond which mobs are despawned.
    pub despawn_distance: f32,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            hostile_cap: 20,
            passive_cap: 10,
            min_distance: 24.0,
            max_distance: 128.0,
            spawn_interval: 100,   // every 5 seconds
            despawn_interval: 200, // every 10 seconds
            despawn_distance: 128.0,
        }
    }
}

/// Periodically spawn mobs near players.
pub fn system_natural_spawn(world: &mut World, mob_registry: &MobRegistry, config: &SpawnConfig) {
    let tick = world.resource::<TickCounter>().0;
    if !tick.is_multiple_of(config.spawn_interval) || tick == 0 {
        return;
    }

    // Count current mobs by category
    let (hostile_count, passive_count) = count_mobs_by_category(world, mob_registry);

    // Get player positions
    let player_positions: Vec<(f32, f32, f32)> = {
        let mut q = world.query_filtered::<&Position, With<Player>>();
        q.iter(world).map(|p| (p.x, p.y, p.z)).collect()
    };

    if player_positions.is_empty() {
        return;
    }

    let mut rng = rand::thread_rng();

    // Spawn hostiles
    if hostile_count < config.hostile_cap {
        let hostile_types: Vec<&str> = mob_registry
            .all()
            .iter()
            .filter(|d| matches!(d.category, MobCategory::Hostile))
            .map(|d| d.type_id.as_str())
            .collect();

        if !hostile_types.is_empty() {
            let player = &player_positions[rng.gen_range(0..player_positions.len())];
            if let Some((x, z)) = random_spawn_position(
                &mut rng,
                player.0,
                player.2,
                config.min_distance,
                config.max_distance,
            ) {
                let type_id = hostile_types[rng.gen_range(0..hostile_types.len())];
                spawn_mob_internal(world, mob_registry, type_id, x, 4.0, z);
            }
        }
    }

    // Spawn passives
    if passive_count < config.passive_cap {
        let passive_types: Vec<&str> = mob_registry
            .all()
            .iter()
            .filter(|d| matches!(d.category, MobCategory::Passive))
            .map(|d| d.type_id.as_str())
            .collect();

        if !passive_types.is_empty() {
            let player = &player_positions[rng.gen_range(0..player_positions.len())];
            if let Some((x, z)) = random_spawn_position(
                &mut rng,
                player.0,
                player.2,
                config.min_distance,
                config.max_distance,
            ) {
                let type_id = passive_types[rng.gen_range(0..passive_types.len())];
                spawn_mob_internal(world, mob_registry, type_id, x, 4.0, z);
            }
        }
    }
}

/// Despawn mobs too far from all players.
pub fn system_despawn_far_mobs(world: &mut World, config: &SpawnConfig) {
    let tick = world.resource::<TickCounter>().0;
    if !tick.is_multiple_of(config.despawn_interval) || tick == 0 {
        return;
    }

    // Get player positions
    let player_positions: Vec<(f32, f32, f32)> = {
        let mut q = world.query_filtered::<&Position, With<Player>>();
        q.iter(world).map(|p| (p.x, p.y, p.z)).collect()
    };

    if player_positions.is_empty() {
        return;
    }

    // Find mobs too far from all players (using distance-squared to avoid sqrt)
    let despawn_dist_sq = config.despawn_distance * config.despawn_distance;
    let mut to_despawn: Vec<(Entity, i64, u64)> = Vec::new();
    {
        let mut q =
            world.query_filtered::<(Entity, &EntityId, &Position), (With<Mob>, Without<Dead>)>();
        for (entity, eid, pos) in q.iter(world) {
            let min_dist_sq = player_positions
                .iter()
                .map(|(px, _, pz)| {
                    let dx = pos.x - px;
                    let dz = pos.z - pz;
                    dx * dx + dz * dz
                })
                .fold(f32::MAX, f32::min);

            if min_dist_sq > despawn_dist_sq {
                to_despawn.push((entity, eid.unique_id, eid.runtime_id));
            }
        }
    }

    // Despawn and emit events
    for (entity, unique_id, runtime_id) in to_despawn {
        world
            .resource_mut::<crate::game_world::MobIndex>()
            .0
            .remove(&runtime_id);
        world
            .resource_mut::<OutgoingEvents>()
            .events
            .push(crate::game_world::GameEvent::EntityRemoved { unique_id });
        world.despawn(entity);
    }
}

/// Count mobs by category (hostile, passive).
fn count_mobs_by_category(world: &mut World, mob_registry: &MobRegistry) -> (u32, u32) {
    let mut hostile = 0u32;
    let mut passive = 0u32;

    let mut q = world.query_filtered::<&MobType, (With<Mob>, Without<Dead>)>();
    for mob_type in q.iter(world) {
        if let Some(def) = mob_registry.get(&mob_type.0) {
            match def.category {
                MobCategory::Hostile => hostile += 1,
                MobCategory::Passive => passive += 1,
            }
        }
    }
    (hostile, passive)
}

/// Pick a random spawn position within [min_dist, max_dist] of a player.
fn random_spawn_position(
    rng: &mut impl Rng,
    player_x: f32,
    player_z: f32,
    min_dist: f32,
    max_dist: f32,
) -> Option<(f32, f32)> {
    let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    let dist: f32 = rng.gen_range(min_dist..max_dist);
    let x = player_x + angle.cos() * dist;
    let z = player_z + angle.sin() * dist;
    Some((x, z))
}

/// Spawn a mob directly into the ECS world (used by natural spawn).
fn spawn_mob_internal(
    world: &mut World,
    mob_registry: &MobRegistry,
    type_id: &str,
    x: f32,
    y: f32,
    z: f32,
) {
    let def = match mob_registry.get(type_id) {
        Some(d) => d.clone(),
        None => return,
    };

    let entity_id = world
        .resource::<crate::game_world::EntityIdAllocator>()
        .allocate();
    let runtime_id = entity_id as u64;

    let entity = world
        .spawn((
            EntityId {
                unique_id: entity_id,
                runtime_id,
            },
            Position { x, y, z },
            Rotation {
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
            },
            Velocity {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: def.max_health,
                max: def.max_health,
            },
            OnGround(false),
            BoundingBox {
                width: def.bb_width,
                height: def.bb_height,
            },
            Mob,
            MobType(type_id.to_string()),
            AttackDamage(def.attack_damage),
            LastDamageTick(None),
            MovementSpeed(def.movement_speed),
            BehaviorList::new(mob_behaviors::create_behaviors(type_id)),
        ))
        .id();

    world
        .resource_mut::<crate::game_world::MobIndex>()
        .0
        .insert(runtime_id, entity);

    world
        .resource_mut::<OutgoingEvents>()
        .events
        .push(crate::game_world::GameEvent::MobSpawned {
            runtime_id,
            unique_id: entity_id,
            mob_type: type_id.to_string(),
            position: (x, y, z),
            health: def.max_health,
            max_health: def.max_health,
            bb_width: def.bb_width,
            bb_height: def.bb_height,
            is_baby: false,
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_world::GameWorld;

    #[test]
    fn spawn_respects_caps() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(50, 50, (0.0, 4.0, 0.0), addr);
        gw.drain_events();

        let config = SpawnConfig {
            hostile_cap: 2,
            passive_cap: 2,
            spawn_interval: 1,
            ..Default::default()
        };

        // Tick enough times for spawns
        for _ in 0..20 {
            gw.world.resource_mut::<TickCounter>().0 += 1;
            system_natural_spawn(&mut gw.world, &gw.mob_registry, &config);
        }

        let mobs = gw.all_mobs();
        // Should not exceed caps (2 hostile + 2 passive = 4 max)
        assert!(mobs.len() <= 4, "Too many mobs: {}", mobs.len());
    }

    #[test]
    fn no_spawn_without_players() {
        let mut gw = GameWorld::new(1);
        gw.drain_events();

        let config = SpawnConfig {
            spawn_interval: 1,
            ..Default::default()
        };

        gw.world.resource_mut::<TickCounter>().0 = 10;
        system_natural_spawn(&mut gw.world, &gw.mob_registry, &config);

        let mobs = gw.all_mobs();
        assert!(mobs.is_empty());
    }

    #[test]
    fn despawn_far_mobs() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(50, 50, (0.0, 4.0, 0.0), addr);

        // Spawn a mob very far away
        gw.spawn_mob("minecraft:cow", 500.0, 4.0, 500.0).unwrap();
        gw.drain_events();

        let config = SpawnConfig {
            despawn_interval: 1,
            despawn_distance: 128.0,
            ..Default::default()
        };

        gw.world.resource_mut::<TickCounter>().0 = 1;
        system_despawn_far_mobs(&mut gw.world, &config);

        // The far mob should have been despawned
        assert!(gw.all_mobs().is_empty());
    }

    #[test]
    fn spawn_produces_events() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(50, 50, (0.0, 4.0, 0.0), addr);
        gw.drain_events();

        let config = SpawnConfig {
            hostile_cap: 5,
            passive_cap: 5,
            spawn_interval: 1,
            ..Default::default()
        };

        gw.world.resource_mut::<TickCounter>().0 = 1;
        system_natural_spawn(&mut gw.world, &gw.mob_registry, &config);

        let events = gw.drain_events();
        let spawn_count = events
            .iter()
            .filter(|e| matches!(e, crate::game_world::GameEvent::MobSpawned { .. }))
            .count();
        // Should have spawned at least 1 mob (one hostile or passive attempt)
        assert!(
            spawn_count >= 1,
            "Expected spawn events, got {}",
            spawn_count
        );
    }
}
