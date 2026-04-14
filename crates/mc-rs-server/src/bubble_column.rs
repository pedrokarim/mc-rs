//! Bubble column — soul sand / magma block under water.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleDirection {
    Up,   // Soul sand
    Down, // Magma block
}

/// Push speed.
pub const PUSH_SPEED: f64 = 0.04;
/// Negate fall damage.
pub fn negates_fall_damage() -> bool { true }

/// Propels boat/player vertically.
pub fn propels_boat() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_distinct() {
        assert_ne!(BubbleDirection::Up, BubbleDirection::Down);
    }
}
