//! Mining speed modifiers (haste, efficiency, fatigue).

/// Haste effect: +20% per amplifier.
pub fn haste_modifier(amplifier: u8) -> f32 {
    1.0 + 0.2 * (amplifier + 1) as f32
}

/// Mining fatigue: slower per amplifier.
pub fn mining_fatigue_modifier(amplifier: u8) -> f32 {
    0.3_f32.powi(amplifier.min(4) as i32 + 1) * 0.7
}

/// Conduit Power also gives haste bonus.
pub const CONDUIT_HASTE_LEVEL: u8 = 1;

/// Water without Aqua Affinity slows mining by 5x.
pub const WATER_SLOWDOWN: f32 = 1.0 / 5.0;
/// Airborne (not ground) slows by 5x.
pub const AIR_SLOWDOWN: f32 = 1.0 / 5.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haste_faster() {
        assert!(haste_modifier(1) > 1.0);
    }

    #[test]
    fn fatigue_slower() {
        assert!(mining_fatigue_modifier(0) < 1.0);
    }
}
