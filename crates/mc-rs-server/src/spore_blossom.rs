//! Spore blossom — hanging pink flower (lush caves).

pub const ITEM_ID: &str = "minecraft:spore_blossom";

/// Spore particles emission range.
pub const PARTICLE_RANGE: f64 = 10.0;
/// Spore particles per tick.
pub const PARTICLES_PER_TICK: u32 = 1;
/// Falls when no block above.
pub fn needs_ceiling() -> bool {
    true
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    #[test]
    fn range_positive() {
        assert!(super::PARTICLE_RANGE > 0.0);
    }
}
