//! Conduit power effect mechanics.

/// Gives Water Breathing + Night Vision + Haste.
pub const EFFECT_DURATION_SECS: u32 = 13;
/// Range scales with activation frame (16 * 6 + center = 42 prismarine blocks max).
pub const MAX_FRAME_SIZE: u32 = 42;
/// Base range per frame power (3×n+16).
pub fn range_for_power(power: u32) -> u32 {
    16 + (power / 7) * 16
}

/// Conduit damages hostile mobs within 8 blocks.
pub const HOSTILE_DAMAGE_RANGE: f64 = 8.0;
/// Hostile damage (4).
pub const HOSTILE_DAMAGE: f32 = 4.0;

/// Only active while touching water.
pub fn needs_water_contact() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_range_16() {
        assert_eq!(range_for_power(0), 16);
    }

    #[test]
    fn higher_power_more_range() {
        assert!(range_for_power(42) > range_for_power(0));
    }
}
