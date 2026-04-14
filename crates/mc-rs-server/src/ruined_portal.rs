//! Ruined portal — broken nether portal frame.

/// Chest loot.
pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:iron_nugget", 9, 18, 40),
        ("minecraft:gold_nugget", 9, 18, 40),
        ("minecraft:iron_ingot", 1, 2, 40),
        ("minecraft:gold_ingot", 1, 2, 15),
        ("minecraft:flint_and_steel", 1, 1, 40),
        ("minecraft:fire_charge", 1, 1, 40),
        ("minecraft:obsidian", 1, 2, 40),
        ("minecraft:golden_apple", 1, 1, 15),
        ("minecraft:glistering_melon_slice", 4, 12, 15),
        ("minecraft:golden_carrot", 4, 12, 15),
        ("minecraft:gold_block", 1, 2, 1),
        ("minecraft:enchanted_golden_apple", 1, 1, 1),
    ]
}

/// Spawns in Overworld + Nether.
pub fn spawn_dimensions() -> &'static [&'static str] {
    &["overworld", "nether"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_golden_apple() {
        assert!(chest_loot().iter().any(|(i, _, _, _)| *i == "minecraft:golden_apple"));
    }
}
