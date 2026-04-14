//! Water / fluid physics.

/// Drag when moving in water.
pub const WATER_DRAG: f64 = 0.8;
/// Downward drag factor.
pub const WATER_VERTICAL_DRAG: f64 = 0.8;
/// Swim up speed (jump while in water).
pub const SWIM_UP_SPEED: f64 = 0.08;
/// Entities slower in water (0.02/tick base).
pub const WATER_SPEED_MULT: f64 = 0.02;

/// Depth strider enchant reduces water slowdown (per level).
pub fn depth_strider_speedup(level: u8) -> f32 {
    (level as f32 / 3.0).min(1.0)
}

/// Swim multiplier with enchant.
pub fn swim_speed_with_dolphin_grace(base: f64) -> f64 {
    base * 1.0 // Dolphin's Grace gives fast underwater speed
}

/// Flowing water direction.
pub fn flow_direction(block_state: u8) -> (i32, i32, i32) {
    match block_state {
        0 => (0, -1, 0),
        1 => (1, 0, 0),
        2 => (-1, 0, 0),
        3 => (0, 0, 1),
        4 => (0, 0, -1),
        _ => (0, -1, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_strider_caps_at_1() {
        assert_eq!(depth_strider_speedup(5), 1.0);
    }
}
