//! Bamboo — grows 12-16 blocks tall.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BambooStage {
    Sapling,
    Young,
    Adult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BambooLeaves {
    None,
    Small,
    Large,
}

/// Max height.
pub const MAX_HEIGHT: u8 = 16;
/// Min height for adult.
pub const ADULT_MIN_HEIGHT: u8 = 3;
/// Growth chance.
pub const GROWTH_CHANCE: f32 = 0.15;

/// Bamboo drops itself.
pub fn drops_per_block() -> u32 {
    1
}
/// Top bamboo drop bonus sapling.
pub fn top_drop() -> &'static str {
    "minecraft:bamboo"
}

/// Valid ground blocks for bamboo.
pub fn valid_ground() -> &'static [u16] {
    &[
        2, 3, 12, 152, 110, 208, // dirt-like
        86,  // pumpkin (no)
        245, // bamboo itself
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bamboo_on_grass() {
        assert!(valid_ground().contains(&2));
    }
}
