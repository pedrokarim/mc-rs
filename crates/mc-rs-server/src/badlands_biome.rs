//! Mesa / Badlands biome.

pub const BIOME_ID: u8 = 37;
pub const BIOME_PLATEAU_ID: u8 = 39;
pub const BIOME_ERODED_ID: u8 = 165;

pub fn unique_blocks() -> &'static [&'static str] {
    &[
        "minecraft:terracotta",
        "minecraft:yellow_terracotta",
        "minecraft:orange_terracotta",
        "minecraft:red_terracotta",
        "minecraft:brown_terracotta",
        "minecraft:white_terracotta",
        "minecraft:red_sand",
        "minecraft:red_sandstone",
    ]
}

/// Extra gold ore generation.
pub fn extra_gold_ore() -> bool { true }
pub const GOLD_ORE_MAX_Y: i32 = 80;

/// No passive mobs spawn.
pub fn passive_mobs_spawn() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_terracotta() {
        assert!(unique_blocks().contains(&"minecraft:terracotta"));
    }
}
