//! Boss fights — Wither, Ender Dragon, Warden.

use crate::mob_ai::MobKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossKind {
    Wither,
    EnderDragon,
    Warden,
    ElderGuardian,
}

impl BossKind {
    pub fn to_mob(&self) -> MobKind {
        match self {
            Self::Wither => MobKind::Wither,
            Self::EnderDragon => MobKind::EnderDragon,
            Self::Warden => MobKind::Enderman, // Warden pas encore dans mob_ai
            Self::ElderGuardian => MobKind::Witch, // Placeholder
        }
    }

    pub fn xp_drop(&self) -> u32 {
        match self {
            Self::Wither => 50,
            Self::EnderDragon => 12000,
            Self::Warden => 5,
            Self::ElderGuardian => 10,
        }
    }

    pub fn music_disc_drop(&self) -> Option<&'static str> {
        None // Vanilla bosses n'droppent pas de disc de base
    }

    pub fn max_health(&self) -> f32 {
        match self {
            Self::Wither => 300.0,
            Self::EnderDragon => 200.0,
            Self::Warden => 500.0,
            Self::ElderGuardian => 80.0,
        }
    }
}

/// Phase d'un boss (Wither a des phases de blindage, Ender Dragon a des
/// phases de respawn / charge / attack / perch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossPhase {
    Armored,  // Wither initial
    Normal,   // Wither après armor
    Perching, // Dragon sitting on portal
    Charging, // Dragon charge attack
    HoveringStrafing,
    LandingApproach,
    Landing,
    TakeOff,
    Dying,
    Regen, // Warden post-hit
}

#[derive(Debug, Clone)]
pub struct BossState {
    pub kind: BossKind,
    pub entity_runtime_id: u64,
    pub health: f32,
    pub phase: BossPhase,
    pub bossbar_id: i64,
}

impl BossState {
    pub fn new(kind: BossKind, entity_runtime_id: u64) -> Self {
        let health = kind.max_health();
        Self {
            kind,
            entity_runtime_id,
            health,
            phase: BossPhase::Normal,
            bossbar_id: entity_runtime_id as i64,
        }
    }

    pub fn health_percent(&self) -> f32 {
        self.health / self.kind.max_health()
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wither_300_hp() {
        assert_eq!(BossKind::Wither.max_health(), 300.0);
    }

    #[test]
    fn dragon_drops_12000_xp() {
        assert_eq!(BossKind::EnderDragon.xp_drop(), 12000);
    }
}
