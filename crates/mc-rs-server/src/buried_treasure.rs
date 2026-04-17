//! Buried treasure — marked on explorer maps.

/// Found at depth ~-40 to sea level, beach biomes.
pub const MIN_DEPTH: i32 = -40;
pub const MAX_DEPTH: i32 = 55;

/// Chest has heart of the sea.
pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:heart_of_the_sea", 1, 1, 100),
        ("minecraft:tnt", 1, 2, 20),
        ("minecraft:iron_ingot", 1, 4, 20),
        ("minecraft:gold_ingot", 1, 4, 15),
        ("minecraft:emerald", 4, 8, 10),
        ("minecraft:diamond", 1, 2, 5),
        ("minecraft:prismarine_crystals", 1, 5, 15),
        ("minecraft:cooked_cod", 2, 4, 25),
        ("minecraft:cooked_salmon", 2, 4, 25),
        ("minecraft:leather_chestplate", 1, 1, 5),
    ]
}

/// Heart of the Sea crafts conduit (with nautilus shells).
pub fn conduit_recipe() -> (&'static [&'static str], &'static str) {
    (
        &[
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
            "minecraft:heart_of_the_sea",
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
            "minecraft:nautilus_shell",
        ],
        "minecraft:conduit",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_heart_of_sea() {
        assert!(chest_loot()
            .iter()
            .any(|(i, _, _, _)| *i == "minecraft:heart_of_the_sea"));
    }
}
