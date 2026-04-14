//! Fortune enchant + drop multipliers — port PMMP `src/block/Block::getDropsForCompatibleTool`.

use rand::Rng;

/// Drop multiplier selon fortune level et type de bloc.
pub fn fortune_multiplier(block_name: &str, fortune_level: u8) -> u32 {
    if fortune_level == 0 {
        return 1;
    }
    let mut rng = rand::thread_rng();
    match block_name {
        "minecraft:coal_ore" | "minecraft:deepslate_coal_ore"
        | "minecraft:diamond_ore" | "minecraft:deepslate_diamond_ore"
        | "minecraft:emerald_ore" | "minecraft:deepslate_emerald_ore"
        | "minecraft:redstone_ore" | "minecraft:deepslate_redstone_ore"
        | "minecraft:lapis_ore" | "minecraft:deepslate_lapis_ore"
        | "minecraft:nether_quartz_ore" => {
            // Binomial-like drop : 1 + random(0, fortune)
            1 + rng.gen_range(0..=fortune_level as u32)
        }
        "minecraft:carrots" | "minecraft:potatoes" | "minecraft:wheat" => {
            // Fortune doesn't affect these, only looting.
            1
        }
        _ => 1,
    }
}

/// Silk touch capability : si true, le drop est le bloc lui-même (pas item).
pub fn silk_touch_changes_drop(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:stone"
            | "minecraft:coal_ore"
            | "minecraft:diamond_ore"
            | "minecraft:emerald_ore"
            | "minecraft:redstone_ore"
            | "minecraft:lapis_ore"
            | "minecraft:grass_block"
            | "minecraft:mycelium"
            | "minecraft:podzol"
            | "minecraft:gravel"
            | "minecraft:ice"
            | "minecraft:packed_ice"
            | "minecraft:blue_ice"
            | "minecraft:snow_block"
            | "minecraft:ender_chest"
            | "minecraft:cobweb"
            | "minecraft:glowstone"
            | "minecraft:sea_lantern"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortune_boosts_diamond_drop() {
        // Statistiquement plusieurs essais : au moins un avec 2+.
        let mut got_2_plus = false;
        for _ in 0..100 {
            if fortune_multiplier("minecraft:diamond_ore", 3) >= 2 {
                got_2_plus = true;
                break;
            }
        }
        assert!(got_2_plus);
    }

    #[test]
    fn grass_is_silk_touch_only() {
        assert!(silk_touch_changes_drop("minecraft:grass_block"));
    }
}
