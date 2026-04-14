//! Sugar cane — grows up to 3 blocks tall.

/// Max growth height.
pub const MAX_HEIGHT: u8 = 3;
/// Needs water adjacent to base block.
pub fn needs_water() -> bool { true }
/// Growth chance per random tick.
pub const GROWTH_CHANCE: f32 = 0.10;

/// Valid ground blocks.
pub fn valid_ground() -> &'static [u16] {
    &[
        2,  // grass
        3,  // dirt
        12, // sand
        110, // mycelium
        208, // podzol
        152, // red sand
    ]
}

/// Sugar cane harvests into 1 sugar cane.
pub fn drops() -> &'static str {
    "minecraft:sugar_cane"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_valid() {
        assert!(valid_ground().contains(&2));
    }

    #[test]
    fn stone_invalid() {
        assert!(!valid_ground().contains(&1));
    }
}
