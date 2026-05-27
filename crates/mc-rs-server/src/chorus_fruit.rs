//! Chorus fruit / plant — End flora.

/// Teleport range when eaten.
pub const TELEPORT_RANGE: f64 = 8.0;
/// Hunger restored.
pub const HUNGER_RESTORED: u8 = 4;
/// Saturation.
pub const SATURATION: f32 = 2.4;
/// Can be eaten even on full hunger.
pub const ALWAYS_EDIBLE: bool = true;

/// Chorus plant growth — requires air above.
pub const GROWTH_CHANCE: f32 = 0.1;
/// Max height of chorus plant (~7 blocks).
pub const MAX_HEIGHT: u8 = 7;

/// Cooking chorus fruit → popped chorus fruit (for end rod, purpur).
pub fn popped_chorus_result() -> &'static str {
    "minecraft:popped_chorus_fruit"
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn always_edible_flag() {
        assert!(ALWAYS_EDIBLE);
    }
}
