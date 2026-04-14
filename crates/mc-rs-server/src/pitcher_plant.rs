//! Pitcher plant — 2-block tall plant from pitcher pod (Sniffer drop).

/// Growth stages (0-4).
pub const MAX_STAGE: u8 = 4;
/// Growth chance.
pub const GROWTH_CHANCE: f32 = 0.10;
/// Pitcher plant drops 1-2 pitcher pod at fully grown.
pub const MATURE_DROPS: (u32, u32) = (1, 2);

#[cfg(test)]
mod tests {
    #[test]
    fn max_stage_4() {
        assert_eq!(super::MAX_STAGE, 4);
    }
}
