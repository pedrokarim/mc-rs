//! Igloo structure.

/// 50% chance has basement with villager + zombie villager + brewing stand.
pub const HAS_BASEMENT_CHANCE: f32 = 0.5;

pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:apple", 1, 3, 15),
        ("minecraft:coal", 1, 4, 15),
        ("minecraft:gold_nugget", 1, 3, 10),
        ("minecraft:stone_axe", 1, 1, 2),
        ("minecraft:emerald", 1, 1, 1),
        ("minecraft:wheat", 2, 3, 10),
        ("minecraft:rotten_flesh", 1, 1, 10),
        ("minecraft:golden_apple", 1, 1, 1),
    ]
}

pub fn valid_biomes() -> &'static [u8] {
    &[12, 13, 30, 31, 140, 158] // snowy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_has_golden_apple() {
        assert!(chest_loot().iter().any(|(i, _, _, _)| *i == "minecraft:golden_apple"));
    }
}
