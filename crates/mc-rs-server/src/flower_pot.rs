//! Flower Pot — displays a single flower/plant.

#[derive(Debug, Clone)]
pub struct FlowerPot {
    pub content: Option<&'static str>,
}

/// Items that can be placed in pot.
pub fn pottable_items() -> &'static [&'static str] {
    &[
        // Small flowers
        "minecraft:dandelion",
        "minecraft:poppy",
        "minecraft:blue_orchid",
        "minecraft:allium",
        "minecraft:azure_bluet",
        "minecraft:red_tulip",
        "minecraft:orange_tulip",
        "minecraft:white_tulip",
        "minecraft:pink_tulip",
        "minecraft:oxeye_daisy",
        "minecraft:cornflower",
        "minecraft:lily_of_the_valley",
        "minecraft:wither_rose",
        "minecraft:torchflower",
        "minecraft:pitcher_plant",
        // Saplings
        "minecraft:oak_sapling",
        "minecraft:spruce_sapling",
        "minecraft:birch_sapling",
        "minecraft:jungle_sapling",
        "minecraft:acacia_sapling",
        "minecraft:dark_oak_sapling",
        "minecraft:mangrove_propagule",
        "minecraft:cherry_sapling",
        // Fungus/mushroom
        "minecraft:brown_mushroom",
        "minecraft:red_mushroom",
        "minecraft:crimson_fungus",
        "minecraft:warped_fungus",
        "minecraft:crimson_roots",
        "minecraft:warped_roots",
        // Special
        "minecraft:cactus",
        "minecraft:bamboo",
        "minecraft:fern",
        "minecraft:dead_bush",
        "minecraft:azalea",
        "minecraft:flowering_azalea",
        "minecraft:open_eyeblossom",
        "minecraft:closed_eyeblossom",
    ]
}

impl FlowerPot {
    pub fn new() -> Self {
        Self { content: None }
    }

    pub fn place(&mut self, item: &'static str) -> bool {
        if self.content.is_some() {
            return false;
        }
        if !pottable_items().contains(&item) {
            return false;
        }
        self.content = Some(item);
        true
    }

    pub fn take(&mut self) -> Option<&'static str> {
        self.content.take()
    }
}

impl Default for FlowerPot {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poppy_pottable() {
        let mut p = FlowerPot::new();
        assert!(p.place("minecraft:poppy"));
    }

    #[test]
    fn stone_not_pottable() {
        let mut p = FlowerPot::new();
        assert!(!p.place("minecraft:stone"));
    }
}
