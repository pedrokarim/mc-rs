//! Soul fire — ignited on soul sand/soil.

/// Damage per tick (same as regular fire: 1.0).
pub const DAMAGE_PER_TICK: f32 = 2.0; // Soul fire does more damage
/// Light emission (10 vs 15 for regular).
pub const LIGHT_LEVEL: u8 = 10;
/// Scares piglins.
pub fn scares_piglins() -> bool { true }

#[cfg(test)]
mod tests {
    #[test]
    fn soul_damage_higher() {
        assert!(super::DAMAGE_PER_TICK > 1.0);
    }
}
