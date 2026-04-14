//! Sky light propagation.

/// Sky light maxes at 15 straight down.
pub const SKY_MAX: u8 = 15;

/// Compute sky light level at Y (above terrain).
pub fn sky_light_at(y: i32, terrain_height: i32) -> u8 {
    if y > terrain_height {
        SKY_MAX
    } else {
        0 // below terrain needs propagation
    }
}

/// Sky light decay through transparent blocks.
pub fn decay_through_block(current: u8, opacity: u8) -> u8 {
    current.saturating_sub(opacity.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn above_terrain_max() {
        assert_eq!(sky_light_at(100, 60), SKY_MAX);
    }
}
