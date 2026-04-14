//! Amethyst block resonance — amplifies vibrations.

/// Amethyst block extends sculk sensor range (16 blocks).
pub const RESONANCE_EXTEND: f64 = 16.0;
/// Calibrated sculk sensor needs amethyst + sensor (bedrock parity).

/// Resonance works with allay duplication when noteblock plays.
pub fn is_allay_duplication_trigger(block_id: u16) -> bool {
    matches!(block_id, 722) // amethyst block
}

/// Resonance frequency (for calibrated sensor detection).
pub fn calibrated_frequency(side_input: u8) -> u8 {
    side_input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amethyst_triggers_allay() {
        assert!(is_allay_duplication_trigger(722));
    }
}
