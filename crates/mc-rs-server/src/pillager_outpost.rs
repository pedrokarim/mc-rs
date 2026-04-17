//! Pillager outpost — tall tower with captain pillager.

/// Height in blocks.
pub const HEIGHT: u32 = 20;
/// Radius (block-wise).
pub const RADIUS: u32 = 4;

/// Chest loot.
pub fn chest_loot() -> &'static [(&'static str, u32, u32, u32)] {
    &[
        ("minecraft:dark_oak_log", 2, 3, 10),
        ("minecraft:crossbow", 1, 1, 2),
        ("minecraft:iron_ingot", 1, 5, 5),
        ("minecraft:arrow", 2, 3, 3),
        ("minecraft:tripwire_hook", 1, 1, 1),
        ("minecraft:iron_pickaxe", 1, 1, 2),
        ("minecraft:bottle_o_enchanting", 1, 1, 1),
    ]
}

/// Captain has ominous banner.
pub const CAPTAIN_HAS_OMINOUS_BANNER: bool = true;
/// Iron golem can spawn nearby.
pub const CAN_SPAWN_IRON_GOLEM: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_crossbow() {
        assert!(chest_loot()
            .iter()
            .any(|(i, _, _, _)| *i == "minecraft:crossbow"));
    }
}
