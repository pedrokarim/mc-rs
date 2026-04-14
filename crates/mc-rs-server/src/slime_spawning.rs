//! Slime spawning — slime chunks + swamp biome surface.

/// Min/Max Y for slime chunk spawning.
pub const SLIME_CHUNK_MIN_Y: i32 = -64;
pub const SLIME_CHUNK_MAX_Y: i32 = 39;

/// Swamp spawning Y range.
pub const SWAMP_MIN_Y: i32 = 51;
pub const SWAMP_MAX_Y: i32 = 69;
/// Swamp light level max.
pub const SWAMP_MAX_LIGHT: u8 = 7;

/// Slime chunk detection (vanilla Java RNG).
pub fn is_slime_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> bool {
    use std::num::Wrapping;
    let cx = Wrapping(chunk_x as u64);
    let cz = Wrapping(chunk_z as u64);
    let seed_w = Wrapping(seed);
    let base = seed_w
        + cx * Wrapping(0x4c1906)
        + cx * cx * Wrapping(0x5ac0db)
        + cz * Wrapping(0x5f24f)
        + cz * cz * Wrapping(0x4307a7);
    let xored = Wrapping(base.0 ^ 0x3ad8025f);
    let a = Wrapping(1181783497276652981_u64);
    let b = Wrapping(0x1_u64);
    let next = xored.0.wrapping_mul(a.0).wrapping_add(b.0);
    (next >> 17) % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slime_chunk_deterministic() {
        let s1 = is_slime_chunk(0, 0, 12345);
        let s2 = is_slime_chunk(0, 0, 12345);
        assert_eq!(s1, s2);
    }
}
