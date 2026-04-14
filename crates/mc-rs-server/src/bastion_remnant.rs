//! Bastion Remnant — Nether structure (piglins).

/// Bastion types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BastionType {
    HousingUnits,
    Stables,
    Treasure,
    Bridge,
}

pub fn bastion_types() -> &'static [BastionType] {
    &[
        BastionType::HousingUnits,
        BastionType::Stables,
        BastionType::Treasure,
        BastionType::Bridge,
    ]
}

/// Loot — treasure room chest.
pub fn treasure_room_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:netherite_ingot", 1, 2, 1),
        ("minecraft:netherite_scrap", 1, 2, 5),
        ("minecraft:golden_carrot", 8, 17, 15),
        ("minecraft:gold_ingot", 9, 18, 10),
        ("minecraft:ancient_debris", 1, 2, 12),
        ("minecraft:enchanted_book", 1, 1, 10),
        ("minecraft:iron_block", 2, 4, 5),
        ("minecraft:crying_obsidian", 3, 8, 10),
        ("minecraft:diamond_sword", 1, 1, 3),
        ("minecraft:diamond_chestplate", 1, 1, 3),
        ("minecraft:spectral_arrow", 2, 15, 25),
    ]
}

/// General chest loot.
pub fn general_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:gold_ingot", 2, 8, 15),
        ("minecraft:golden_apple", 1, 1, 10),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:string", 3, 10, 10),
        ("minecraft:book", 1, 1, 10),
        ("minecraft:arrow", 1, 12, 10),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treasure_has_netherite() {
        assert!(treasure_room_loot().iter().any(|(i, _, _, _)| *i == "minecraft:netherite_ingot"));
    }
}
