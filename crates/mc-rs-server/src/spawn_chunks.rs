//! Spawn chunks (always loaded).

/// Default spawn chunk radius.
pub const DEFAULT_RADIUS: u32 = 2; // 5x5 chunks ticked
pub const KEEP_LOADED_RADIUS: u32 = 8; // 17x17 always loaded

/// Calculate which chunks are within spawn area.
pub fn is_spawn_chunk(chunk_x: i32, chunk_z: i32, spawn_x: i32, spawn_z: i32, radius: i32) -> bool {
    let dx = (chunk_x - spawn_x).abs();
    let dz = (chunk_z - spawn_z).abs();
    dx <= radius && dz <= radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_radius() {
        assert!(is_spawn_chunk(0, 0, 0, 0, 8));
    }

    #[test]
    fn far_outside() {
        assert!(!is_spawn_chunk(100, 100, 0, 0, 8));
    }
}
