//! ECS game world: bevy_ecs World, entity management, tick systems, and event bus.

use std::collections::HashMap;
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

/// O(1) mob lookup by runtime_id.
#[derive(Resource, Default)]
pub struct MobIndex(pub HashMap<u64, Entity>);

/// O(1) player lookup by unique_id.
#[derive(Resource, Default)]
pub struct PlayerIndex(pub HashMap<i64, Entity>);

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
    pub is_baby: bool,
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
        is_baby: bool,
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
    MobDied {
        runtime_id: u64,
        unique_id: i64,
        mob_type: String,
        killed_by: Option<u64>,
    },
    /// An entity was removed (despawn).
    EntityRemoved { unique_id: i64 },
    /// A mob attacks a player (melee).
    MobAttackPlayer {
        mob_runtime_id: u64,
        target_runtime_id: u64,
        damage: f32,
        knockback: (f32, f32, f32),
    },
    /// A mob shows love particles (breeding).
    MobLoveParticles { runtime_id: u64 },
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
        world.insert_resource(MobIndex::default());
        world.insert_resource(PlayerIndex::default());

        Self {
            world,
            mob_registry: MobRegistry::new(),
            spawn_config: SpawnConfig::default(),
        }
    }

    /// Run one game tick: AI, breeding, gravity, movement collection, dead cleanup, spawning.
    pub fn tick(&mut self) {
        self.world.resource_mut::<TickCounter>().0 += 1;
        system_ai_tick(&mut self.world);
        self.system_breeding_tick();
        system_mob_gravity(&mut self.world);
        system_collect_mob_moves(&mut self.world);
        system_cleanup_dead(&mut self.world);
        spawning::system_natural_spawn(&mut self.world, &self.mob_registry, &self.spawn_config);
        spawning::system_despawn_far_mobs(&mut self.world, &self.spawn_config);
    }

    /// Breeding system: pair in-love mobs, spawn babies, expire timers.
    fn system_breeding_tick(&mut self) {
        let current_tick = self.world.resource::<TickCounter>().0;

        // 1. Expire old InLove (> 600 ticks)
        let expired_love: Vec<Entity> = {
            let mut q = self.world.query_filtered::<(Entity, &InLove), With<Mob>>();
            q.iter(&self.world)
                .filter(|(_, love)| current_tick.saturating_sub(love.0) > 600)
                .map(|(e, _)| e)
                .collect()
        };
        for entity in expired_love {
            self.world.entity_mut(entity).remove::<InLove>();
        }

        // 2. Expire old BreedCooldown
        let expired_cd: Vec<Entity> = {
            let mut q = self
                .world
                .query_filtered::<(Entity, &BreedCooldown), With<Mob>>();
            q.iter(&self.world)
                .filter(|(_, cd)| current_tick >= cd.0)
                .map(|(e, _)| e)
                .collect()
        };
        for entity in expired_cd {
            self.world.entity_mut(entity).remove::<BreedCooldown>();
        }

        // 3. Baby → adult (> 24000 ticks)
        let grown_up: Vec<Entity> = {
            let mut q = self.world.query_filtered::<(Entity, &Baby), With<Mob>>();
            q.iter(&self.world)
                .filter(|(_, baby)| current_tick.saturating_sub(baby.0) > 24000)
                .map(|(e, _)| e)
                .collect()
        };
        for entity in grown_up {
            self.world.entity_mut(entity).remove::<Baby>();
        }

        // 4. Find breeding pairs (same type, both InLove, not Baby, close enough)
        let candidates: Vec<(Entity, u64, String, f32, f32, f32)> = {
            let mut q = self.world.query_filtered::<(
                Entity,
                &EntityId,
                &MobType,
                &Position,
                &InLove,
            ), (With<Mob>, Without<Dead>, Without<Baby>)>();
            q.iter(&self.world)
                .map(|(e, eid, mt, pos, _)| (e, eid.runtime_id, mt.0.clone(), pos.x, pos.y, pos.z))
                .collect()
        };

        let mut paired: Vec<Entity> = Vec::new();
        let mut babies_to_spawn: Vec<(String, f32, f32, f32, u64, u64)> = Vec::new();

        for i in 0..candidates.len() {
            if paired.contains(&candidates[i].0) {
                continue;
            }
            for j in (i + 1)..candidates.len() {
                if paired.contains(&candidates[j].0) {
                    continue;
                }
                if candidates[i].2 != candidates[j].2 {
                    continue;
                }
                let dx = candidates[i].3 - candidates[j].3;
                let dz = candidates[i].5 - candidates[j].5;
                let dist_sq = dx * dx + dz * dz;
                if dist_sq <= 1.5 * 1.5 {
                    // Found a pair!
                    paired.push(candidates[i].0);
                    paired.push(candidates[j].0);
                    let mid_x = (candidates[i].3 + candidates[j].3) / 2.0;
                    let mid_y = (candidates[i].4 + candidates[j].4) / 2.0;
                    let mid_z = (candidates[i].5 + candidates[j].5) / 2.0;
                    babies_to_spawn.push((
                        candidates[i].2.clone(),
                        mid_x,
                        mid_y,
                        mid_z,
                        candidates[i].1,
                        candidates[j].1,
                    ));
                    break;
                }
            }
        }

        // Remove InLove + add BreedCooldown on paired entities
        for entity in &paired {
            self.world.entity_mut(*entity).remove::<InLove>();
            self.world
                .entity_mut(*entity)
                .insert(BreedCooldown(current_tick + 6000));
        }

        // Spawn babies and emit love particles for parents
        for (mob_type, x, y, z, parent1_rid, parent2_rid) in babies_to_spawn {
            self.spawn_baby_mob(&mob_type, x, y, z);
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::MobLoveParticles {
                    runtime_id: parent1_rid,
                });
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::MobLoveParticles {
                    runtime_id: parent2_rid,
                });
        }
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

        let entity = self
            .world
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
                LastAttacker(None),
                MovementSpeed(def.movement_speed),
                BehaviorList::new(mob_behaviors::create_behaviors(type_id)),
            ))
            .id();

        self.world
            .resource_mut::<MobIndex>()
            .0
            .insert(runtime_id, entity);

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
                is_baby: false,
            });

        Some((entity_id, runtime_id))
    }

    /// Spawn a baby mob entity. Returns `(unique_id, runtime_id)` or `None` if type unknown.
    pub fn spawn_baby_mob(&mut self, type_id: &str, x: f32, y: f32, z: f32) -> Option<(i64, u64)> {
        let def = self.mob_registry.get(type_id)?.clone();
        let entity_id = self.world.resource::<EntityIdAllocator>().allocate();
        let runtime_id = entity_id as u64;
        let current_tick = self.world.resource::<TickCounter>().0;

        let entity = self
            .world
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
                LastAttacker(None),
                MovementSpeed(def.movement_speed),
                BehaviorList::new(mob_behaviors::create_behaviors(type_id)),
                Baby(current_tick),
            ))
            .id();

        self.world
            .resource_mut::<MobIndex>()
            .0
            .insert(runtime_id, entity);

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
                is_baby: true,
            });

        Some((entity_id, runtime_id))
    }

    /// Deal damage to a mob. Returns remaining health, or `None` if invulnerable or not found.
    ///
    /// `attacker_rid` is the runtime_id of the attacking entity (for XP attribution).
    pub fn damage_mob(
        &mut self,
        runtime_id: u64,
        damage: f32,
        tick: u64,
        attacker_rid: Option<u64>,
    ) -> Option<f32> {
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

        // Track attacker for XP attribution
        if attacker_rid.is_some() {
            if let Some(mut la) = self.world.get_mut::<LastAttacker>(target) {
                la.0 = attacker_rid;
            }
        }

        let eid = self.world.get::<EntityId>(target)?.clone();

        if new_health <= 0.0 {
            let mob_type = self
                .world
                .get::<MobType>(target)
                .map(|m| m.0.clone())
                .unwrap_or_default();
            let killed_by = self.world.get::<LastAttacker>(target).and_then(|la| la.0);
            self.world
                .resource_mut::<OutgoingEvents>()
                .events
                .push(GameEvent::MobDied {
                    runtime_id,
                    unique_id: eid.unique_id,
                    mob_type,
                    killed_by,
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
        let entity = self.find_mob_entity(runtime_id)?;
        let pos = self.world.get::<Position>(entity)?;
        Some((pos.x, pos.y, pos.z))
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
            self.world.resource_mut::<MobIndex>().0.remove(&runtime_id);
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
            Option<&Baby>,
        ), (With<Mob>, Without<Dead>)>();
        for (eid, pos, rot, health, mob_type, bb, baby) in query.iter(&self.world) {
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
                is_baby: baby.is_some(),
            });
        }
        result
    }

    /// Update the ECS mirror position for a player.
    pub fn update_player_position(&mut self, unique_id: i64, x: f32, y: f32, z: f32) {
        let entity = match self
            .world
            .resource::<PlayerIndex>()
            .0
            .get(&unique_id)
            .copied()
        {
            Some(e) => e,
            None => return,
        };
        if let Some(mut pos) = self.world.get_mut::<Position>(entity) {
            pos.x = x;
            pos.y = y;
            pos.z = z;
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
        let entity = self
            .world
            .spawn((
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
            ))
            .id();
        self.world
            .resource_mut::<PlayerIndex>()
            .0
            .insert(unique_id, entity);
    }

    /// Despawn the ECS mirror entity for a player by unique_id.
    pub fn despawn_player(&mut self, unique_id: i64) {
        if let Some(entity) = self
            .world
            .resource_mut::<PlayerIndex>()
            .0
            .remove(&unique_id)
        {
            self.world.despawn(entity);
        }
    }

    /// Update the held item name for a player ECS mirror entity.
    pub fn update_player_held_item(&mut self, unique_id: i64, item_name: String) {
        let entity = match self
            .world
            .resource::<PlayerIndex>()
            .0
            .get(&unique_id)
            .copied()
        {
            Some(e) => e,
            None => return,
        };
        self.world
            .entity_mut(entity)
            .insert(HeldItemName(item_name));
    }

    /// Set a mob as "in love". Returns false if mob not found or on cooldown or baby.
    pub fn set_mob_in_love(&mut self, runtime_id: u64) -> bool {
        let entity = match self.find_mob_entity(runtime_id) {
            Some(e) => e,
            None => return false,
        };
        // Cannot breed if baby
        if self.world.get::<Baby>(entity).is_some() {
            return false;
        }
        // Cannot breed if on cooldown
        let current_tick = self.world.resource::<TickCounter>().0;
        if let Some(cd) = self.world.get::<BreedCooldown>(entity) {
            if current_tick < cd.0 {
                return false;
            }
        }
        self.world.entity_mut(entity).insert(InLove(current_tick));
        self.world
            .resource_mut::<OutgoingEvents>()
            .events
            .push(GameEvent::MobLoveParticles { runtime_id });
        true
    }

    /// Get a mob's type string by runtime_id.
    pub fn mob_type(&mut self, runtime_id: u64) -> Option<String> {
        let entity = self.find_mob_entity(runtime_id)?;
        self.world.get::<MobType>(entity).map(|m| m.0.clone())
    }

    /// Check if a mob is on breeding cooldown.
    pub fn is_mob_on_breed_cooldown(&mut self, runtime_id: u64) -> bool {
        let entity = match self.find_mob_entity(runtime_id) {
            Some(e) => e,
            None => return false,
        };
        let current_tick = self.world.resource::<TickCounter>().0;
        self.world
            .get::<BreedCooldown>(entity)
            .map(|cd| current_tick < cd.0)
            .unwrap_or(false)
    }

    /// Check if a mob is a baby.
    pub fn is_mob_baby(&mut self, runtime_id: u64) -> bool {
        let entity = match self.find_mob_entity(runtime_id) {
            Some(e) => e,
            None => return false,
        };
        self.world.get::<Baby>(entity).is_some()
    }

    /// Find a mob entity by runtime_id (O(1) via MobIndex).
    fn find_mob_entity(&mut self, runtime_id: u64) -> Option<Entity> {
        self.world
            .resource::<MobIndex>()
            .0
            .get(&runtime_id)
            .copied()
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
    let dead_entities: Vec<(Entity, u64)> = world
        .query_filtered::<(Entity, &EntityId), With<Dead>>()
        .iter(world)
        .map(|(e, eid)| (e, eid.runtime_id))
        .collect();
    for (entity, runtime_id) in &dead_entities {
        world.resource_mut::<MobIndex>().0.remove(runtime_id);
        world.despawn(*entity);
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

        let result = gw.damage_mob(rid, 5.0, 0, None);
        assert_eq!(result, Some(15.0));
    }

    #[test]
    fn invulnerability_frames() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();

        // First hit at tick 0
        gw.damage_mob(rid, 5.0, 0, None);
        // Second hit at tick 5 — should be blocked (< 10 ticks)
        let result = gw.damage_mob(rid, 5.0, 5, None);
        assert!(result.is_none());
        // Third hit at tick 10 — should work
        let result = gw.damage_mob(rid, 5.0, 10, None);
        assert_eq!(result, Some(10.0));
    }

    #[test]
    fn damage_to_death() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:chicken", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Chicken has 4 HP
        let result = gw.damage_mob(rid, 10.0, 0, None);
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
        gw.damage_mob(rid, 100.0, 0, None);
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

    #[test]
    fn set_mob_in_love() {
        let mut gw = GameWorld::new(1);
        // Advance tick so InLove(0) doesn't expire immediately
        gw.world.resource_mut::<TickCounter>().0 = 10;
        let (_, rid) = gw.spawn_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        assert!(gw.set_mob_in_love(rid));

        // Check love particles event was emitted
        let events = gw.drain_events();
        assert!(events.iter().any(
            |e| matches!(e, GameEvent::MobLoveParticles { runtime_id } if *runtime_id == rid)
        ));
    }

    #[test]
    fn set_mob_in_love_baby_rejected() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_baby_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        assert!(!gw.set_mob_in_love(rid));
    }

    #[test]
    fn spawn_baby_mob_has_baby_component() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_baby_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();

        assert!(gw.is_mob_baby(rid));

        let events = gw.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::MobSpawned { is_baby: true, .. })));
    }

    #[test]
    fn breeding_pair_spawns_baby() {
        let mut gw = GameWorld::new(1);
        gw.world.resource_mut::<TickCounter>().0 = 10;

        // Spawn two cows very close together
        let (_, rid1) = gw.spawn_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        let (_, rid2) = gw.spawn_mob("minecraft:cow", 0.5, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Make both in love
        gw.set_mob_in_love(rid1);
        gw.set_mob_in_love(rid2);
        gw.drain_events();

        // Tick to trigger breeding
        gw.tick();

        let events = gw.drain_events();
        // Should have spawned a baby
        let baby_spawned = events
            .iter()
            .any(|e| matches!(e, GameEvent::MobSpawned { is_baby: true, .. }));
        assert!(baby_spawned, "Expected a baby cow to be spawned");
    }

    #[test]
    fn baby_grows_up() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_baby_mob("minecraft:cow", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        assert!(gw.is_mob_baby(rid));

        // Advance past growth time (24000 ticks)
        gw.world.resource_mut::<TickCounter>().0 = 25000;
        gw.tick();

        assert!(!gw.is_mob_baby(rid));
    }

    #[test]
    fn mob_index_insert_and_lookup() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();

        // MobIndex should contain the entity
        assert!(gw.world.resource::<MobIndex>().0.contains_key(&rid));
        assert!(gw.find_mob_entity(rid).is_some());
    }

    #[test]
    fn mob_index_remove_on_death() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:chicken", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        // Kill it
        gw.damage_mob(rid, 100.0, 0, None);
        // Tick to trigger cleanup
        gw.tick();

        // MobIndex should no longer contain the entity
        assert!(!gw.world.resource::<MobIndex>().0.contains_key(&rid));
        assert!(gw.find_mob_entity(rid).is_none());
    }

    #[test]
    fn player_index_insert_and_lookup() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(42, 42, (0.0, 5.0, 0.0), addr);

        assert!(gw.world.resource::<PlayerIndex>().0.contains_key(&42));
    }

    #[test]
    fn player_index_remove_on_despawn() {
        let mut gw = GameWorld::new(1);
        let addr: std::net::SocketAddr = "127.0.0.1:19132".parse().unwrap();
        gw.spawn_player(42, 42, (0.0, 5.0, 0.0), addr);

        gw.despawn_player(42);

        assert!(!gw.world.resource::<PlayerIndex>().0.contains_key(&42));
    }

    #[test]
    fn mob_index_remove_on_explicit_remove() {
        let mut gw = GameWorld::new(1);
        let (_, rid) = gw.spawn_mob("minecraft:zombie", 0.0, 4.0, 0.0).unwrap();
        gw.drain_events();

        assert!(gw.remove_mob(rid));
        assert!(!gw.world.resource::<MobIndex>().0.contains_key(&rid));
    }
}
