//! Player respawn logic.

#[derive(Debug, Clone, Copy)]
pub struct SpawnPoint {
    pub world: &'static str,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub angle: f32,
    pub forced: bool, // /spawnpoint forces even if bed
}

impl SpawnPoint {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            world: "overworld",
            x, y, z,
            angle: 0.0,
            forced: false,
        }
    }
}

/// Respawn immune ticks after spawn.
pub const RESPAWN_IMMUNITY: u32 = 40;
/// Title shown on death.
pub const DEATH_TITLE: &str = "§4You Died!";

/// Check if bed spawn is still valid (bed intact).
pub fn is_valid_bed_spawn(block_id: u16) -> bool {
    block_id == 26 // Bed block
}

/// Check if respawn anchor has charges.
pub fn is_valid_anchor_spawn(charges: u8) -> bool {
    charges > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_needs_charges() {
        assert!(is_valid_anchor_spawn(1));
        assert!(!is_valid_anchor_spawn(0));
    }
}
