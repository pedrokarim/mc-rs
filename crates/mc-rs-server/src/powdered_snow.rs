//! Powdered snow — freeze mechanic, walk on with leather boots.

/// Freeze time before damage (140 ticks = 7s).
pub const FULL_FREEZE_TICKS: u32 = 140;
/// Freeze damage per 2s once fully frozen.
pub const FREEZE_DAMAGE_PER_TICK: f32 = 1.0;
pub const FREEZE_DAMAGE_INTERVAL: u32 = 40;

/// Leather boots protect from sinking in powder snow.
pub fn can_walk_on_with(boots: Option<&str>) -> bool {
    matches!(boots, Some("minecraft:leather_boots"))
}

/// Slowness when standing in powdered snow (per tick).
pub const MOVEMENT_PENALTY: f32 = 0.5;

/// Rabbits + foxes don't sink.
pub fn mob_immune_to_sink(mob: &str) -> bool {
    matches!(mob, "minecraft:rabbit" | "minecraft:fox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leather_boots_walk() {
        assert!(can_walk_on_with(Some("minecraft:leather_boots")));
    }

    #[test]
    fn iron_boots_sink() {
        assert!(!can_walk_on_with(Some("minecraft:iron_boots")));
    }
}
