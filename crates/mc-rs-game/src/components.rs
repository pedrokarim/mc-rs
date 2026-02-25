//! ECS components for all entities (players and mobs).

use std::net::SocketAddr;

use bevy_ecs::prelude::*;

/// Network identity for an entity.
#[derive(Component, Debug, Clone)]
pub struct EntityId {
    pub unique_id: i64,
    pub runtime_id: u64,
}

/// Position in the world.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Rotation angles in degrees.
#[derive(Component, Debug, Clone, Copy)]
pub struct Rotation {
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
}

/// Velocity vector.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Health points.
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

/// Whether the entity is standing on the ground.
#[derive(Component, Debug, Clone, Copy)]
pub struct OnGround(pub bool);

/// Axis-aligned bounding box dimensions.
#[derive(Component, Debug, Clone, Copy)]
pub struct BoundingBox {
    pub width: f32,
    pub height: f32,
}

/// The Bedrock identifier type string, e.g. `"minecraft:zombie"`.
#[derive(Component, Debug, Clone)]
pub struct MobType(pub String);

/// Base attack damage dealt by this mob.
#[derive(Component, Debug, Clone, Copy)]
pub struct AttackDamage(pub f32);

/// Tick when this entity last took damage (invulnerability frames).
#[derive(Component, Debug, Clone, Copy)]
pub struct LastDamageTick(pub Option<u64>);

/// Marker: this entity is a mob (non-player).
#[derive(Component, Debug)]
pub struct Mob;

/// Marker: this entity is a player.
#[derive(Component, Debug)]
pub struct Player;

/// Marker: this entity is dead (pending cleanup or respawn).
#[derive(Component, Debug)]
pub struct Dead;

/// Associates a player ECS entity with a network address.
#[derive(Component, Debug, Clone)]
pub struct NetworkAddr(pub SocketAddr);

/// Base movement speed in blocks/tick (copied from MobDefinition at spawn time).
#[derive(Component, Debug, Clone, Copy)]
pub struct MovementSpeed(pub f32);

/// The entity this mob is currently targeting.
#[derive(Component, Debug, Clone)]
pub struct AiTarget {
    pub entity: Entity,
    pub runtime_id: u64,
}
