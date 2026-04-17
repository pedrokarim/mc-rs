//! Frog spawn — laid by frog in water, hatches tadpoles.

#[derive(Debug, Clone)]
pub struct FrogSpawn {
    pub hatch_ticks: u32,
}

/// Hatch time (3600-7200 ticks vanilla).
pub const HATCH_MIN: u32 = 3600;
pub const HATCH_MAX: u32 = 7200;

impl FrogSpawn {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            hatch_ticks: rng.gen_range(HATCH_MIN..=HATCH_MAX),
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.hatch_ticks > 0 {
            self.hatch_ticks -= 1;
        }
        self.hatch_ticks == 0
    }
}

impl Default for FrogSpawn {
    fn default() -> Self {
        Self::new()
    }
}

/// Tadpole mob (grows into frog).
#[derive(Debug, Clone)]
pub struct Tadpole {
    pub age: i32,
    pub warmth_temperature: f32, // biome temp — affects variant
}

impl Tadpole {
    pub fn new(warmth: f32) -> Self {
        Self {
            age: -24000,
            warmth_temperature: warmth,
        }
    }

    pub fn is_ready_to_become_frog(&self) -> bool {
        self.age >= 0
    }

    pub fn tick(&mut self) {
        self.age += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatch_eventually() {
        let mut s = FrogSpawn::new();
        s.hatch_ticks = 1;
        assert!(s.tick());
    }
}
