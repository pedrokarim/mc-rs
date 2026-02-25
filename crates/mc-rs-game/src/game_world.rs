//! ECS game world: bevy_ecs World, entity management, tick systems, and event bus.

use std::sync::atomic::{AtomicI64, Ordering};

use bevy_ecs::prelude::*;

use crate::ai::brain::BehaviorList;
use crate::ai::mob_behaviors;
use crate::ai::spawning::{self, SpawnConfig};
use crate::ai::system::system_ai_tick;
use crate::components::*;
use crate::mob_registry::MobRegistry;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Outgoing events queued by ECS operations for the network layer to send.
#[derive(Resource, Default)]
pub struct OutgoingEvents {
    pub events: Vec<GameEvent>,
}

/// Global tick counter (incremented every 50 ms).
#[derive(Resource, Default)]
pub struct TickCounter(pub u64);

/// Thread-safe entity ID allocator (shared by mobs and players).
#[derive(Resource)]
pub struct EntityIdAllocator {
    next: AtomicI64,
}

impl EntityIdAllocator {
    pub fn new(start: i64) -> Self {
        Self {
            next: AtomicI64::new(start),
        }
    }

    /// Allocate the next unique entity ID.
    pub fn allocate(&self) -> i64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// Current value (next ID that will be allocated).
    pub fn current(&self) -> i64 {
        self.next.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Game events (ECS → network layer)
// ---------------------------------------------------------------------------

/// Snapshot of a mob for sending to new players.
#[derive(Debug, Clone)]
pub struct MobSnapshot {
    pub unique_id: i64,
    pub runtime_id: u64,
    pub mob_type: String,
    pub position: (f32, f32, f32),
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub bb_width: f32,
    pub bb_height: f32,
}

/// Events produced by the game world, consumed by the network layer.
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// A new mob was spawned — broadcast AddActor to all players.
    MobSpawned {
        runtime_id: u64,
        unique_id: i64,
        mob_type: String,
        position: (f32, f32, f32),
        health: f32,
        max_health: f32,
        bb_width: f32,
        bb_height: f32,
    },
    /// A mob moved — broadcast MoveActorAbsolute.
    MobMoved {
        runtime_id: u64,
        position: (f32, f32, f32),
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
        on_ground: bool,
    },
    /// A mob was damaged — broadcast EntityEvent(hurt) + UpdateAttributes.
    MobHurt {
        runtime_id: u64,
        new_health: f32,
        tick: u64,
    },
    /// A mob died — broadcast EntityEvent(death) + RemoveEntity.
    MobDied { runtime_id: u64, unique_id: i64 },
    /// An entity was removed (despawn).
    EntityRemoved { unique_id: i64 },
    /// A mob attacks a player (melee).
    MobAttackPlayer {
        mob_runtime_id: u64,
        target_runtime_id: u64,
        damage: f32,
        knockback: (f32, f32, f32),
    },
}

// ---------------------------------------------------------------------------
// GameWorld
// ---------------------------------------------------------------------------

/// The ECS game world.
pub struct GameWorld {
    pub world: World,
    pub mob_registry: MobRegistry,
    pub spawn_config: SpawnConfig,
}

impl GameWorld {
    /// Create a new game world with the given starting entity ID.
    pub fn new(starting_entity_id: i64) -> Self {
        let mut world = World::new();
        world.insert_resource(OutgoingEvents::default());
        world.insert_resource(TickCounter::default());
        world.insert_resource(EntityIdAllocator::new(starting_entity_id));

        Self {
            world,
            mob_registry: MobRegistry::new(),
            spawn_config: SpawnConfig::default(),
        }
    }

    /// Run one game tick: AI, gravity, movement collection, dead cleanup, spawning.
    pub fn tick(&mut self) {
        self.world.resource_mut::<TickCounter>().0 += 1;
        system_ai_tick(&mut self.world);
        system_mob_gravity(&mut self.world);
        system_collect_mob_moves(&mut self.world);
        system_cleanup_dead(&mut self.world);
        spawning::system_natural_spawn(&mut self.world, &self.mob_registry, &self.spawn_config);
        spawning::system_despawn_far_mobs(&mut self.world, &self.spawn_config);
    }

    /// Drain all pending outgoing events.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.world.resource_mut::<OutgoingEvents>().events)
    }

