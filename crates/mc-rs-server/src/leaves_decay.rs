//! Leaves decay — leaves break if no log within N blocks.

/// Max distance to valid log before decay.
pub const MAX_DISTANCE_FROM_LOG: u8 = 6;
/// Decay chance per random tick.
pub const DECAY_CHANCE: f32 = 0.05;
/// Stick drop chance (2%).
pub const STICK_DROP_CHANCE: f32 = 0.02;
/// Sapling drop chance (general).
pub const SAPLING_DROP_CHANCE: f32 = 0.05;
/// Sapling drop chance for jungle (2.5%).
pub const SAPLING_DROP_CHANCE_JUNGLE: f32 = 0.025;
/// Apple drop chance for oak (0.5%).
pub const APPLE_DROP_CHANCE: f32 = 0.005;

pub fn should_decay(persistent: bool, distance_to_log: Option<u8>) -> bool {
    if persistent {
        return false;
    }
    match distance_to_log {
        Some(d) if d <= MAX_DISTANCE_FROM_LOG => false,
        _ => true,
    }
}

/// Shears + silk touch prevent decay drops.
pub fn bypass_decay_drops(tool: &str) -> bool {
    matches!(tool, "minecraft:shears" | "minecraft:silk_touch")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistent_never_decays() {
        assert!(!should_decay(true, Some(100)));
    }

    #[test]
    fn far_leaves_decay() {
        assert!(should_decay(false, Some(10)));
    }

    #[test]
    fn close_leaves_ok() {
        assert!(!should_decay(false, Some(3)));
    }
}
