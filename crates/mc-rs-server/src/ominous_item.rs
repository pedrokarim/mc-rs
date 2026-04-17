//! Ominous bottle / ominous trial key.

/// Ominous bottle gives bad omen effect.
pub const OMINOUS_BOTTLE_AMPLIFIER_MAX: u8 = 5;

/// Ominous trial key opens ominous vault (more loot).
pub fn ominous_key_loot_multiplier() -> f32 {
    3.0
}

/// Bad omen level from ominous bottle consumption.
pub fn bad_omen_from_bottle(amplifier: u8) -> u8 {
    (amplifier + 1).min(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_at_5() {
        assert_eq!(bad_omen_from_bottle(10), 5);
    }
}
