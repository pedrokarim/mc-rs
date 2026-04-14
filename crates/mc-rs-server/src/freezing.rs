//! Freezing / powder snow damage.

/// Frozen ticks before damage starts (140 ticks = 7s).
pub const FROZEN_THRESHOLD: u32 = 140;
/// Freezing damage per 2s once fully frozen.
pub const FREEZE_DAMAGE: f32 = 1.0;
pub const FREEZE_DAMAGE_INTERVAL: u32 = 40;

/// Leather armor protects from freezing.
pub fn leather_armor_protection(pieces_worn: u8) -> bool {
    pieces_worn == 4
}

/// Freeze tick decrements when not in powder snow.
pub const DECREMENT_RATE: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_leather_protects() {
        assert!(leather_armor_protection(4));
        assert!(!leather_armor_protection(3));
    }
}
