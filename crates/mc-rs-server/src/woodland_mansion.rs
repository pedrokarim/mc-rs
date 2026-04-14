//! Woodland mansion structure.

pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:book", 1, 1, 1),
        ("minecraft:bread", 1, 3, 20),
        ("minecraft:diamond_hoe", 1, 1, 1),
        ("minecraft:iron_pickaxe", 1, 1, 5),
        ("minecraft:apple", 1, 3, 15),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 1, 3, 5),
        ("minecraft:redstone", 4, 9, 5),
        ("minecraft:golden_apple", 1, 1, 1),
        ("minecraft:diamond", 1, 3, 1),
        ("minecraft:saddle", 1, 1, 3),
        ("minecraft:enchanted_book", 1, 1, 10),
    ]
}

/// Native mobs.
pub fn native_mobs() -> &'static [&'static str] {
    &[
        "minecraft:evoker",
        "minecraft:vindicator",
        "minecraft:vex", // summoned by evoker
    ]
}

/// Mansion generation only in dark forest / dark forest hills.
pub fn valid_biomes() -> &'static [u8] {
    &[29] // dark forest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_evoker() {
        assert!(native_mobs().contains(&"minecraft:evoker"));
    }
}
