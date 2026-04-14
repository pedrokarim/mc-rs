//! Snow Golem — defender, throws snowballs.

#[derive(Debug, Clone)]
pub struct SnowGolem {
    pub has_pumpkin: bool,   // Can be sheared off
    pub attack_cooldown: u32,
}

/// Attacks blaze / hostile mobs.
pub const DAMAGE: f32 = 0.0; // 0-4 depending on target (blaze 3)
/// Attack cooldown.
pub const ATTACK_COOLDOWN: u32 = 20;
/// Snowball damage to blaze.
pub const SNOWBALL_BLAZE_DAMAGE: f32 = 3.0;

impl SnowGolem {
    pub fn new() -> Self {
        Self { has_pumpkin: true, attack_cooldown: 0 }
    }

    pub fn shear(&mut self) -> Option<&'static str> {
        if !self.has_pumpkin {
            return None;
        }
        self.has_pumpkin = false;
        Some("minecraft:carved_pumpkin")
    }

    /// Damaged by water/rain/desert biomes/nether.
    pub fn damaged_by_water() -> bool { true }
    pub fn damaged_in_desert() -> bool { true }
    pub fn damaged_in_nether() -> bool { true }

    /// Leaves trail of snow layers.
    pub fn leaves_snow_trail(&self) -> bool {
        // Only in cold biomes (vanilla). simplified:
        true
    }
}

impl Default for SnowGolem {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shearing_once() {
        let mut g = SnowGolem::new();
        assert!(g.shear().is_some());
        assert!(g.shear().is_none());
    }
}
