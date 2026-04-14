//! Mace — 1.21 weapon avec smash attack + density enchant.

#[derive(Debug, Clone)]
pub struct Mace {
    pub durability: u16,
    pub density_level: u8,
    pub breach_level: u8,
    pub wind_burst_level: u8,
}

/// Max durability (500).
pub const MAX_DURABILITY: u16 = 500;
/// Base damage (5).
pub const BASE_DAMAGE: f32 = 5.0;
/// Bonus damage per fall block (linear, +4 first, +2 every following).
pub fn smash_damage(fall_blocks: f32, density_level: u8) -> f32 {
    let base = if fall_blocks <= 3.0 {
        4.0 * fall_blocks
    } else if fall_blocks <= 8.0 {
        4.0 * 3.0 + 2.0 * (fall_blocks - 3.0)
    } else {
        4.0 * 3.0 + 2.0 * 5.0 + 1.0 * (fall_blocks - 8.0)
    };
    base + (density_level as f32) * 0.5 * fall_blocks
}

/// Wind burst provides double jump on smash.
pub fn wind_burst_boost(level: u8) -> f64 {
    0.25 + 0.25 * level as f64
}

/// Breach reduces armor effectiveness.
pub fn breach_armor_reduction(level: u8) -> f32 {
    0.15 * level as f32
}

impl Mace {
    pub fn new() -> Self {
        Self {
            durability: MAX_DURABILITY,
            density_level: 0,
            breach_level: 0,
            wind_burst_level: 0,
        }
    }
}

impl Default for Mace {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smash_scales_with_fall() {
        assert!(smash_damage(5.0, 0) > smash_damage(2.0, 0));
    }

    #[test]
    fn density_increases_damage() {
        assert!(smash_damage(5.0, 5) > smash_damage(5.0, 0));
    }
}
