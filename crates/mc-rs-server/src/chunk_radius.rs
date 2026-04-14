//! Chunk radius / view distance calculation.

/// Default view distance (10 chunks).
pub const DEFAULT_VIEW_DISTANCE: u32 = 10;
/// Max view distance (32 chunks).
pub const MAX_VIEW_DISTANCE: u32 = 32;
/// Min view distance (4 chunks for playability).
pub const MIN_VIEW_DISTANCE: u32 = 4;

/// Chunks around player = (2r+1)^2.
pub fn chunks_in_radius(radius: u32) -> u32 {
    (2 * radius + 1).pow(2)
}

/// Check if chunk (cx, cz) is within radius.
pub fn is_in_radius(px: i32, pz: i32, cx: i32, cz: i32, radius: i32) -> bool {
    let dx = (cx - px).abs();
    let dz = (cz - pz).abs();
    dx <= radius && dz <= radius
}

/// Distance (chunk units).
pub fn chunk_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    ((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_5_includes_121_chunks() {
        assert_eq!(chunks_in_radius(5), 121);
    }

    #[test]
    fn same_chunk_in_radius() {
        assert!(is_in_radius(0, 0, 0, 0, 5));
    }
}
