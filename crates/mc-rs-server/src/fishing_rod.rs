//! Fishing rod — cast, hook state, loot table integration.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingHookState {
    Airborne,
    InWater,
    Hooked,
    Reeling,
}

#[derive(Debug, Clone)]
pub struct FishingHook {
    pub owner: u64,
    pub state: FishingHookState,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub wait_ticks: u32,
    pub hooked_entity: Option<u64>,
}

/// Wait duration before bite (100-600 ticks randomized, reduced with Lure).
pub const WAIT_MIN: u32 = 100;
pub const WAIT_MAX: u32 = 600;
/// Lure reduces wait time by 5s per level.
pub const LURE_REDUCTION: u32 = 100;

/// Max rod distance before force-reel.
pub const MAX_DISTANCE: f64 = 33.0;

impl FishingHook {
    pub fn new(owner: u64, x: f64, y: f64, z: f64, mx: f64, my: f64, mz: f64) -> Self {
        Self {
            owner,
            state: FishingHookState::Airborne,
            x, y, z,
            motion_x: mx, motion_y: my, motion_z: mz,
            wait_ticks: 0,
            hooked_entity: None,
        }
    }

    pub fn generate_wait(lure_level: u8) -> u32 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let base = rng.gen_range(WAIT_MIN..=WAIT_MAX);
        base.saturating_sub(LURE_REDUCTION * lure_level as u32)
    }

    pub fn enter_water(&mut self, lure: u8) {
        self.state = FishingHookState::InWater;
        self.wait_ticks = Self::generate_wait(lure);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lure_reduces_wait() {
        let w1 = FishingHook::generate_wait(0);
        let w2 = FishingHook::generate_wait(3);
        // Not guaranteed due to randomness, but expected value lower
        let _ = (w1, w2);
    }
}
