//! Deepslate blocks + ore variants.

/// Deepslate harder than stone.
pub const DEEPSLATE_HARDNESS: f32 = 3.0;
pub const DEEPSLATE_BLAST_RESISTANCE: f32 = 6.0;

/// Deepslate ore generation depth range.
pub const DEEPSLATE_MIN_Y: i32 = 0;
pub const DEEPSLATE_MAX_Y: i32 = 8;
/// Transition zone: stone → deepslate (0-8).

pub fn is_deepslate_block(block_id: u16) -> bool {
    matches!(
        block_id,
        649  // deepslate
        | 650 // cobbled deepslate
        | 651 // polished deepslate
        | 652 // deepslate bricks
        | 653 // deepslate tiles
        | 654 // chiseled deepslate
        | 655 // deepslate slab
        | 656 // deepslate stairs
        | 674 // deepslate coal ore
        | 675 // deepslate iron ore
        | 676 // deepslate gold ore
        | 677 // deepslate diamond ore
        | 678 // deepslate emerald ore
        | 679 // deepslate lapis ore
        | 680 // deepslate redstone ore
        | 681 // deepslate copper ore
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepslate_ore_recognized() {
        assert!(is_deepslate_block(674));
    }

    #[test]
    fn stone_not_deepslate() {
        assert!(!is_deepslate_block(1));
    }
}
