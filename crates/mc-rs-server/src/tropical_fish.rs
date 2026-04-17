//! Tropical fish — color + variant pairs (3,072 possible combinations).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalFishPattern {
    Kob,
    Sunstreak,
    Snooper,
    Dasher,
    Brinely,
    Spotty,
    Flopper,
    Stripey,
    Glitter,
    Blockfish,
    Betty,
    Clayfish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TropicalFishShape {
    Small,
    Large,
}

#[derive(Debug, Clone)]
pub struct TropicalFish {
    pub shape: TropicalFishShape,
    pub pattern: TropicalFishPattern,
    pub body_color: u8, // dye id 0-15
    pub pattern_color: u8,
}

/// Bucket captured fish — retains variant.
pub const CAN_BUCKET: bool = true;

impl TropicalFish {
    pub fn new_random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let shapes = [TropicalFishShape::Small, TropicalFishShape::Large];
        let patterns = [
            TropicalFishPattern::Kob,
            TropicalFishPattern::Sunstreak,
            TropicalFishPattern::Snooper,
            TropicalFishPattern::Dasher,
            TropicalFishPattern::Brinely,
            TropicalFishPattern::Spotty,
            TropicalFishPattern::Flopper,
            TropicalFishPattern::Stripey,
            TropicalFishPattern::Glitter,
            TropicalFishPattern::Blockfish,
            TropicalFishPattern::Betty,
            TropicalFishPattern::Clayfish,
        ];
        Self {
            shape: shapes[rng.gen_range(0..shapes.len())],
            pattern: patterns[rng.gen_range(0..patterns.len())],
            body_color: rng.gen_range(0..16),
            pattern_color: rng.gen_range(0..16),
        }
    }

    /// Damage to attacker.
    pub fn attack_damage() -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_fish_creation() {
        let _ = TropicalFish::new_random();
    }
}
