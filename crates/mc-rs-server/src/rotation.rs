//! Entity rotation / yaw/pitch normalization.

/// Normalize yaw to -180 to 180.
pub fn wrap_degrees(yaw: f32) -> f32 {
    let y = yaw.rem_euclid(360.0);
    if y > 180.0 {
        y - 360.0
    } else if y < -180.0 {
        y + 360.0
    } else {
        y
    }
}

/// Clamp pitch to -90..90.
pub fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(-90.0, 90.0)
}

/// Smooth interpolation between yaws (shortest path).
pub fn smooth_yaw(from: f32, to: f32, factor: f32) -> f32 {
    let diff = wrap_degrees(to - from);
    from + diff * factor
}

/// Face from yaw: 0 = south, 90 = west, 180 = north, 270 = east (vanilla Java).
pub fn yaw_to_cardinal(yaw: f32) -> &'static str {
    let n = wrap_degrees(yaw);
    if n > -45.0 && n < 45.0 {
        "south"
    } else if n >= 45.0 && n < 135.0 {
        "west"
    } else if n >= -135.0 && n < -45.0 {
        "east"
    } else {
        "north"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_handles_large_values() {
        assert!(wrap_degrees(720.0).abs() < 0.001);
    }

    #[test]
    fn clamp_pitch_bounds() {
        assert_eq!(clamp_pitch(100.0), 90.0);
        assert_eq!(clamp_pitch(-200.0), -90.0);
    }
}
