//! Priority-based behavior trait for mob AI.

use bevy_ecs::prelude::Entity;

/// (entity, runtime_id, distance, (x, y, z)) of the nearest player.
pub type NearestPlayerInfo = (Entity, u64, f32, (f32, f32, f32));

/// What kind of output a behavior produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorType {
    /// Controls movement (only one active at a time).
    Movement,
    /// Non-movement (e.g., look at player). Multiple can co-exist.
    Passive,
    /// Target selector (picks who to attack). Only one active at a time.
    TargetSelector,
}

/// Read-only snapshot of the world state relevant to one mob.
pub struct BehaviorContext {
    /// Mob's current position.
    pub mob_position: (f32, f32, f32),
    /// Mob's base movement speed (blocks/tick).
    pub mob_speed: f32,
    /// Mob's base attack damage.
    pub mob_attack_damage: f32,
    /// Whether the mob is on the ground.
    pub mob_on_ground: bool,
    /// Current game tick.
    pub current_tick: u64,
    /// Tick when this mob was last damaged.
    pub last_damage_tick: Option<u64>,
    /// Current target: (entity, runtime_id, x, y, z).
    pub current_target: Option<(Entity, u64, f32, f32, f32)>,
    /// Nearest player info.
    pub nearest_player: Option<NearestPlayerInfo>,
}

/// Output actions from a behavior tick.
#[derive(Debug, Default)]
pub struct BehaviorOutput {
    /// Desired goal position to walk toward.
    pub move_to: Option<(f32, f32, f32)>,
    /// Desired yaw and head_yaw rotation (degrees).
    pub look_at: Option<(f32, f32)>,
    /// Attack the current target this tick.
    pub attack: bool,
    /// Set a new target entity (entity, runtime_id).
    pub set_target: Option<(Entity, u64)>,
    /// Clear the current target.
    pub clear_target: bool,
}

/// A single behavior in the priority list.
pub trait Behavior: Send + Sync + std::fmt::Debug {
    /// What kind of behavior this is.
    fn behavior_type(&self) -> BehaviorType;

    /// Priority (lower = higher priority).
    fn priority(&self) -> u32;

    /// Can this behavior start right now?
    fn can_start(&self, ctx: &BehaviorContext) -> bool;

    /// Should this behavior continue running?
    fn should_continue(&self, ctx: &BehaviorContext) -> bool {
        self.can_start(ctx)
    }

    /// Called once when the behavior activates.
    fn start(&mut self, _ctx: &BehaviorContext) -> BehaviorOutput {
        BehaviorOutput::default()
    }

    /// Called every tick while active.
    fn tick(&mut self, ctx: &BehaviorContext) -> BehaviorOutput;

    /// Called once when the behavior deactivates.
    fn stop(&mut self) {}
}
