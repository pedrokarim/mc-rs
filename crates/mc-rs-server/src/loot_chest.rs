//! Loot chest generation (dungeons, villages, etc.)

#[derive(Debug, Clone)]
pub struct LootEntry {
    pub item: &'static str,
    pub min: u32,
    pub max: u32,
    pub weight: u32,
    pub enchant_book: bool,
}

#[derive(Debug, Clone)]
pub struct LootPool {
    pub rolls: (u32, u32),
    pub entries: Vec<LootEntry>,
}

/// Dungeon chest loot (simple monster rooms).
pub fn dungeon_loot() -> LootPool {
    LootPool {
        rolls: (3, 5),
        entries: vec![
            LootEntry {
                item: "minecraft:music_disc_cat",
                min: 1,
                max: 1,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:music_disc_13",
                min: 1,
                max: 1,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:name_tag",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:golden_apple",
                min: 1,
                max: 1,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:enchanted_golden_apple",
                min: 1,
                max: 1,
                weight: 1,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:iron_ingot",
                min: 1,
                max: 4,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:gold_ingot",
                min: 1,
                max: 4,
                weight: 5,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:redstone",
                min: 1,
                max: 4,
                weight: 5,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:bread",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:wheat",
                min: 1,
                max: 4,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:bucket",
                min: 1,
                max: 1,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:book",
                min: 1,
                max: 1,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:beetroot_seeds",
                min: 2,
                max: 4,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:saddle",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:horse_armor_iron",
                min: 1,
                max: 1,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:horse_armor_gold",
                min: 1,
                max: 1,
                weight: 10,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:horse_armor_diamond",
                min: 1,
                max: 1,
                weight: 5,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:enchanted_book",
                min: 1,
                max: 1,
                weight: 10,
                enchant_book: true,
            },
        ],
    }
}

/// Desert temple treasure.
pub fn desert_temple_loot() -> LootPool {
    LootPool {
        rolls: (2, 4),
        entries: vec![
            LootEntry {
                item: "minecraft:diamond",
                min: 1,
                max: 3,
                weight: 5,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:iron_ingot",
                min: 1,
                max: 5,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:gold_ingot",
                min: 2,
                max: 7,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:emerald",
                min: 1,
                max: 3,
                weight: 15,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:bone",
                min: 4,
                max: 6,
                weight: 25,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:spider_eye",
                min: 1,
                max: 3,
                weight: 25,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:rotten_flesh",
                min: 3,
                max: 7,
                weight: 25,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:saddle",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:enchanted_book",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: true,
            },
            LootEntry {
                item: "minecraft:golden_apple",
                min: 1,
                max: 1,
                weight: 20,
                enchant_book: false,
            },
            LootEntry {
                item: "minecraft:enchanted_golden_apple",
                min: 1,
                max: 1,
                weight: 2,
                enchant_book: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_has_entries() {
        assert!(!dungeon_loot().entries.is_empty());
    }
}
