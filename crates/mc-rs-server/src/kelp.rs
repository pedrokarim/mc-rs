//! Kelp — grows in water up to 26 blocks tall.

/// Max kelp column height.
pub const MAX_HEIGHT: u8 = 26;
/// Growth chance (per random tick).
pub const GROWTH_CHANCE: f32 = 0.14;
/// Needs water above it.
pub fn needs_water() -> bool {
    true
}

/// Drops kelp when broken (not silk touch needed).
pub fn drops() -> &'static str {
    "minecraft:kelp"
}
/// Dried kelp smelting result.
pub fn dried_kelp_item() -> &'static str {
    "minecraft:dried_kelp"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_water_true() {
        assert!(needs_water());
    }
}
