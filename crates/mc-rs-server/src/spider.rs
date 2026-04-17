//! Spider + CaveSpider — climbing mob.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiderVariant {
    Normal, // HP 16, damage 2
    Cave,   // HP 12, damage 2, poison
}

#[derive(Debug, Clone)]
pub struct Spider {
    pub variant: SpiderVariant,
    pub climbing_wall: bool,
    pub hostile_threshold_light: u8, // Spiders become passive in bright light
}

/// Light level threshold (>= 11 = passive).
pub const PASSIVE_LIGHT_LEVEL: u8 = 11;
/// Cave spider damage adds poison on hit (Normal+ difficulty).
pub const CAVE_SPIDER_POISON_DURATION: u32 = 7 * 20; // 7s

impl Spider {
    pub fn new(variant: SpiderVariant) -> Self {
        Self {
            variant,
            climbing_wall: false,
            hostile_threshold_light: PASSIVE_LIGHT_LEVEL,
        }
    }

    /// Passive when light > threshold.
    pub fn is_passive(&self, light_level: u8) -> bool {
        light_level >= self.hostile_threshold_light
    }

    pub fn applies_poison(&self) -> bool {
        self.variant == SpiderVariant::Cave
    }

    pub fn hp(&self) -> f32 {
        match self.variant {
            SpiderVariant::Normal => 16.0,
            SpiderVariant::Cave => 12.0,
        }
    }

    /// Spider can fit in 0.7x0.5 tiles, cave spider in 0.5x0.5.
    pub fn hitbox(&self) -> (f32, f32) {
        match self.variant {
            SpiderVariant::Normal => (1.4, 0.9),
            SpiderVariant::Cave => (0.7, 0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bright_light_makes_passive() {
        let s = Spider::new(SpiderVariant::Normal);
        assert!(s.is_passive(12));
        assert!(!s.is_passive(5));
    }

    #[test]
    fn cave_poisons() {
        assert!(Spider::new(SpiderVariant::Cave).applies_poison());
        assert!(!Spider::new(SpiderVariant::Normal).applies_poison());
    }
}
