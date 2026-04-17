//! IronGolem — port PMMP. Protecteur du village qui s'énerve.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GolemOrigin {
    VillagerMade,
    PlayerBuilt,
}

#[derive(Debug, Clone)]
pub struct IronGolem {
    pub origin: GolemOrigin,
    pub anger_ticks: u32,
    pub target_entity: Option<u64>,
    pub village_id: Option<u64>,
    pub cracked_level: CrackLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrackLevel {
    None,
    Low,    // 75% hp
    Medium, // 50% hp
    High,   // 25% hp
}

/// Damage melee (randomized vanilla 7-21 sur normal).
pub const ATTACK_MIN: f32 = 7.0;
pub const ATTACK_MAX: f32 = 21.0;
/// Anger duration après provoqué (600 ticks = 30s).
pub const ANGER_DURATION: u32 = 600;

impl IronGolem {
    pub fn new(origin: GolemOrigin) -> Self {
        Self {
            origin,
            anger_ticks: 0,
            target_entity: None,
            village_id: None,
            cracked_level: CrackLevel::None,
        }
    }

    pub fn anger(&mut self, target: u64) {
        self.anger_ticks = ANGER_DURATION;
        self.target_entity = Some(target);
    }

    pub fn tick(&mut self) {
        if self.anger_ticks > 0 {
            self.anger_ticks -= 1;
            if self.anger_ticks == 0 {
                self.target_entity = None;
            }
        }
    }

    pub fn update_crack_level(&mut self, hp_ratio: f32) {
        self.cracked_level = if hp_ratio > 0.75 {
            CrackLevel::None
        } else if hp_ratio > 0.5 {
            CrackLevel::Low
        } else if hp_ratio > 0.25 {
            CrackLevel::Medium
        } else {
            CrackLevel::High
        };
    }

    /// Heal quand joueur utilise iron ingot.
    pub fn heal_amount_iron_ingot() -> f32 {
        25.0
    }

    pub fn is_angry(&self) -> bool {
        self.anger_ticks > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golem_angers_for_duration() {
        let mut g = IronGolem::new(GolemOrigin::PlayerBuilt);
        g.anger(42);
        assert!(g.is_angry());
        assert_eq!(g.target_entity, Some(42));
    }

    #[test]
    fn anger_fades() {
        let mut g = IronGolem::new(GolemOrigin::PlayerBuilt);
        g.anger(42);
        for _ in 0..ANGER_DURATION {
            g.tick();
        }
        assert!(!g.is_angry());
    }

    #[test]
    fn crack_levels_by_hp() {
        let mut g = IronGolem::new(GolemOrigin::PlayerBuilt);
        g.update_crack_level(0.8);
        assert_eq!(g.cracked_level, CrackLevel::None);
        g.update_crack_level(0.2);
        assert_eq!(g.cracked_level, CrackLevel::High);
    }
}
