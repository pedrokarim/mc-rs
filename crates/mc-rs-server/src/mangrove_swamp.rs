//! Mangrove swamp biome.

pub const BIOME_ID: u8 = 198;

/// Unique blocks.
pub fn unique_blocks() -> &'static [&'static str] {
    &[
        "minecraft:mangrove_log",
        "minecraft:mangrove_leaves",
        "minecraft:mangrove_propagule",
        "minecraft:mangrove_roots",
        "minecraft:muddy_mangrove_roots",
        "minecraft:mud",
        "minecraft:mud_bricks",
        "minecraft:packed_mud",
        "minecraft:mangrove_planks",
    ]
}

/// Mangrove tree grows from hanging propagule.
pub fn tree_min_height() -> u32 {
    5
}
pub fn tree_max_height() -> u32 {
    14
}

/// Mobs: frog, slime, mudfish (not real).
pub fn native_mobs() -> &'static [&'static str] {
    &["minecraft:frog", "minecraft:slime"]
}

#[cfg(test)]
mod tests {
    #[test]
    fn unique_blocks_has_mangrove() {
        assert!(super::unique_blocks()
            .iter()
            .any(|s| s.contains("mangrove")));
    }
}
