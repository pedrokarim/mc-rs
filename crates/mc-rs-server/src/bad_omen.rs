//! Bad Omen effect — triggers raid when entering village.

/// Max amplifier from pillager captain kill stacking.
pub const MAX_AMPLIFIER: u8 = 5;
/// Duration per captain kill (100 min = 120000 ticks).
pub const DURATION_PER_CAPTAIN: u32 = 120_000;
/// Village radius for raid trigger.
pub const VILLAGE_DETECTION_RANGE: f64 = 32.0;

/// Multiplier for raid waves based on amplifier + difficulty.
pub fn raid_waves(amplifier: u8, difficulty: u8) -> u32 {
    let base_waves = match difficulty {
        0 | 1 => 3,
        2 => 5,
        _ => 7,
    };
    base_waves + amplifier as u32
}

/// Bad Omen gets replaced by Raid Omen when trigger pyramids.
pub const TRIAL_OMEN_DURATION: u32 = 15 * 60 * 20; // 15 min

/// Cured when milk is drunk.
pub fn cured_by_milk() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_more_waves() {
        assert!(raid_waves(1, 3) > raid_waves(1, 1));
    }

    #[test]
    fn amplifier_adds_waves() {
        assert!(raid_waves(3, 2) > raid_waves(1, 2));
    }
}
