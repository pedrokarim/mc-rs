//! World border damage & warnings.

/// Default world border size (60m blocks).
pub const DEFAULT_WORLD_BORDER_SIZE: f64 = 59_999_968.0;
/// Damage per block past border per tick.
pub const DAMAGE_PER_BLOCK: f32 = 0.2;
/// Damage buffer (no damage for 5 blocks past border).
pub const DAMAGE_BUFFER: f64 = 5.0;
/// Warning distance (15 blocks).
pub const WARNING_DISTANCE: u32 = 15;
/// Warning time (15 seconds).
pub const WARNING_TIME: u32 = 15 * 20;

/// Compute damage for player past border.
pub fn border_damage(distance_past_border: f64) -> f32 {
    let past = (distance_past_border - DAMAGE_BUFFER).max(0.0);
    (past * DAMAGE_PER_BLOCK as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_buffer_no_damage() {
        assert_eq!(border_damage(3.0), 0.0);
    }

    #[test]
    fn past_border_scales() {
        let small = border_damage(10.0);
        let large = border_damage(100.0);
        assert!(large > small);
    }
}
