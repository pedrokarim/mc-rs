//! Moss — bone-meal can spread moss + grow foliage.

/// Moss block spreads when bone-mealed.
pub const BONE_MEAL_RADIUS: u32 = 3;
/// Foliage density on spread (flowers, grass, trees).
pub const FOLIAGE_DENSITY: f32 = 0.5;

/// Moss converts certain blocks.
pub fn convertible_blocks() -> &'static [u16] {
    &[
        1,   // stone → mossy cobblestone
        4,   // cobblestone → mossy cobblestone
        98,  // stone bricks → mossy stone bricks
        139, // cobblestone wall → mossy wall
    ]
}

/// Conversion map.
pub fn convert_block(block_id: u16) -> u16 {
    match block_id {
        1 => 4,   // stone → cobblestone (not really moss, simplification)
        4 => 48,  // cobblestone → mossy cobblestone
        98 => 97, // stone brick → mossy stone brick
        _ => block_id,
    }
}

/// Sprouts on moss: grass, flowers, azalea.
pub fn sprouts() -> &'static [&'static str] {
    &[
        "minecraft:tall_grass",
        "minecraft:azalea",
        "minecraft:flowering_azalea",
        "minecraft:moss_carpet",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_stone_brick() {
        assert_eq!(convert_block(98), 97);
    }
}
