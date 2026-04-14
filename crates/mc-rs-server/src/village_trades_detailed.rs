//! Detailed villager trade lists for all professions.

/// Cleric trades by level.
pub fn cleric_trades_level_2() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:lapis_lazuli", 1, "minecraft:emerald", 1),
        ("minecraft:emerald", 1, "minecraft:redstone", 4),
    ]
}

pub fn cleric_trades_level_3() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:emerald", 1, "minecraft:glowstone", 1),
        ("minecraft:rabbit_foot", 2, "minecraft:emerald", 1),
    ]
}

pub fn cleric_trades_level_4() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:scute", 4, "minecraft:emerald", 1),
        ("minecraft:glass_bottle", 9, "minecraft:emerald", 1),
        ("minecraft:emerald", 5, "minecraft:ender_pearl", 1),
    ]
}

pub fn cleric_trades_level_5() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:nether_wart", 22, "minecraft:emerald", 1),
        ("minecraft:emerald", 3, "minecraft:experience_bottle", 1),
    ]
}

/// Armorer trades.
pub fn armorer_trades_level_2() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:iron_ingot", 4, "minecraft:emerald", 1),
        ("minecraft:emerald", 36, "minecraft:bell", 1),
        ("minecraft:emerald", 5, "minecraft:chainmail_leggings", 1),
    ]
}

pub fn armorer_trades_level_3() -> &'static [(&'static str, u32, &'static str, u32)] {
    &[
        ("minecraft:lava_bucket", 1, "minecraft:emerald", 1),
        ("minecraft:emerald", 3, "minecraft:chainmail_helmet", 1),
    ]
}

/// Librarian Enchanted Book trade.
pub fn librarian_enchanted_book_cost_range() -> (u32, u32) {
    (5, 64) // 5-64 emeralds depending on enchant
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleric_has_redstone() {
        let trades = cleric_trades_level_2();
        assert!(trades.iter().any(|(_, _, sell, _)| *sell == "minecraft:redstone"));
    }
}
