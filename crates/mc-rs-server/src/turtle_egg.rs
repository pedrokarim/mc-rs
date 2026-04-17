//! Turtle egg — laid by turtle, hatches into baby turtles.

#[derive(Debug, Clone)]
pub struct TurtleEgg {
    pub egg_count: u8,   // 1-4 eggs per block
    pub hatch_stage: u8, // 0-2 (crack progression)
    pub nights_survived: u32,
}

/// Max crack stage.
pub const MAX_STAGE: u8 = 2;
/// Nights to hatch (approximate).
pub const MIN_NIGHTS: u32 = 3;

impl TurtleEgg {
    pub fn new(count: u8) -> Self {
        Self {
            egg_count: count.clamp(1, 4),
            hatch_stage: 0,
            nights_survived: 0,
        }
    }

    pub fn random_crack_at_night(&mut self) -> bool {
        self.nights_survived += 1;
        if self.hatch_stage < MAX_STAGE {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen::<f32>() < 0.25 {
                self.hatch_stage += 1;
            }
        }
        self.hatch_stage >= MAX_STAGE && self.nights_survived >= MIN_NIGHTS
    }

    pub fn entity_step_breaks(&mut self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f32>() < 1.0 / 3.0
    }

    pub fn zombie_step_always_breaks() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egg_count_bounded() {
        let e = TurtleEgg::new(10);
        assert!(e.egg_count <= 4);
    }
}
