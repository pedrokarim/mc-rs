//! Pointed dripstone — stalagmites/stalactites.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DripstoneOrientation {
    Up,   // Stalagmite
    Down, // Stalactite
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DripstoneSize {
    Tip,
    TipMerged,
    Frustum,
    Middle,
    Base,
}

#[derive(Debug, Clone)]
pub struct PointedDripstone {
    pub orientation: DripstoneOrientation,
    pub size: DripstoneSize,
    pub is_waterlogged: bool,
}

/// Sharp dripstone damages entities falling on it.
pub fn landing_damage(fall_distance: f32) -> f32 {
    (fall_distance * 2.0).clamp(2.0, 40.0)
}

/// Thrown trident breaks dripstone.
pub const TRIDENT_DESTROYS_TIP: bool = true;

/// Dripstone grows from water drops (very slow — 3,700 blocks per day vanilla).
pub const GROWTH_CHANCE_PER_RANDOM_TICK: f32 = 0.000_079;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_capped_at_40() {
        assert_eq!(landing_damage(100.0), 40.0);
    }

    #[test]
    fn damage_min_2() {
        assert_eq!(landing_damage(0.0), 2.0);
    }
}
