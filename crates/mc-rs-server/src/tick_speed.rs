//! Random tick speed and scheduled tick system.

/// Default random tick speed (3).
pub const DEFAULT_RANDOM_TICK_SPEED: u32 = 3;
/// Per-chunk random ticks = randomTickSpeed blocks per section per tick.

/// Scheduled ticks take effect after N game ticks.
#[derive(Debug, Clone)]
pub struct ScheduledBlockTick {
    pub position: (i32, i32, i32),
    pub block_id: u16,
    pub delay_ticks: u32,
    pub priority: i8,
}

impl ScheduledBlockTick {
    pub fn new(pos: (i32, i32, i32), block_id: u16, delay: u32) -> Self {
        Self {
            position: pos,
            block_id,
            delay_ticks: delay,
            priority: 0,
        }
    }

    pub fn is_due(&self, tick: u32) -> bool {
        tick >= self.delay_ticks
    }
}

/// Random block tick handler per block (simplified).
pub fn should_random_tick(block_id: u16) -> bool {
    matches!(
        block_id,
        2   // grass
        | 6 // sapling
        | 31 // grass
        | 59 // wheat
        | 79 // ice
        | 81 // cactus
        | 83 // sugar cane
        | 116 // farmland (hydration)
        | 92 // cake (nothing)
        | 110 // mycelium
        | 175 // tall grass
        | 212 // frosted ice
        | 295 // wheat seeds
        | 352 // dead bush
        | 393 // potato crop
        | 391 // carrot crop
        | 402 // beetroot
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_ticks() {
        assert!(should_random_tick(2));
    }

    #[test]
    fn stone_not_ticked() {
        assert!(!should_random_tick(1));
    }
}
