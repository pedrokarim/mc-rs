//! Sea pickle — 1-4 per block, glow in water.

#[derive(Debug, Clone)]
pub struct SeaPickle {
    pub count: u8, // 1-4
    pub in_water: bool,
}

/// Max pickles.
pub const MAX_COUNT: u8 = 4;
/// Light per pickle when in water.
pub fn light_emission(count: u8, in_water: bool) -> u8 {
    if !in_water {
        return 0;
    }
    3 + 3 * (count.saturating_sub(1))
}

impl SeaPickle {
    pub fn new(count: u8) -> Self {
        Self {
            count: count.clamp(1, MAX_COUNT),
            in_water: false,
        }
    }

    pub fn add(&mut self) -> bool {
        if self.count >= MAX_COUNT {
            return false;
        }
        self.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_scales_with_count() {
        assert!(light_emission(4, true) > light_emission(1, true));
    }

    #[test]
    fn no_light_out_of_water() {
        assert_eq!(light_emission(4, false), 0);
    }
}
