//! Zombie Villager — conversion back to villager.

use crate::trading::VillagerProfession;

#[derive(Debug, Clone)]
pub struct ZombieVillager {
    pub profession: VillagerProfession,
    pub level: u8,
    pub curing: bool,
    pub cure_ticks: u32,
}

/// Cure potion = Weakness splash + Golden apple.
pub const WEAKNESS_DURATION: u32 = 120 * 20; // 2 min
/// Cure duration (3-6 min randomized).
pub const CURE_MIN: u32 = 3 * 60 * 20;
pub const CURE_MAX: u32 = 6 * 60 * 20;

impl ZombieVillager {
    pub fn new(profession: VillagerProfession, level: u8) -> Self {
        Self {
            profession,
            level,
            curing: false,
            cure_ticks: 0,
        }
    }

    pub fn start_cure(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.curing = true;
        self.cure_ticks = rng.gen_range(CURE_MIN..=CURE_MAX);
    }

    pub fn tick(&mut self) -> bool {
        if self.curing && self.cure_ticks > 0 {
            self.cure_ticks -= 1;
        }
        self.curing && self.cure_ticks == 0
    }

    /// Cure reduction if iron bars near + bed near (~4%).
    pub fn cure_speedup_multiplier(bed_near: bool, iron_near: u32) -> f32 {
        let mut factor = 1.0;
        if bed_near {
            factor *= 0.95;
        }
        factor *= 0.95_f32.powi(iron_near.min(14) as i32);
        factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bed_speeds_cure() {
        assert!(ZombieVillager::cure_speedup_multiplier(true, 0) < 1.0);
    }
}
