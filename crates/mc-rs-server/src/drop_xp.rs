//! XP drops from blocks (ores).

use rand::Rng;

/// XP drop per ore type.
pub fn ore_xp_drop(block_id: u16) -> (u32, u32) {
    match block_id {
        16 | 620 => (0, 2),    // coal
        15 | 621 => (0, 0),    // iron (smelt for xp)
        14 | 622 => (0, 1),    // gold
        56 | 623 => (3, 7),    // diamond
        129 | 624 => (3, 7),   // emerald
        73 | 74 | 625 => (2, 5), // redstone
        21 | 626 => (2, 5),    // lapis
        153 => (2, 5),         // nether quartz
        627 => (3, 7),         // ancient debris (no — drops in raw)
        628 => (0, 0),         // copper (no XP)
        _ => (0, 0),
    }
}

pub fn roll_ore_xp(block_id: u16) -> u32 {
    let (min, max) = ore_xp_drop(block_id);
    if max == 0 {
        return 0;
    }
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Furnace smelting XP per item.
pub fn smelt_xp(item: &str) -> f32 {
    match item {
        "minecraft:iron_ore" | "minecraft:raw_iron" => 0.7,
        "minecraft:gold_ore" | "minecraft:raw_gold" => 1.0,
        "minecraft:copper_ore" | "minecraft:raw_copper" => 0.7,
        "minecraft:diamond_ore" => 1.0,
        "minecraft:emerald_ore" => 1.0,
        "minecraft:nether_quartz_ore" => 0.2,
        "minecraft:coal_ore" => 0.1,
        "minecraft:redstone_ore" => 0.7,
        "minecraft:lapis_ore" => 0.2,
        "minecraft:cobblestone" => 0.1,
        "minecraft:sand" => 0.1,
        "minecraft:clay" => 0.35,
        "minecraft:kelp" => 0.1,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_gives_xp() {
        assert!(ore_xp_drop(56).1 > 0);
    }

    #[test]
    fn iron_no_direct_xp() {
        assert_eq!(ore_xp_drop(15), (0, 0));
    }
}
