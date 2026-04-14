//! Item max stack sizes.

/// Default stack size.
pub const DEFAULT_STACK: u16 = 64;

/// Items that don't stack (tools, armor, bows, etc.)
pub fn max_stack_size(item: &str) -> u16 {
    match item {
        // Tools (non-stackable)
        s if s.contains("_pickaxe") || s.contains("_axe") || s.contains("_shovel")
            || s.contains("_sword") || s.contains("_hoe") => 1,
        // Armor
        s if s.contains("_helmet") || s.contains("_chestplate")
            || s.contains("_leggings") || s.contains("_boots") => 1,
        // Bows, arrows aren't here — items like bow.
        "minecraft:bow" | "minecraft:crossbow" | "minecraft:trident" | "minecraft:shield"
        | "minecraft:fishing_rod" | "minecraft:carrot_on_a_stick" | "minecraft:warped_fungus_on_a_stick"
        | "minecraft:elytra" | "minecraft:shears" | "minecraft:flint_and_steel"
        | "minecraft:bucket" | "minecraft:water_bucket" | "minecraft:lava_bucket"
        | "minecraft:milk_bucket" | "minecraft:cod_bucket" | "minecraft:pufferfish_bucket"
        | "minecraft:salmon_bucket" | "minecraft:tropical_fish_bucket" | "minecraft:axolotl_bucket"
        | "minecraft:tadpole_bucket" | "minecraft:powder_snow_bucket"
        | "minecraft:saddle" | "minecraft:minecart" | "minecraft:chest_minecart"
        | "minecraft:hopper_minecart" | "minecraft:tnt_minecart" | "minecraft:furnace_minecart"
        | "minecraft:oak_boat" | "minecraft:spruce_boat" | "minecraft:birch_boat"
        | "minecraft:jungle_boat" | "minecraft:acacia_boat" | "minecraft:dark_oak_boat"
        | "minecraft:mangrove_boat" | "minecraft:cherry_boat" | "minecraft:bamboo_raft"
        | "minecraft:written_book" | "minecraft:writable_book" | "minecraft:knowledge_book"
        | "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion"
        | "minecraft:cake" | "minecraft:armor_stand" | "minecraft:bed"
        | "minecraft:conduit" | "minecraft:end_crystal" | "minecraft:goat_horn"
        | "minecraft:totem_of_undying" | "minecraft:wolf_armor"
        => 1,
        // Snowballs, eggs, ender pearls stack to 16.
        "minecraft:snowball" | "minecraft:egg" | "minecraft:ender_pearl"
        | "minecraft:honey_bottle" | "minecraft:oak_sign" | "minecraft:spruce_sign"
        | "minecraft:birch_sign" | "minecraft:jungle_sign" | "minecraft:acacia_sign"
        | "minecraft:dark_oak_sign" | "minecraft:mangrove_sign" | "minecraft:cherry_sign"
        | "minecraft:bamboo_sign" | "minecraft:crimson_sign" | "minecraft:warped_sign"
        | "minecraft:pale_oak_sign"
        => 16,
        _ => DEFAULT_STACK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sword_unstackable() {
        assert_eq!(max_stack_size("minecraft:iron_sword"), 1);
    }

    #[test]
    fn stone_stacks_64() {
        assert_eq!(max_stack_size("minecraft:stone"), 64);
    }
}
