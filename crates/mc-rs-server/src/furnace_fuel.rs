//! Furnace fuel — burn time per item.

/// Burn time in ticks (200 ticks = 1 item smelted).
pub fn burn_ticks(item: &str) -> u32 {
    match item {
        "minecraft:lava_bucket" => 20_000,
        "minecraft:coal_block" => 16_000,
        "minecraft:blaze_rod" => 2400,
        "minecraft:coal" | "minecraft:charcoal" => 1600,
        "minecraft:dried_kelp_block" => 4000,
        "minecraft:boat" => 400,
        "minecraft:chest_boat" => 400,
        "minecraft:scaffolding" => 400,
        "minecraft:wood_planks" | "minecraft:oak_planks" | "minecraft:birch_planks"
        | "minecraft:spruce_planks" | "minecraft:jungle_planks" | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks" | "minecraft:mangrove_planks" | "minecraft:cherry_planks"
        | "minecraft:bamboo_planks" | "minecraft:bamboo_mosaic" | "minecraft:pale_oak_planks" => 300,
        "minecraft:ladder" => 300,
        "minecraft:wooden_pickaxe" | "minecraft:wooden_axe" | "minecraft:wooden_shovel"
        | "minecraft:wooden_sword" | "minecraft:wooden_hoe" => 200,
        "minecraft:log" | "minecraft:oak_log" | "minecraft:spruce_log" | "minecraft:birch_log"
        | "minecraft:jungle_log" | "minecraft:acacia_log" | "minecraft:dark_oak_log"
        | "minecraft:mangrove_log" | "minecraft:cherry_log" | "minecraft:pale_oak_log" => 300,
        "minecraft:stick" => 100,
        "minecraft:sapling" | "minecraft:oak_sapling" | "minecraft:spruce_sapling"
        | "minecraft:birch_sapling" | "minecraft:jungle_sapling" | "minecraft:acacia_sapling"
        | "minecraft:dark_oak_sapling" | "minecraft:mangrove_propagule" | "minecraft:cherry_sapling"
        | "minecraft:azalea" | "minecraft:flowering_azalea" => 100,
        "minecraft:wool" => 100,
        "minecraft:carpet" => 67,
        "minecraft:bowl" => 100,
        "minecraft:bamboo" => 50,
        _ => 0,
    }
}

pub fn is_fuel(item: &str) -> bool {
    burn_ticks(item) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coal_fuel() {
        assert!(is_fuel("minecraft:coal"));
    }

    #[test]
    fn stone_not_fuel() {
        assert!(!is_fuel("minecraft:stone"));
    }
}
