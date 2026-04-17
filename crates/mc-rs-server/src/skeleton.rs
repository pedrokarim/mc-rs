//! Skeleton + WitherSkeleton + Stray + Bogged — archer AI.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonVariant {
    Normal,
    Wither, // fire melee sword
    Stray,  // shoots slowness arrows
    Bogged, // shoots poison arrows
}

#[derive(Debug, Clone)]
pub struct Skeleton {
    pub variant: SkeletonVariant,
    pub attack_cooldown: u32,
    pub charge_ticks: u32,
    pub target_entity: Option<u64>,
}

/// Bow charge time (20 ticks = 1s).
pub const BOW_CHARGE: u32 = 20;
/// Attack cooldown between shots.
pub const SHOT_COOLDOWN: u32 = 30;
/// Damage varie par difficulté + crit.
pub const ARROW_DAMAGE: f32 = 2.0;

impl Skeleton {
    pub fn new(variant: SkeletonVariant) -> Self {
        Self {
            variant,
            attack_cooldown: 0,
            charge_ticks: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        } else if self.target_entity.is_some() {
            self.charge_ticks += 1;
        }
    }

    pub fn set_target(&mut self, target: u64) {
        self.target_entity = Some(target);
        self.charge_ticks = 0;
    }

    pub fn try_fire(&mut self) -> Option<ArrowEffect> {
        if self.attack_cooldown > 0 || self.charge_ticks < BOW_CHARGE {
            return None;
        }
        self.attack_cooldown = SHOT_COOLDOWN;
        self.charge_ticks = 0;
        Some(match self.variant {
            SkeletonVariant::Stray => ArrowEffect::Slowness,
            SkeletonVariant::Bogged => ArrowEffect::Poison,
            _ => ArrowEffect::Normal,
        })
    }

    /// Burns in sunlight (all but Wither Skeleton).
    pub fn burns_in_sunlight(&self) -> bool {
        self.variant != SkeletonVariant::Wither
    }

    /// Helmet protects from sunlight.
    pub fn protected_by_helmet() -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowEffect {
    Normal,
    Slowness,
    Poison,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stray_fires_slow_arrows() {
        let mut s = Skeleton::new(SkeletonVariant::Stray);
        s.set_target(1);
        for _ in 0..BOW_CHARGE {
            s.tick();
        }
        assert_eq!(s.try_fire(), Some(ArrowEffect::Slowness));
    }

    #[test]
    fn wither_skeleton_immune_to_sun() {
        let s = Skeleton::new(SkeletonVariant::Wither);
        assert!(!s.burns_in_sunlight());
    }
}
