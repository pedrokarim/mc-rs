//! Events entité — port sélectif de `.reference/PocketMine-MP/src/event/entity/*`.

use super::Event;
use crate::cancellable_event;

/// Port de `EntityDamageEvent.php`. Cancellable + éditable (damage, baseDamage, modifiers).
pub struct EntityDamageEvent {
    pub target_entity_id: u64,
    pub cause: DamageCause,
    pub base_damage: f32,
    pub final_damage: f32,
    pub knockback: f32,
    /// Si damage via attaque d'une autre entité, son runtime_id.
    pub attacker_entity_id: Option<u64>,
    pub cancelled: bool,
}
impl Event for EntityDamageEvent {}
cancellable_event!(EntityDamageEvent);

/// Port PMMP `EntityDamageEvent::CAUSE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageCause {
    Contact,
    EntityAttack,
    Projectile,
    Suffocation,
    Fall,
    Fire,
    FireTick,
    Lava,
    Drowning,
    BlockExplosion,
    EntityExplosion,
    Void,
    Suicide,
    Magic,
    Custom,
    Starvation,
}

/// Port de `EntityDeathEvent.php`. Porte les drops et XP drop.
pub struct EntityDeathEvent {
    pub entity_id: u64,
    pub drops: Vec<mc_rs_proto::packets::player::ItemStack>,
    pub xp_drop: u32,
}
impl Event for EntityDeathEvent {}

/// Port de `EntitySpawnEvent.php`.
pub struct EntitySpawnEvent {
    pub entity_id: u64,
    pub entity_type: String,
    pub position: [f32; 3],
}
impl Event for EntitySpawnEvent {}

/// Port de `EntityDespawnEvent.php`.
pub struct EntityDespawnEvent {
    pub entity_id: u64,
}
impl Event for EntityDespawnEvent {}

/// Port de `EntityRegainHealthEvent.php`. Cancellable.
pub struct EntityRegainHealthEvent {
    pub entity_id: u64,
    pub amount: f32,
    pub reason: RegainReason,
    pub cancelled: bool,
}
impl Event for EntityRegainHealthEvent {}
cancellable_event!(EntityRegainHealthEvent);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegainReason {
    Eating,
    Magic,
    Saturation,
    Custom,
}

/// Port de `EntityMotionEvent.php`. Cancellable.
pub struct EntityMotionEvent {
    pub entity_id: u64,
    pub motion: [f32; 3],
    pub cancelled: bool,
}
impl Event for EntityMotionEvent {}
cancellable_event!(EntityMotionEvent);
