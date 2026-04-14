//! Cherry grove — 1.20 biome.

pub const BIOME_ID: u8 = 199;

pub fn unique_blocks() -> &'static [&'static str] {
    &[
        "minecraft:cherry_log",
        "minecraft:cherry_leaves",
        "minecraft:cherry_sapling",
        "minecraft:cherry_planks",
        "minecraft:pink_petals",
    ]
}

pub fn native_mobs() -> &'static [&'static str] {
    &["minecraft:pig", "minecraft:sheep", "minecraft:rabbit", "minecraft:bee"]
}

/// Pink petals grow on grass.
pub const PETAL_MAX_COUNT: u8 = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn cherry_tree_exists() {
        assert!(super::unique_blocks().contains(&"minecraft:cherry_log"));
    }
}
