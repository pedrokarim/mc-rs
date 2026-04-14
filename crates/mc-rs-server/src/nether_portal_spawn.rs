//! Nether portal — zombified piglin spawn logic vanilla.

use rand::Rng;

/// Zombified piglin spawn cooldown (~2000 ticks).
pub const SPAWN_COOLDOWN: u32 = 2000;
/// Spawn chance when triggered.
pub const SPAWN_CHANCE: f32 = 0.0125;

pub fn should_spawn_piglin(difficulty: u8) -> bool {
    if difficulty == 0 {
        return false;
    }
    let mut rng = rand::thread_rng();
    rng.gen::<f32>() < SPAWN_CHANCE * difficulty as f32
}

/// Portal frame size: min 4x5 (including frame), max 23x23.
pub const MIN_WIDTH: usize = 2;
pub const MAX_WIDTH: usize = 21;
pub const MIN_HEIGHT: usize = 3;
pub const MAX_HEIGHT: usize = 21;

pub fn is_valid_frame_size(width: usize, height: usize) -> bool {
    (MIN_WIDTH..=MAX_WIDTH).contains(&width) && (MIN_HEIGHT..=MAX_HEIGHT).contains(&height)
}

/// Lighter item types that can light nether portal.
pub fn ignition_items() -> &'static [&'static str] {
    &["minecraft:flint_and_steel", "minecraft:fire_charge"]
}

/// Trigger transport when player stands in portal (4 seconds survival, 1 sec creative).
pub const SURVIVAL_PORTAL_DELAY: u32 = 80;
pub const CREATIVE_PORTAL_DELAY: u32 = 20;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_min_size() {
        assert!(is_valid_frame_size(2, 3));
    }

    #[test]
    fn too_large_invalid() {
        assert!(!is_valid_frame_size(30, 30));
    }
}
