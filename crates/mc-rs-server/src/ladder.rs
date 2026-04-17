//! Ladder — climbable, wall-attached.

#[derive(Debug, Clone)]
pub struct Ladder {
    pub facing: u8, // 2=north, 3=south, 4=west, 5=east
    pub waterlogged: bool,
}

/// Climbing speed (up/down).
pub const CLIMBING_SPEED: f64 = 0.15;
/// Hold onto ladder (no fall damage).
pub fn cancels_fall_damage() -> bool {
    true
}

impl Ladder {
    pub fn new(facing: u8) -> Self {
        Self {
            facing,
            waterlogged: false,
        }
    }

    /// Attached side must be solid.
    pub fn attached_side_valid(block_id: u16) -> bool {
        !matches!(block_id, 0 | 6 | 31 | 32 | 37 | 38 | 39 | 40 | 50 | 65)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_invalid_attach() {
        assert!(!Ladder::attached_side_valid(0));
    }
}
