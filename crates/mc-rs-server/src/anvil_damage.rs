//! Anvil — falling damage, degradation, rename/repair/enchant combine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnvilState {
    Normal,
    Chipped,
    Damaged,
    Destroyed,
}

impl AnvilState {
    pub fn block_id(&self) -> u16 {
        match self {
            Self::Normal => 145,
            Self::Chipped => 146,
            Self::Damaged => 147,
            Self::Destroyed => 0,
        }
    }

    pub fn next_degradation(&self) -> Self {
        match self {
            Self::Normal => Self::Chipped,
            Self::Chipped => Self::Damaged,
            Self::Damaged => Self::Destroyed,
            Self::Destroyed => Self::Destroyed,
        }
    }
}

/// Degradation chance per use (12% vanilla).
pub const DEGRADATION_CHANCE: f32 = 0.12;
/// Max anvil XP cost (39 prior work before "too expensive").
pub const MAX_XP_COST: u32 = 39;
/// Rename cost (1 XP).
pub const RENAME_COST: u32 = 1;

/// Computed XP cost for combine.
pub fn combine_cost(prior_work_a: u32, prior_work_b: u32, rename: bool) -> u32 {
    let base = prior_work_a * 2 + prior_work_b * 2;
    base + if rename { RENAME_COST } else { 0 }
}

/// Falling anvil damage = 2 * fall_distance (capped 40).
pub fn falling_damage(fall_distance: f32) -> f32 {
    (fall_distance * 2.0).clamp(0.0, 40.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_degrades_to_chipped() {
        assert_eq!(AnvilState::Normal.next_degradation(), AnvilState::Chipped);
    }

    #[test]
    fn fall_damage_capped() {
        assert_eq!(falling_damage(100.0), 40.0);
    }
}