    /// Return the current tick count.
    pub fn current_tick(&mut self) -> u64 {
        self.world.resource::<TickCounter>().0
    }

    /// Allocate an entity ID (for players or mobs).
    pub fn allocate_entity_id(&self) -> i64 {
        self.world.resource::<EntityIdAllocator>().allocate()
    }

    /// Spawn a mob entity. Returns `(unique_id, runtime_id)` or `None` if type unknown.
    pub fn spawn_mob(&mut self, type_id: &str, x: f32, y: f32, z: f32) -> Option<(i64, u64)> {
        let def = self.mob_registry.get(type_id)?.clone();
        let entity_id = self.world.resource::<EntityIdAllocator>().allocate();
        let runtime_id = entity_id as u64;

        self.world.spawn((
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
        ));

        self.world
            .resource_mut::<OutgoingEvents>()
            .events
            .push(GameEvent::MobSpawned {
                runtime_id,
                unique_id: entity_id,
                mob_type: type_id.to_string(),
                position: (x, y, z),
                health: def.max_health,
                max_health: def.max_health,
                bb_width: def.bb_width,
                bb_height: def.bb_height,
            });

        Some((entity_id, runtime_id))
    }

    /// Deal damage to a mob. Returns remaining health, or `None` if invulnerable or not found.
    pub fn damage_mob(&mut self, runtime_id: u64, damage: f32, tick: u64) -> Option<f32> {
        let target = self.find_mob_entity(runtime_id)?;

        // Invulnerability check (10 ticks)
        if let Some(ldt) = self.world.get::<LastDamageTick>(target) {
            if let Some(last) = ldt.0 {
                if tick.saturating_sub(last) < 10 {
                    return None;
                }
            }
        }

        // Apply damage
        let new_health = {
            let mut health = self.world.get_mut::<Health>(target)?;
            health.current = (health.current - damage).max(0.0);
            health.current
        };

        // Update last damage tick
        if let Some(mut ldt) = self.world.get_mut::<LastDamageTick>(target) {
            ldt.0 = Some(tick);
        }

        let eid = self.world.get::<EntityId>(target)?.clone();

        if new_health <= 0.0 {
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::MobDied {
                    runtime_id,
                    unique_id: eid.unique_id,
                });
            self.world.entity_mut(target).insert(Dead);
        } else {
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::MobHurt {
                    runtime_id,
                    new_health,
                    tick,
                });
        }

        Some(new_health)
    }

    /// Apply knockback velocity to a mob.
    pub fn apply_knockback(&mut self, runtime_id: u64, vx: f32, vy: f32, vz: f32) {
        if let Some(entity) = self.find_mob_entity(runtime_id) {
            if let Some(mut vel) = self.world.get_mut::<Velocity>(entity) {
                vel.x = vx;
                vel.y = vy;
                vel.z = vz;
            }
        }
    }

    /// Check if a runtime_id belongs to a mob in the ECS.
    pub fn is_mob(&mut self, runtime_id: u64) -> bool {
        self.find_mob_entity(runtime_id).is_some()
    }

    /// Get a mob's position by runtime_id.
    pub fn mob_position(&mut self, runtime_id: u64) -> Option<(f32, f32, f32)> {
        let mut query = self.world.query::<(&EntityId, &Position)>();
        for (eid, pos) in query.iter(&self.world) {
            if eid.runtime_id == runtime_id {
                return Some((pos.x, pos.y, pos.z));
            }
        }
        None
    }

    /// Remove a mob by runtime_id. Returns `true` if found and removed.
    pub fn remove_mob(&mut self, runtime_id: u64) -> bool {
        if let Some(entity) = self.find_mob_entity(runtime_id) {
            let unique_id = self
                .world
                .get::<EntityId>(entity)
                .map(|e| e.unique_id)
                .unwrap_or(0);
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::EntityRemoved { unique_id });
            self.world.despawn(entity);
            true
        } else {
            false
        }
    }

    /// Get snapshots of all alive mobs (for sending to new players).
    pub fn all_mobs(&mut self) -> Vec<MobSnapshot> {
        let mut result = Vec::new();
        let mut query = self.world.query_filtered::<(
            &EntityId,
            &Position,
            &Rotation,
            &Health,
            &MobType,
            &BoundingBox,
        ), (With<Mob>, Without<Dead>)>();
        for (eid, pos, rot, health, mob_type, bb) in query.iter(&self.world) {
            result.push(MobSnapshot {
                unique_id: eid.unique_id,
                runtime_id: eid.runtime_id,
                mob_type: mob_type.0.clone(),
                position: (pos.x, pos.y, pos.z),
                pitch: rot.pitch,
                yaw: rot.yaw,
                head_yaw: rot.head_yaw,
                health: health.current,
                max_health: health.max,
                bb_width: bb.width,
                bb_height: bb.height,
            });
        }
        result
    }

    /// Update the ECS mirror position for a player.
    pub fn update_player_position(&mut self, unique_id: i64, x: f32, y: f32, z: f32) {
        let mut query = self
            .world
            .query_filtered::<(&EntityId, &mut Position), With<Player>>();
        for (eid, mut pos) in query.iter_mut(&mut self.world) {
            if eid.unique_id == unique_id {
                pos.x = x;
                pos.y = y;
                pos.z = z;
                return;
            }
        }
    }

    /// Spawn an ECS mirror entity for a player.
    pub fn spawn_player(
        &mut self,
        unique_id: i64,
        runtime_id: u64,
        position: (f32, f32, f32),
        addr: std::net::SocketAddr,
    ) {
        self.world.spawn((
            EntityId {
                unique_id,
                runtime_id,
            },
            Position {
                x: position.0,
                y: position.1,
                z: position.2,
            },
            Health {
                current: 20.0,
                max: 20.0,
            },
            Player,
            NetworkAddr(addr),
        ));
    }

    /// Despawn the ECS mirror entity for a player by unique_id.
    pub fn despawn_player(&mut self, unique_id: i64) {
        let mut to_despawn = None;
        let mut query = self
            .world
            .query_filtered::<(Entity, &EntityId), With<Player>>();
        for (entity, eid) in query.iter(&self.world) {
            if eid.unique_id == unique_id {
                to_despawn = Some(entity);
                break;
            }
        }
        if let Some(entity) = to_despawn {
            self.world.despawn(entity);
        }
    }

    /// Find a mob entity by runtime_id.
    fn find_mob_entity(&mut self, runtime_id: u64) -> Option<Entity> {
        let mut query = self
            .world
            .query_filtered::<(Entity, &EntityId), With<Mob>>();
        for (entity, eid) in query.iter(&self.world) {
            if eid.runtime_id == runtime_id {
                return Some(entity);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Systems (manual, called by GameWorld::tick)
// ---------------------------------------------------------------------------

/// Apply gravity to mobs not on the ground.
fn system_mob_gravity(world: &mut World) {
    const GRAVITY: f32 = 0.08;
    const FLAT_FLOOR_Y: f32 = 4.0; // feet position on flat world

    let mut query =
        world.query_filtered::<(&mut Position, &mut Velocity, &mut OnGround), (With<Mob>, Without<Dead>)>();

    // SAFETY: we need to iterate mutably; using `iter_mut` on the world.
    for (mut pos, mut vel, mut on_ground) in query.iter_mut(world) {
        vel.y -= GRAVITY;
        vel.y *= 0.98; // air drag

        pos.x += vel.x;
        pos.y += vel.y;
        pos.z += vel.z;

        // Horizontal drag
        vel.x *= 0.91;
        vel.z *= 0.91;

        // Simple floor collision (flat world)
        if pos.y <= FLAT_FLOOR_Y {
            pos.y = FLAT_FLOOR_Y;
            vel.y = 0.0;
            on_ground.0 = true;
        } else {
            on_ground.0 = false;
        }
    }
}

/// Collect position changes and emit MobMoved events.
fn system_collect_mob_moves(world: &mut World) {
    let mut moves = Vec::new();

    let mut query = world
        .query_filtered::<(&EntityId, &Position, &Rotation, &Velocity, &OnGround), (With<Mob>, Without<Dead>)>();

    for (eid, pos, rot, vel, on_ground) in query.iter(world) {
        if vel.x.abs() > 0.001 || vel.y.abs() > 0.001 || vel.z.abs() > 0.001 {
            moves.push(GameEvent::MobMoved {
                runtime_id: eid.runtime_id,
                position: (pos.x, pos.y, pos.z),
                pitch: rot.pitch,
                yaw: rot.yaw,
                head_yaw: rot.head_yaw,
                on_ground: on_ground.0,
            });
        }
    }

    world.resource_mut::<OutgoingEvents>().events.extend(moves);
}

/// Remove dead entities after their death events have been emitted.
fn system_cleanup_dead(world: &mut World) {
    let dead_entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Dead>>()
        .iter(world)
        .collect();
    for entity in dead_entities {
        world.despawn(entity);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_world_new() {
        let gw = GameWorld::new(1);
        assert_eq!(gw.world.resource::<TickCounter>().0, 0);
        assert_eq!(gw.world.resource::<EntityIdAllocator>().current(), 1);
    }

    #[test]
    fn spawn_mob_returns_ids() {
        let mut gw = GameWorld::new(100);
        let (uid, rid) = gw.spawn_mob("minecraft:zombie", 5.0, 10.0, 5.0).unwrap();
        assert_eq!(uid, 100);
        assert_eq!(rid, 100);

        let (uid2, rid2) = gw.spawn_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        assert_eq!(uid2, 101);
        assert_eq!(rid2, 101);
    }

    #[test]
    fn spawn_unknown_none() {
        let mut gw = GameWorld::new(1);
        assert!(gw.spawn_mob("minecraft:enderman", 0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn damage_reduces_health() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events(); // clear spawn event

        let result = gw.damage_mob(rid, 5.0, 0);
        assert_eq!(result, Some(15.0));
    }

    #[test]
    fn invulnerability_frames() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();

        // First hit at tick 0
        gw.damage_mob(rid, 5.0, 0);
        // Second hit at tick 5 — should be blocked (< 10 ticks)
        let result = gw.damage_mob(rid, 5.0, 5);
        assert!(result.is_none());
        // Third hit at tick 10 — should work
        let result = gw.damage_mob(rid, 5.0, 10);
        assert_eq!(result, Some(10.0));
    }

    #[test]
    fn damage_to_death() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:chicken", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Chicken has 4 HP
        let result = gw.damage_mob(rid, 10.0, 0);
        assert_eq!(result, Some(0.0));

        let events = gw.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::MobDied { .. })));
    }

    #[test]
    fn gravity_falls_to_floor() {
        let mut gw = GameWorld::new(1);
        gw.spawn_mob("minecraft:zombie", 0.0, 10.0, 0.0).unwrap();
        gw.drain_events();

        // Tick many times to let it fall
        for _ in 0..200 {
            gw.tick();
        }
        gw.drain_events(); // clear move events

        let mobs = gw.all_mobs();
        assert_eq!(mobs.len(), 1);
        // Should be at floor level (4.0)
        assert!((mobs[0].position.1 - 4.0).abs() < 0.01);
    }

    #[test]
    fn cleanup_removes_dead() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:chicken", 0.0, 4.0, 0.0).unwrap();

        // Kill it
        gw.damage_mob(rid, 100.0, 0);
        // Tick triggers cleanup
        gw.tick();

        assert!(gw.all_mobs().is_empty());
        assert!(!gw.is_mob(rid));
    }

    #[test]
    fn update_player_position_syncs() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(1, 1, (0.5, 5.62, 0.5), addr);

        // Update position
        gw.update_player_position(1, 10.0, 20.0, 30.0);

        // Verify the ECS position changed
        let mut query = gw
            .world
            .query_filtered::<(&EntityId, &Position), With<Player>>();
        let (_, pos) = query.iter(&gw.world).next().unwrap();
        assert!((pos.x - 10.0).abs() < 0.01);
        assert!((pos.y - 20.0).abs() < 0.01);
        assert!((pos.z - 30.0).abs() < 0.01);
    }

    #[test]
    fn spawn_mob_has_movement_speed() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();

        let mut query = gw
            .world
            .query_filtered::<(&EntityId, &MovementSpeed), With<Mob>>();
        for (eid, speed) in query.iter(&gw.world) {
            if eid.runtime_id == rid {
                assert!((speed.0 - 0.23).abs() < 0.01);
                return;
            }
        }
        panic!("MovementSpeed component not found on spawned mob");
    }
}
