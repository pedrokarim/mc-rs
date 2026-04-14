//! Husk — zombie du desert immune au soleil.

#[derive(Debug, Clone)]
pub struct Husk {
    pub attack_cooldown: u32,
}

/// Husk hit applies hunger effect (7s vanilla).
pub const HUNGER_DURATION_HARD: u32 = 140;
pub const HUNGER_DURATION_NORMAL: u32 = 140;
pub const HUNGER_DURATION_EASY: u32 = 70;

/// Converts to zombie if drowning for 30s.
pub const DROWNING_CONVERSION_TICKS: u32 = 600;

impl Husk {
    pub fn new() -> Self {
        Self { attack_cooldown: 0 }
    }

    pub fn hunger_duration(difficulty: u8) -> u32 {
        match difficulty {
            0 | 1 => HUNGER_DURATION_EASY,
            2 => HUNGER_DURATION_NORMAL,
            _ => HUNGER_DURATION_HARD,
        }
    }

    /// Husks don't burn in sunlight.
    pub fn burns_in_sunlight() -> bool { false }

    /// Husk underwater converts to zombie.
    pub fn converts_to_zombie_in_water() -> bool { true }
}

impl Default for Husk {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunger_scales_difficulty() {
        assert!(Husk::hunger_duration(3) >= Husk::hunger_duration(1));
    }

    #[test]
    fn immune_sunlight() {
        assert!(!Husk::burns_in_sunlight());
    }
}
