//! Daylight sensor — outputs redstone based on sky light.

#[derive(Debug, Clone, Copy)]
pub struct DaylightSensor {
    pub inverted: bool, // flipped mode = output at night
}

/// Compute redstone strength based on time of day.
pub fn output_strength(time_of_day: u64, inverted: bool) -> u8 {
    let t = (time_of_day % 24000) as f64;
    // Raw light: peaks at noon (6000), 0 at midnight (18000).
    let normalized = ((t - 6000.0).abs() / 12000.0).min(1.0);
    let raw = ((1.0 - normalized) * 15.0).round() as u8;
    if inverted {
        15 - raw
    } else {
        raw
    }
}

impl DaylightSensor {
    pub fn new() -> Self {
        Self { inverted: false }
    }

    pub fn toggle_inverted(&mut self) {
        self.inverted = !self.inverted;
    }
}

impl Default for DaylightSensor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noon_max_light() {
        assert!(output_strength(6000, false) >= 14);
    }

    #[test]
    fn midnight_zero() {
        assert_eq!(output_strength(18000, false), 0);
    }

    #[test]
    fn inverted_flips_output() {
        let normal = output_strength(6000, false);
        let inverted = output_strength(6000, true);
        assert_eq!(normal + inverted, 15);
    }
}
