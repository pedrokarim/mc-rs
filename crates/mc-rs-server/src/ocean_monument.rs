//! Ocean monument structure.

/// Center of monument (treasure room).
pub const TREASURE_OFFSET: (i32, i32, i32) = (11, -15, 11);

/// Block types used.
pub fn blocks() -> &'static [&'static str] {
    &[
        "minecraft:prismarine",
        "minecraft:prismarine_bricks",
        "minecraft:dark_prismarine",
        "minecraft:sea_lantern",
        "minecraft:water",
        "minecraft:gold_block", // in treasure room
    ]
}

/// Treasure room gold blocks.
pub const TREASURE_GOLD_BLOCKS: u32 = 8;

/// Guardians spawn here.
pub fn native_mobs() -> &'static [&'static str] {
    &["minecraft:guardian", "minecraft:elder_guardian"]
}

/// 3 elder guardians per monument.
pub const ELDER_GUARDIAN_COUNT: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_prismarine() {
        assert!(blocks().contains(&"minecraft:prismarine"));
    }
}
