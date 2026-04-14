//! Scaffolding — portable climbing structure.

/// Max distance from floor (6 blocks).
pub const MAX_DISTANCE_FROM_FLOOR: u8 = 6;
/// Sneak to descend through scaffolding.
pub const SNEAK_TO_DESCEND: bool = true;

#[derive(Debug, Clone)]
pub struct Scaffolding {
    pub distance: u8,
    pub waterlogged: bool,
    pub is_bottom: bool,
}

impl Scaffolding {
    pub fn new() -> Self {
        Self { distance: 0, waterlogged: false, is_bottom: false }
    }

    /// Breaks if distance > max.
    pub fn should_break(&self) -> bool {
        self.distance > MAX_DISTANCE_FROM_FLOOR
    }
}

impl Default for Scaffolding {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_when_too_far() {
        let mut s = Scaffolding::new();
        s.distance = MAX_DISTANCE_FROM_FLOOR + 1;
        assert!(s.should_break());
    }
}
