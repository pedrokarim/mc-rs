//! Trap chest loot (jungle temple trap).

pub fn jungle_temple_chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    // (item, min, max, weight)
    &[
        ("minecraft:diamond", 1, 3, 1),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 2, 7, 5),
        ("minecraft:bamboo", 1, 3, 10),
        ("minecraft:emerald", 1, 3, 10),
        ("minecraft:bone", 4, 6, 20),
        ("minecraft:rotten_flesh", 3, 7, 15),
        ("minecraft:saddle", 1, 1, 15),
        ("minecraft:iron_horse_armor", 1, 1, 8),
        ("minecraft:gold_horse_armor", 1, 1, 4),
        ("minecraft:diamond_horse_armor", 1, 1, 1),
        ("minecraft:enchanted_book", 1, 1, 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_non_empty() {
        assert!(!jungle_temple_chest_loot().is_empty());
    }
}
