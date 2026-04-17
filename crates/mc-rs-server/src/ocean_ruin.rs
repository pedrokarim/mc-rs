//! Ocean ruin / underwater ruin structure.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuinTemperature {
    Warm, // Yellow sandstone
    Cold, // Gray stone
}

pub fn chest_loot(temp: RuinTemperature) -> &'static [(&'static str, u32, u32, u32)] {
    match temp {
        RuinTemperature::Warm => &[
            ("minecraft:leather_helmet", 1, 1, 3),
            ("minecraft:leather_chestplate", 1, 1, 3),
            ("minecraft:coal", 1, 4, 10),
            ("minecraft:stone_axe", 1, 1, 2),
            ("minecraft:rotten_flesh", 1, 1, 5),
            ("minecraft:emerald", 1, 1, 2),
            ("minecraft:wheat", 2, 3, 10),
            ("minecraft:gold_nugget", 1, 3, 5),
            ("minecraft:fishing_rod", 1, 1, 1),
            ("minecraft:golden_apple", 1, 1, 1),
            ("minecraft:buried_treasure_map", 1, 1, 5),
        ],
        RuinTemperature::Cold => &[
            ("minecraft:iron_axe", 1, 1, 1),
            ("minecraft:coal", 1, 4, 10),
            ("minecraft:rotten_flesh", 1, 1, 5),
            ("minecraft:emerald", 1, 1, 2),
            ("minecraft:iron_ingot", 1, 2, 5),
            ("minecraft:buried_treasure_map", 1, 1, 5),
        ],
    }
}

/// Drowned often spawn inside.
pub const SPAWNS_DROWNED: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warm_has_golden_apple() {
        let loot = chest_loot(RuinTemperature::Warm);
        assert!(loot
            .iter()
            .any(|(i, _, _, _)| *i == "minecraft:golden_apple"));
    }
}
