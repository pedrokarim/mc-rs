//! Fireworks — port PMMP `src/entity/object/FireworkRocket.php` + `FireworkExplosion`.

use crate::banner::BannerColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireworkShape {
    SmallBall = 0,
    LargeBall = 1,
    Star = 2,
    Creeper = 3,
    Burst = 4,
}

#[derive(Debug, Clone)]
pub struct FireworkExplosion {
    pub shape: FireworkShape,
    pub has_trail: bool,
    pub has_twinkle: bool,
    pub colors: Vec<BannerColor>,
    pub fade_colors: Vec<BannerColor>,
}

#[derive(Debug, Clone)]
pub struct Firework {
    /// Flight duration : 0=1 gunpowder, 1=2, 2=3.
    pub flight_duration: u8,
    pub explosions: Vec<FireworkExplosion>,
}

impl Firework {
    pub fn new(flight_duration: u8) -> Self {
        Self {
            flight_duration,
            explosions: Vec::new(),
        }
    }

    /// Ticks de vol (vanilla : ~10 + 10 * flight_duration).
    pub fn flight_ticks(&self) -> u32 {
        10 + 10 * self.flight_duration as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flight_ticks_matches_gunpowder() {
        let f = Firework::new(0);
        assert_eq!(f.flight_ticks(), 10);
        let f = Firework::new(2);
        assert_eq!(f.flight_ticks(), 30);
    }
}
