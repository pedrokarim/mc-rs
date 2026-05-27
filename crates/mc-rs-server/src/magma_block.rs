//! Magma block — damages entities standing on top (unless sneaking or frost walker).

/// Damage per tick.
pub const DAMAGE_PER_TICK: f32 = 1.0;
/// Damage interval.
pub const DAMAGE_INTERVAL: u32 = 20; // 1 per second

/// Turns water above into bubble column.
pub fn creates_bubble_column() -> bool {
    true
}

/// Frost walker enchant prevents damage.
pub fn frost_walker_prevents_damage() -> bool {
    true
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    #[test]
    fn damage_nonzero() {
        assert!(super::DAMAGE_PER_TICK > 0.0);
    }
}
