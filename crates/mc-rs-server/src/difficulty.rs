//! Difficulty — port PMMP `src/world/Difficulty.php`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Difficulty {
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl Difficulty {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::Peaceful,
            1 => Self::Easy,
            3 => Self::Hard,
            _ => Self::Normal,
        }
    }

    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    /// Multiplicateur de damage mob → joueur.
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            Self::Peaceful => 0.0,
            Self::Easy => 0.5,
            Self::Normal => 1.0,
            Self::Hard => 1.5,
        }
    }

    /// Mobs hostiles spawn ?
    pub fn hostile_spawn_enabled(&self) -> bool {
        !matches!(self, Self::Peaceful)
    }

    /// Natural regen de vie active (si hunger ≥ 18).
    pub fn natural_regen(&self) -> bool {
        true // always unless disabled by gamerule
    }

    /// Starvation damage enabled ?
    pub fn starvation_enabled(&self) -> bool {
        matches!(self, Self::Easy | Self::Normal | Self::Hard)
    }

    /// Starvation minimum HP (ne descend pas en dessous).
    pub fn starvation_min_hp(&self) -> f32 {
        match self {
            Self::Peaceful => 20.0, // no starvation
            Self::Easy => 10.0,
            Self::Normal => 1.0,
            Self::Hard => 0.0, // can die
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaceful_no_hostiles() {
        assert!(!Difficulty::Peaceful.hostile_spawn_enabled());
        assert!(Difficulty::Normal.hostile_spawn_enabled());
    }

    #[test]
    fn hard_1_5x_damage() {
        assert_eq!(Difficulty::Hard.damage_multiplier(), 1.5);
    }

    #[test]
    fn hard_can_kill_from_starvation() {
        assert_eq!(Difficulty::Hard.starvation_min_hp(), 0.0);
    }
}
