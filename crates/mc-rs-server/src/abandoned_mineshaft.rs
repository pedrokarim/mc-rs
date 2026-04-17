//! Abandoned mineshaft — corridors, cave spiders, chest minecarts.

pub fn minecart_chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:rail", 1, 4, 20),
        ("minecraft:powered_rail", 1, 1, 5),
        ("minecraft:detector_rail", 1, 1, 5),
        ("minecraft:activator_rail", 1, 1, 5),
        ("minecraft:iron_ingot", 1, 5, 10),
        ("minecraft:gold_ingot", 1, 3, 5),
        ("minecraft:diamond", 1, 2, 3),
        ("minecraft:emerald", 1, 1, 1),
        ("minecraft:redstone", 4, 9, 5),
        ("minecraft:lapis_lazuli", 1, 5, 5),
        ("minecraft:bread", 1, 3, 15),
        ("minecraft:name_tag", 1, 1, 1),
        ("minecraft:enchanted_book", 1, 1, 5),
        ("minecraft:iron_pickaxe", 1, 1, 1),
    ]
}

/// Cave spider spawners at cobweb piles.
pub const CAVE_SPIDER_SPAWNERS: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_rail_items() {
        assert!(minecart_chest_loot()
            .iter()
            .any(|(i, _, _, _)| i.contains("rail")));
    }
}
