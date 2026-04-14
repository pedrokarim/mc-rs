//! Block pickaxe tier mapping.

use crate::tool_types::ToolTier;

/// Required tier to drop this block.
pub fn required_tier(block_id: u16) -> ToolTier {
    match block_id {
        // Wood tier (stone, coal, copper)
        1 | 4 | 7 | 15 | 16 | 45 | 47 | 48 | 97 | 98 | 108 | 109 | 139 | 215 | 281 | 282 | 283 => ToolTier::Wood,
        // Stone tier (iron, lapis)
        14 | 21 | 42 | 56 | 73 | 74 | 120 | 122 | 129 | 153 => ToolTier::Stone,
        // Iron tier (diamond, gold, emerald, redstone)
        57 | 152 | 524 | 525 | 526 | 527 => ToolTier::Iron,
        // Diamond tier (obsidian, netherite)
        49 | 528 | 529 | 530 => ToolTier::Diamond,
        // Netherite (crying obsidian, respawn anchor)
        460 | 461 => ToolTier::Diamond,
        _ => ToolTier::Wood,
    }
}

/// Does breaking this block drop an item?
pub fn drops_without_tool(block_id: u16) -> bool {
    matches!(block_id,
        2 | 3  // dirt/grass
        | 5 | 17 | 162 // wood
        | 10 | 11 // lava (no drop)
        | 12 | 13 // sand/gravel
        | 20 | 95 // glass
        | 18 | 161 // leaves
        | 30 | 82 // cobweb/clay
        | 35 | 41 // wool/gold block (requires pickaxe)
        | 60 // farmland
        | 79 | 212 // ice
        | 81 // cactus
        | 86 | 103 // pumpkin/melon
        | 87 // netherrack (pickaxe)
        | 110 // mycelium
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_any_tier() {
        assert_eq!(required_tier(1), ToolTier::Wood);
    }

    #[test]
    fn obsidian_diamond() {
        assert_eq!(required_tier(49), ToolTier::Diamond);
    }
}
