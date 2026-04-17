//! End city structure.

/// Loot tables.
pub fn city_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:diamond", 2, 7, 5),
        ("minecraft:iron_ingot", 4, 8, 10),
        ("minecraft:gold_ingot", 2, 7, 15),
        ("minecraft:emerald", 2, 6, 5),
        ("minecraft:beetroot_seeds", 1, 10, 5),
        ("minecraft:saddle", 1, 1, 3),
        ("minecraft:horse_armor_iron", 1, 1, 1),
        ("minecraft:horse_armor_gold", 1, 1, 1),
        ("minecraft:horse_armor_diamond", 1, 1, 1),
        ("minecraft:enchanted_book", 1, 1, 1),
        ("minecraft:iron_pickaxe", 1, 1, 5),
        ("minecraft:iron_sword", 1, 1, 5),
        ("minecraft:iron_chestplate", 1, 1, 5),
        ("minecraft:iron_leggings", 1, 1, 5),
        ("minecraft:iron_boots", 1, 1, 5),
        ("minecraft:iron_helmet", 1, 1, 5),
        ("minecraft:iron_shovel", 1, 1, 5),
        ("minecraft:diamond_pickaxe", 1, 1, 3),
        ("minecraft:diamond_sword", 1, 1, 3),
        ("minecraft:diamond_chestplate", 1, 1, 3),
    ]
}

/// Shulker spawn chance on ship.
pub const SHIP_SHULKER_CHANCE: f32 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_diamond_loot() {
        assert!(city_loot()
            .iter()
            .any(|(i, _, _, _)| *i == "minecraft:diamond"));
    }
}
