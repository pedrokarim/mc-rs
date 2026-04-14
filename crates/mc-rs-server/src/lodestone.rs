//! Lodestone compass tracks the lodestone position.

/// Lodestone block ID.
pub const LODESTONE_ID: u16 = 756;

#[derive(Debug, Clone, Copy)]
pub struct LodestoneCompassData {
    pub target_position: (i32, i32, i32),
    pub dimension: &'static str,
    pub tracked: bool,
}

impl LodestoneCompassData {
    pub fn new(pos: (i32, i32, i32), dim: &'static str) -> Self {
        Self { target_position: pos, dimension: dim, tracked: true }
    }

    /// If lodestone gone, compass spins.
    pub fn invalidate(&mut self) {
        self.tracked = false;
    }

    /// Compass spins randomly when in wrong dimension.
    pub fn points_random_if_wrong_dimension(&self, current_dim: &str) -> bool {
        self.dimension != current_dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_dim_random() {
        let d = LodestoneCompassData::new((0, 64, 0), "overworld");
        assert!(d.points_random_if_wrong_dimension("nether"));
    }
}
