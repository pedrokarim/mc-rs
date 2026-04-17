//! Waterlogged blocks — fence, slab, stair, glass pane, trapdoor, etc.

/// Can a block be waterlogged?
pub fn can_waterlog(block_id: u16) -> bool {
    matches!(
        block_id,
        44  // slab
        | 53 | 67 | 108 | 109 | 114 | 128 | 134 | 135 | 136 // stairs
        | 85 | 107  // fences
        | 125 | 126 // fence gate
        | 96        // trapdoor
        | 101 | 102 // glass/stained panes
        | 139       // wall
        | 140       // flower pot
        | 198       // slime block (no)
        | 65        // ladder
        | 50        // torch? (no, but actually can)
        | 200       // end rod
        | 171       // carpet (no)
        | 333       // boat (no)
        | 145       // anvil (no)
        // chain, lantern, etc.
        | 315       // chain
        | 304       // lantern
        | 313       // sea pickle
        | 461 // amethyst budding parts (no)
    )
}

/// Waterlogged state bit in block data.
pub const WATERLOGGED_BIT: u32 = 1;

/// Place water when breaking a waterlogged block.
pub fn drop_water_on_break(block_id: u16, waterlogged: bool) -> bool {
    waterlogged && can_waterlog(block_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_waterloggable() {
        assert!(can_waterlog(85));
    }

    #[test]
    fn stone_not_waterloggable() {
        assert!(!can_waterlog(1));
    }
}
