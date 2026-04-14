//! Banner block types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerPlacement {
    Standing(u8), // Rotation 0-15
    Wall(u8),     // Facing direction
}

#[derive(Debug, Clone)]
pub struct PlacedBanner {
    pub color: u8,
    pub placement: BannerPlacement,
    pub patterns: Vec<u8>, // Pattern IDs
}

/// Illager banner (found on raid captain/pillager patrol).
pub const OMINOUS_BANNER_PATTERN_COUNT: usize = 6;

/// Wash banner in cauldron removes top pattern.
pub fn wash_in_cauldron() -> bool { true }

impl PlacedBanner {
    pub fn new(color: u8, placement: BannerPlacement) -> Self {
        Self { color, placement, patterns: Vec::new() }
    }

    pub fn wash(&mut self) -> bool {
        self.patterns.pop().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wash_removes_pattern() {
        let mut b = PlacedBanner::new(0, BannerPlacement::Standing(0));
        b.patterns.push(1);
        b.patterns.push(2);
        assert!(b.wash());
        assert_eq!(b.patterns.len(), 1);
    }
}
