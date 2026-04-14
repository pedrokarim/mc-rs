//! Broadcast velocity updates.

/// Velocity change triggers broadcast if above threshold.
pub const VELOCITY_CHANGE_THRESHOLD: f64 = 0.01;

/// Max velocity per axis (to avoid physics breaking).
pub const MAX_VELOCITY_PER_AXIS: f64 = 3.9;

/// Time between forced broadcasts (10 ticks).
pub const FORCED_BROADCAST_INTERVAL: u32 = 10;

pub fn clamp_velocity(v: f64) -> f64 {
    v.clamp(-MAX_VELOCITY_PER_AXIS, MAX_VELOCITY_PER_AXIS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_extreme_values() {
        assert_eq!(clamp_velocity(100.0), MAX_VELOCITY_PER_AXIS);
    }
}
