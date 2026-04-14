//! Nether fortress structure.

/// Wither skeleton spawn range (32 blocks in fortress).
pub const WITHER_SKELETON_SPAWN_RANGE: f64 = 32.0;
/// Blaze spawner common in fortress.
pub const BLAZE_SPAWNER: bool = true;

/// Fortress loot chest.
pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:obsidian", 2, 4, 10),
        ("minecraft:nether_wart", 3, 7, 5),
        ("minecraft:flint_and_steel", 1, 1, 10),
        ("minecraft:nether_brick", 4, 9, 10),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 1, 3, 15),
        ("minecraft:gold_horse_armor", 1, 1, 8),
        ("minecraft:diamond_horse_armor", 1, 1, 5),
        ("minecraft:iron_horse_armor", 1, 1, 8),
        ("minecraft:saddle", 1, 1, 10),
        ("minecraft:diamond", 1, 3, 5),
    ]
}

/// Common mobs.
pub fn native_mobs() -> &'static [&'static str] {
    &[
        "minecraft:wither_skeleton",
        "minecraft:blaze",
        "minecraft:zombified_piglin",
        "minecraft:magma_cube",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_nether_mobs() {
        assert!(native_mobs().contains(&"minecraft:blaze"));
    }
}
