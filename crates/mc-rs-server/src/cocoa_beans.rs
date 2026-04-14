//! Cocoa — growth on jungle log, 3 stages.

/// Max stage.
pub const MAX_STAGE: u8 = 2;
/// Growth chance per random tick.
pub const GROWTH_CHANCE: f32 = 0.20;
/// Bone meal advances 1 stage.

/// Drops per stage (stage 2 drops 2-3).
pub fn drops_for_stage(stage: u8, looting: u8) -> u32 {
    match stage {
        2 => 2 + looting as u32,
        _ => 1,
    }
}

/// Cocoa only grows on jungle logs.
pub fn valid_support(block_id: u16) -> bool {
    matches!(block_id, 17) // jungle log
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_2_drops_more() {
        assert!(drops_for_stage(2, 0) > drops_for_stage(1, 0));
    }
}
