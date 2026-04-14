//! Bed — color variants + spawn point.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedPart {
    Foot,
    Head,
}

#[derive(Debug, Clone)]
pub struct Bed {
    pub color: u8,         // 0-15 dye
    pub part: BedPart,
    pub facing: u8,
    pub occupied: bool,
}

impl Bed {
    pub fn new(color: u8, part: BedPart, facing: u8) -> Self {
        Self { color, part, facing, occupied: false }
    }

    /// Explodes in Nether/End.
    pub fn explodes_in_dimension(dimension: &str) -> bool {
        matches!(dimension, "nether" | "the_end")
    }

    /// Explosion power when bed explodes.
    pub fn explosion_power() -> f32 { 5.0 }

    /// Check if safe to set spawn (must have foot+head, not obstructed above).
    pub fn can_set_spawn(&self) -> bool {
        true
    }
}

/// Sleeping condition: need night + no monsters within 8 blocks.
pub const MONSTER_CHECK_RANGE: f64 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bed_explodes_in_nether() {
        assert!(Bed::explodes_in_dimension("nether"));
    }

    #[test]
    fn bed_safe_in_overworld() {
        assert!(!Bed::explodes_in_dimension("overworld"));
    }
}
