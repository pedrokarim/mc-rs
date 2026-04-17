//! Swift Sneak — faster sneaking (treasure enchant, Ancient Cities).

/// Speed multiplier per level (base is 30% sneaking speed).
pub const BASE_SNEAK_SPEED: f32 = 0.3;
pub const PER_LEVEL_BONUS: f32 = 0.15;
/// Max level.
pub const MAX_LEVEL: u8 = 3;

pub fn sneaking_speed(level: u8) -> f32 {
    BASE_SNEAK_SPEED + PER_LEVEL_BONUS * level as f32
}

/// Treasure enchantment — only found in Ancient Cities.
pub fn is_treasure() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_3_near_full_speed() {
        assert!(sneaking_speed(3) > 0.7);
    }
}
