//! Shipwreck — ocean structure with loot.

/// Three chest types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipwreckChest {
    Map,
    Supply,
    Treasure,
}

pub fn map_chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:buried_treasure_map", 1, 1, 8),
        ("minecraft:compass", 1, 1, 1),
        ("minecraft:book", 1, 5, 5),
        ("minecraft:paper", 1, 12, 20),
        ("minecraft:clock", 1, 1, 1),
        ("minecraft:empty_map", 1, 1, 1),
        ("minecraft:feather", 1, 5, 10),
    ]
}

pub fn supply_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:paper", 1, 12, 8),
        ("minecraft:suspicious_stew", 1, 1, 1),
        ("minecraft:coal", 2, 8, 2),
        ("minecraft:rotten_flesh", 5, 24, 5),
        ("minecraft:potato", 2, 6, 7),
        ("minecraft:poisonous_potato", 2, 6, 7),
        ("minecraft:carrot", 4, 8, 10),
        ("minecraft:wheat", 8, 21, 20),
        ("minecraft:bamboo", 1, 3, 2),
        ("minecraft:gunpowder", 1, 5, 5),
        ("minecraft:tnt", 1, 2, 1),
    ]
}

pub fn treasure_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:iron_ingot", 1, 5, 90),
        ("minecraft:gold_ingot", 1, 5, 10),
        ("minecraft:emerald", 1, 5, 40),
        ("minecraft:diamond", 1, 5, 5),
        ("minecraft:experience_bottle", 1, 1, 5),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treasure_map_valid() {
        assert!(map_chest_loot().iter().any(|(i, _, _, _)| *i == "minecraft:buried_treasure_map"));
    }
}
