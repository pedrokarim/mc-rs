//! Phantom — spawne la nuit quand joueur pas dormi >3 jours.

#[derive(Debug, Clone)]
pub struct Phantom {
    pub target_entity: Option<u64>,
    pub circle_ticks: u32,
    pub dive_ticks: u32,
    pub cooldown_after_hit: u32,
    pub swoop_active: bool,
}

/// Spawn cooldown required (3 jours = 72000 ticks).
pub const INSOMNIA_TICKS: u32 = 72_000;
/// Min altitude above target (7 blocs).
pub const MIN_ALTITUDE_ABOVE: f64 = 7.0;
/// Dive speed.
pub const DIVE_SPEED: f64 = 0.3;
/// Dive duration before next circle.
pub const DIVE_DURATION: u32 = 40;
/// Damage (2 hard, 1 normal, 0.5 easy).
pub fn damage_by_difficulty(d: u8) -> f32 {
    match d {
        0 | 1 => 2.0,
        2 => 2.5,
        _ => 3.0,
    }
}

impl Phantom {
    pub fn new() -> Self {
        Self {
            target_entity: None,
            circle_ticks: 0,
            dive_ticks: 0,
            cooldown_after_hit: 0,
            swoop_active: false,
        }
    }

    pub fn tick(&mut self) {
        if self.cooldown_after_hit > 0 {
            self.cooldown_after_hit -= 1;
        }
        if self.swoop_active {
            self.dive_ticks += 1;
            if self.dive_ticks >= DIVE_DURATION {
                self.swoop_active = false;
                self.dive_ticks = 0;
                self.cooldown_after_hit = 200;
            }
        } else {
            self.circle_ticks += 1;
        }
    }

    pub fn start_swoop(&mut self) {
        if self.cooldown_after_hit > 0 {
            return;
        }
        self.swoop_active = true;
        self.dive_ticks = 0;
    }

    /// Burns in daylight (retreats to altitude).
    pub fn burns_in_sunlight() -> bool {
        true
    }

    /// Only spawns when player hasn't slept > 3 days.
    pub fn can_spawn_for(player_ticks_since_sleep: u32) -> bool {
        player_ticks_since_sleep >= INSOMNIA_TICKS
    }
}

impl Default for Phantom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_spawn_within_3_days() {
        assert!(!Phantom::can_spawn_for(10000));
    }

    #[test]
    fn spawn_after_insomnia() {
        assert!(Phantom::can_spawn_for(80000));
    }

    #[test]
    fn swoop_completes() {
        let mut p = Phantom::new();
        p.start_swoop();
        for _ in 0..=DIVE_DURATION {
            p.tick();
        }
        assert!(!p.swoop_active);
    }
}
