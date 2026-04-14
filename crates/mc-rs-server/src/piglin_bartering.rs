//! Piglin bartering — table de loot complète.

use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct BarterEntry {
    pub item: &'static str,
    pub count: (u32, u32),
    pub weight: u32,
}

/// Vanilla barter loot table (poids + count range).
pub fn bartering_loot() -> &'static [BarterEntry] {
    &[
        BarterEntry { item: "minecraft:enchanted_book",  count: (1, 1), weight: 5 },
        BarterEntry { item: "minecraft:iron_boots",      count: (1, 1), weight: 8 },
        BarterEntry { item: "minecraft:potion",          count: (1, 1), weight: 8 },
        BarterEntry { item: "minecraft:splash_potion",   count: (1, 1), weight: 8 },
        BarterEntry { item: "minecraft:iron_nugget",     count: (10, 36), weight: 10 },
        BarterEntry { item: "minecraft:ender_pearl",     count: (2, 4), weight: 10 },
        BarterEntry { item: "minecraft:string",          count: (3, 9), weight: 20 },
        BarterEntry { item: "minecraft:quartz",          count: (5, 12), weight: 20 },
        BarterEntry { item: "minecraft:obsidian",        count: (1, 1), weight: 40 },
        BarterEntry { item: "minecraft:crying_obsidian", count: (1, 3), weight: 40 },
        BarterEntry { item: "minecraft:fire_charge",     count: (1, 1), weight: 40 },
        BarterEntry { item: "minecraft:leather",         count: (2, 4), weight: 40 },
        BarterEntry { item: "minecraft:soul_sand",       count: (2, 8), weight: 40 },
        BarterEntry { item: "minecraft:nether_brick",    count: (2, 8), weight: 40 },
        BarterEntry { item: "minecraft:spectral_arrow",  count: (6, 12), weight: 40 },
        BarterEntry { item: "minecraft:gravel",          count: (8, 16), weight: 40 },
        BarterEntry { item: "minecraft:blackstone",      count: (8, 16), weight: 40 },
    ]
}

pub fn roll_bartering() -> Option<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let loot = bartering_loot();
    let total: u32 = loot.iter().map(|e| e.weight).sum();
    let mut roll = rng.gen_range(0..total);
    for entry in loot {
        if roll < entry.weight {
            let count = rng.gen_range(entry.count.0..=entry.count.1);
            return Some((entry.item, count));
        }
        roll -= entry.weight;
    }
    None
}

/// Gold item attraction range for piglins.
pub const GOLD_ATTRACT_RANGE: f64 = 6.0;

/// Items en or qui attirent les piglins.
pub fn gold_items() -> &'static [&'static str] {
    &[
        "minecraft:gold_ingot",
        "minecraft:gold_nugget",
        "minecraft:gold_block",
        "minecraft:raw_gold",
        "minecraft:raw_gold_block",
        "minecraft:gilded_blackstone",
        "minecraft:golden_apple",
        "minecraft:enchanted_golden_apple",
        "minecraft:gold_sword",
        "minecraft:gold_axe",
        "minecraft:gold_helmet",
        "minecraft:gold_chestplate",
        "minecraft:gold_leggings",
        "minecraft:gold_boots",
    ]
}

pub fn is_gold_item(name: &str) -> bool {
    gold_items().contains(&name)
}

/// Currency accepté par piglin en barter (lingot d'or).
pub const BARTER_CURRENCY: &str = "minecraft:gold_ingot";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_table_not_empty() {
        assert!(!bartering_loot().is_empty());
    }

    #[test]
    fn gold_ingot_is_gold() {
        assert!(is_gold_item("minecraft:gold_ingot"));
    }

    #[test]
    fn bartering_rolls_entry() {
        let res = roll_bartering();
        assert!(res.is_some());
    }
}
