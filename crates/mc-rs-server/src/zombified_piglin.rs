//! ZombifiedPiglin (formerly ZombiePigman) — neutral unless attacked, group anger.

#[derive(Debug, Clone)]
pub struct ZombifiedPiglin {
    pub anger_ticks: u32,
    pub anger_target: Option<u64>,
}

/// Anger duration when provoked (20-40 seconds randomized vanilla).
pub const ANGER_DURATION_MIN: u32 = 400;
pub const ANGER_DURATION_MAX: u32 = 800;
/// Group anger range.
pub const GROUP_ANGER_RANGE: f64 = 32.0;

impl ZombifiedPiglin {
    pub fn new() -> Self {
        Self {
            anger_ticks: 0,
            anger_target: None,
        }
    }

    pub fn anger_at(&mut self, target: u64) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.anger_ticks = rng.gen_range(ANGER_DURATION_MIN..=ANGER_DURATION_MAX);
        self.anger_target = Some(target);
    }

    pub fn tick(&mut self) {
        if self.anger_ticks > 0 {
            self.anger_ticks -= 1;
            if self.anger_ticks == 0 {
                self.anger_target = None;
            }
        }
    }

    pub fn is_angry(&self) -> bool {
        self.anger_ticks > 0
    }
}

impl Default for ZombifiedPiglin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_neutral() {
        let z = ZombifiedPiglin::new();
        assert!(!z.is_angry());
    }

    #[test]
    fn anger_fades_to_neutral() {
        let mut z = ZombifiedPiglin::new();
        z.anger_at(42);
        for _ in 0..ANGER_DURATION_MAX {
            z.tick();
        }
        assert!(!z.is_angry());
    }
}
