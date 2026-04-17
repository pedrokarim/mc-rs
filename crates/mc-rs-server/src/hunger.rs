//! Hunger system — port PMMP.

#[derive(Debug, Clone)]
pub struct HungerState {
    pub hunger: u8,      // 0-20 (visible)
    pub saturation: f32, // 0-20 (invisible)
    pub exhaustion: f32, // 0-4 (triggers saturation/hunger decrease at 4)
    pub food_tick: u32,  // tracks healing/damage
}

/// Natural regen max hunger threshold.
pub const NATURAL_REGEN_HUNGER: u8 = 18;
/// Exhaustion threshold to trigger decrease (4.0).
pub const EXHAUSTION_THRESHOLD: f32 = 4.0;
/// Regen interval while hunger = 20 (10 ticks).
pub const REGEN_INTERVAL_FAST: u32 = 10;
/// Regen interval while hunger >= 18 (80 ticks = 4s).
pub const REGEN_INTERVAL_SLOW: u32 = 80;
/// Starvation interval (80 ticks).
pub const STARVE_INTERVAL: u32 = 80;

/// Actions causing exhaustion.
pub mod actions {
    pub const JUMP_SPRINT: f32 = 0.2;
    pub const JUMP: f32 = 0.05;
    pub const SPRINT: f32 = 0.1;
    pub const WALK: f32 = 0.0;
    pub const SWIM: f32 = 0.01;
    pub const ATTACK: f32 = 0.1;
    pub const BREAK_BLOCK: f32 = 0.005;
    pub const REGENERATION: f32 = 6.0;
}

impl HungerState {
    pub fn new() -> Self {
        Self {
            hunger: 20,
            saturation: 5.0,
            exhaustion: 0.0,
            food_tick: 0,
        }
    }

    pub fn add_exhaustion(&mut self, amount: f32) {
        self.exhaustion += amount;
        while self.exhaustion >= EXHAUSTION_THRESHOLD {
            self.exhaustion -= EXHAUSTION_THRESHOLD;
            if self.saturation > 0.0 {
                self.saturation = (self.saturation - 1.0).max(0.0);
            } else if self.hunger > 0 {
                self.hunger -= 1;
            }
        }
    }

    pub fn restore(&mut self, hunger: u8, saturation: f32) {
        self.hunger = (self.hunger + hunger).min(20);
        self.saturation = (self.saturation + saturation).min(self.hunger as f32);
    }

    pub fn is_starving(&self) -> bool {
        self.hunger == 0
    }

    pub fn can_naturally_regen(&self) -> bool {
        self.hunger >= NATURAL_REGEN_HUNGER
    }

    pub fn can_sprint(&self) -> bool {
        self.hunger > 6
    }
}

impl Default for HungerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustion_consumes_saturation_first() {
        let mut h = HungerState::new();
        h.add_exhaustion(EXHAUSTION_THRESHOLD);
        assert!(h.saturation < 5.0);
    }

    #[test]
    fn cannot_sprint_hungry() {
        let mut h = HungerState::new();
        h.hunger = 3;
        assert!(!h.can_sprint());
    }

    #[test]
    fn starving_at_zero() {
        let mut h = HungerState::new();
        h.hunger = 0;
        assert!(h.is_starving());
    }
}
