//! Golem/wither crafting — build patterns.

/// Iron golem: T of iron blocks + pumpkin head.
pub fn iron_golem_pattern() -> Vec<Vec<u16>> {
    vec![
        vec![0, 86, 0],   // air, pumpkin, air (top)
        vec![42, 42, 42], // iron, iron, iron (middle)
        vec![0, 42, 0],   // air, iron, air (bottom)
    ]
}

/// Wither: 4-T of soul sand/soil + 3 wither skulls.
pub fn wither_pattern() -> Vec<Vec<u16>> {
    vec![
        vec![0, 397, 397, 397, 0], // 3 wither skulls (top)
        vec![88, 88, 88, 88, 88],  // soul sand row (4 blocks)
        vec![0, 88, 0, 88, 0],     // T shape (2 blocks)
    ]
}

/// Snow golem: snow blocks + pumpkin.
pub fn snow_golem_pattern() -> Vec<Vec<u16>> {
    vec![
        vec![86], // pumpkin
        vec![80], // snow block
        vec![80], // snow block
    ]
}

pub const PUMPKIN_ID: u16 = 86;
pub const CARVED_PUMPKIN_ID: u16 = 86;
pub const IRON_BLOCK_ID: u16 = 42;
pub const SNOW_BLOCK_ID: u16 = 80;
pub const SOUL_SAND_ID: u16 = 88;
pub const SOUL_SOIL_ID: u16 = 395;
pub const WITHER_SKULL_ID: u16 = 397;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_golem_has_pumpkin() {
        let p = iron_golem_pattern();
        assert_eq!(p[0][1], PUMPKIN_ID);
    }
}
