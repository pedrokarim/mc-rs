//! Weather damage — lightning strikes, rain damage to endermen, drying soul_sand, etc.

use rand::Rng;

pub const LIGHTNING_DAMAGE: f32 = 5.0;
pub const LIGHTNING_RANGE: f32 = 3.0;
pub const LIGHTNING_FIRE_TICKS: u32 = 160; // 8s

/// Chance par game tick de foudre durant thunderstorm (per loaded chunk).
pub const LIGHTNING_CHANCE_PER_CHUNK: f32 = 1.0 / 100_000.0;

pub fn lightning_can_strike(in_thunderstorm: bool) -> bool {
    in_thunderstorm
}

pub fn random_lightning_strike(in_thunderstorm: bool, loaded_chunks: u32) -> bool {
    if !in_thunderstorm || loaded_chunks == 0 {
        return false;
    }
    let chance = LIGHTNING_CHANCE_PER_CHUNK * loaded_chunks as f32;
    rand::thread_rng().gen::<f32>() < chance
}

/// Mobs que le rain peut damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainSensitive {
    Enderman,
    Blaze,
    Snowman,
    MagmaCube,
}

impl RainSensitive {
    pub fn rain_damage_per_tick(&self) -> f32 {
        match self {
            Self::Snowman => 0.5, // melts in rain
            Self::Enderman | Self::Blaze => 0.2,
            Self::MagmaCube => 0.0, // not damaged actually
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enderman_hates_rain() {
        assert!(RainSensitive::Enderman.rain_damage_per_tick() > 0.0);
    }

    #[test]
    fn no_lightning_without_thunder() {
        assert!(!random_lightning_strike(false, 1000));
    }
}
