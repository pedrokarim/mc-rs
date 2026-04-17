//! Item properties — port PMMP `src/item/*`.
//! Max stack size, digeur/outil, food, placeable, etc.

/// Max stack size d'un item. Par défaut 64, exceptions vanilla.
pub fn max_stack_size(item_name: &str) -> u16 {
    match item_name {
        // Non stackable
        "minecraft:wooden_sword"
        | "minecraft:stone_sword"
        | "minecraft:iron_sword"
        | "minecraft:golden_sword"
        | "minecraft:diamond_sword"
        | "minecraft:netherite_sword" => 1,
        "minecraft:wooden_pickaxe"
        | "minecraft:stone_pickaxe"
        | "minecraft:iron_pickaxe"
        | "minecraft:golden_pickaxe"
        | "minecraft:diamond_pickaxe"
        | "minecraft:netherite_pickaxe" => 1,
        "minecraft:wooden_axe"
        | "minecraft:stone_axe"
        | "minecraft:iron_axe"
        | "minecraft:golden_axe"
        | "minecraft:diamond_axe"
        | "minecraft:netherite_axe" => 1,
        "minecraft:wooden_shovel"
        | "minecraft:stone_shovel"
        | "minecraft:iron_shovel"
        | "minecraft:golden_shovel"
        | "minecraft:diamond_shovel"
        | "minecraft:netherite_shovel" => 1,
        "minecraft:wooden_hoe"
        | "minecraft:stone_hoe"
        | "minecraft:iron_hoe"
        | "minecraft:golden_hoe"
        | "minecraft:diamond_hoe"
        | "minecraft:netherite_hoe" => 1,
        "minecraft:leather_helmet"
        | "minecraft:leather_chestplate"
        | "minecraft:leather_leggings"
        | "minecraft:leather_boots" => 1,
        "minecraft:iron_helmet"
        | "minecraft:iron_chestplate"
        | "minecraft:iron_leggings"
        | "minecraft:iron_boots" => 1,
        "minecraft:chainmail_helmet"
        | "minecraft:chainmail_chestplate"
        | "minecraft:chainmail_leggings"
        | "minecraft:chainmail_boots" => 1,
        "minecraft:golden_helmet"
        | "minecraft:golden_chestplate"
        | "minecraft:golden_leggings"
        | "minecraft:golden_boots" => 1,
        "minecraft:diamond_helmet"
        | "minecraft:diamond_chestplate"
        | "minecraft:diamond_leggings"
        | "minecraft:diamond_boots" => 1,
        "minecraft:netherite_helmet"
        | "minecraft:netherite_chestplate"
        | "minecraft:netherite_leggings"
        | "minecraft:netherite_boots" => 1,
        "minecraft:bow" | "minecraft:crossbow" | "minecraft:trident" | "minecraft:fishing_rod" => 1,
        "minecraft:shears"
        | "minecraft:shield"
        | "minecraft:carrot_on_a_stick"
        | "minecraft:warped_fungus_on_a_stick" => 1,
        "minecraft:totem_of_undying" | "minecraft:saddle" | "minecraft:elytra" => 1,
        "minecraft:filled_map" | "minecraft:written_book" | "minecraft:writable_book" => 1,
        "minecraft:lava_bucket"
        | "minecraft:water_bucket"
        | "minecraft:milk_bucket"
        | "minecraft:cod_bucket"
        | "minecraft:salmon_bucket"
        | "minecraft:axolotl_bucket"
        | "minecraft:tropical_fish_bucket"
        | "minecraft:pufferfish_bucket"
        | "minecraft:tadpole_bucket" => 1,
        "minecraft:oak_boat"
        | "minecraft:birch_boat"
        | "minecraft:spruce_boat"
        | "minecraft:jungle_boat"
        | "minecraft:acacia_boat"
        | "minecraft:dark_oak_boat"
        | "minecraft:mangrove_boat"
        | "minecraft:cherry_boat" => 1,
        "minecraft:oak_chest_boat"
        | "minecraft:birch_chest_boat"
        | "minecraft:spruce_chest_boat"
        | "minecraft:jungle_chest_boat"
        | "minecraft:acacia_chest_boat"
        | "minecraft:dark_oak_chest_boat"
        | "minecraft:mangrove_chest_boat"
        | "minecraft:cherry_chest_boat" => 1,
        "minecraft:honey_bottle"
        | "minecraft:potion"
        | "minecraft:lingering_potion"
        | "minecraft:splash_potion" => 1,
        "minecraft:minecart"
        | "minecraft:chest_minecart"
        | "minecraft:furnace_minecart"
        | "minecraft:hopper_minecart"
        | "minecraft:tnt_minecart"
        | "minecraft:command_block_minecart" => 1,
        // Stack 16
        "minecraft:ender_pearl"
        | "minecraft:snowball"
        | "minecraft:egg"
        | "minecraft:bucket"
        | "minecraft:banner"
        | "minecraft:sign" => 16,
        // Default = 64
        _ => 64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_stacks_64() {
        assert_eq!(max_stack_size("minecraft:stone"), 64);
    }

    #[test]
    fn sword_stacks_1() {
        assert_eq!(max_stack_size("minecraft:iron_sword"), 1);
    }

    #[test]
    fn ender_pearl_stacks_16() {
        assert_eq!(max_stack_size("minecraft:ender_pearl"), 16);
    }

    #[test]
    fn bucket_stacks_16() {
        assert_eq!(max_stack_size("minecraft:bucket"), 16);
    }

    #[test]
    fn water_bucket_stacks_1() {
        assert_eq!(max_stack_size("minecraft:water_bucket"), 1);
    }
}
