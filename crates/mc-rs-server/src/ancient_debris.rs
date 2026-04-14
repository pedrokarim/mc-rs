//! Ancient debris generation.

/// Generation range in Nether (Y 8-22, with max chance at 15).
pub const MIN_Y: i32 = 8;
pub const MAX_Y: i32 = 22;
pub const PEAK_Y: i32 = 15;

/// Spawns in small veins (max 3 blocks).
pub const MAX_VEIN_SIZE: u32 = 3;
/// Chance per chunk (about 1 attempt per chunk).
pub const ATTEMPTS_PER_CHUNK: u32 = 1;

/// Blast resistance very high.
pub const BLAST_RESISTANCE: f32 = 1200.0;
/// Hardness.
pub const HARDNESS: f32 = 30.0;

/// Can't be blown up by regular explosions.
pub fn resists_explosion(explosion_power: f32) -> bool {
    explosion_power < 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tnt_cant_break() {
        assert!(resists_explosion(4.0));
    }
}
