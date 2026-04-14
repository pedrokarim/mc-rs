//! Hoglin — port PMMP / wiki. Mob du Nether.

#[derive(Debug, Clone)]
pub struct Hoglin {
    pub age: i32,         // -24000..0 = baby, 0+ = adult
    pub zombification_ticks: u32,
    pub love_mode_ticks: u32,
    pub is_immune_to_zombification: bool,
    pub repel_cooldown: u32,
}

/// Zombification en Overworld (300 ticks = 15s).
pub const ZOMBIFICATION_TICKS: u32 = 300;
/// Repellent: warped fungus/warped nylium (PMMP).
pub const REPEL_DETECT_RANGE: f64 = 8.0;
/// Breeding item = crimson fungus.
pub const BREEDING_ITEM_ID: u16 = 259;
/// Love mode duration (30s).
pub const LOVE_MODE_DURATION: u32 = 600;

impl Hoglin {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            zombification_ticks: 0,
            love_mode_ticks: 0,
            is_immune_to_zombification: false,
            repel_cooldown: 0,
        }
    }

    pub fn new_baby() -> Self {
        Self {
            age: -24000,
            zombification_ticks: 0,
            love_mode_ticks: 0,
            is_immune_to_zombification: false,
            repel_cooldown: 0,
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    pub fn tick(&mut self, in_overworld: bool, near_warped_block: bool) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.love_mode_ticks > 0 {
            self.love_mode_ticks -= 1;
        }
        if self.repel_cooldown > 0 {
            self.repel_cooldown -= 1;
        }
        if near_warped_block {
            self.repel_cooldown = 20;
        }
        if in_overworld && !self.is_immune_to_zombification {
            self.zombification_ticks += 1;
        } else {
            self.zombification_ticks = 0;
        }
    }

    pub fn is_zombified(&self) -> bool {
        self.zombification_ticks >= ZOMBIFICATION_TICKS
    }

    pub fn is_repelled(&self) -> bool {
        self.repel_cooldown > 0
    }

    pub fn start_love_mode(&mut self) {
        if !self.is_baby() {
            self.love_mode_ticks = LOVE_MODE_DURATION;
        }
    }

    pub fn is_in_love(&self) -> bool {
        self.love_mode_ticks > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baby_grows_over_time() {
        let mut h = Hoglin::new_baby();
        assert!(h.is_baby());
        for _ in 0..24000 {
            h.tick(false, false);
        }
        assert!(!h.is_baby());
    }

    #[test]
    fn zombifies_in_overworld() {
        let mut h = Hoglin::new_adult();
        for _ in 0..ZOMBIFICATION_TICKS {
            h.tick(true, false);
        }
        assert!(h.is_zombified());
    }

    #[test]
    fn immune_does_not_zombify() {
        let mut h = Hoglin::new_adult();
        h.is_immune_to_zombification = true;
        for _ in 0..ZOMBIFICATION_TICKS {
            h.tick(true, false);
        }
        assert!(!h.is_zombified());
    }

    #[test]
    fn repelled_by_warped() {
        let mut h = Hoglin::new_adult();
        h.tick(false, true);
        assert!(h.is_repelled());
    }
}
