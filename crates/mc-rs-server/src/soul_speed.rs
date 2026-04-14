//! Soul speed — walk faster on soul sand/soil.

/// Speed bonus per level.
pub const SPEED_PER_LEVEL: f32 = 0.0405; // Vanilla
/// Max level.
pub const MAX_LEVEL: u8 = 3;
/// Boots durability loss chance per block moved.
pub const DURABILITY_LOSS_CHANCE: f32 = 0.04;

/// Blocks that trigger soul speed.
pub fn triggering_blocks() -> &'static [u16] {
    &[
        88,  // soul sand
        395, // soul soil
    ]
}

pub fn speed_bonus(level: u8) -> f32 {
    (SPEED_PER_LEVEL * level as f32).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_level_more_speed() {
        assert!(speed_bonus(3) > speed_bonus(1));
    }
}
