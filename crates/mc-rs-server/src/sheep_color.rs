//! Sheep color mechanics — port PMMP `src/entity/Sheep.php`.

use crate::dyes::DyeColor;
use rand::Rng;

/// Chance d'apparition de chaque couleur en nature (PMMP/Bedrock vanilla).
pub fn natural_sheep_color() -> DyeColor {
    let mut rng = rand::thread_rng();
    let r = rng.gen_range(0..1000);
    if r < 8 {
        DyeColor::Black
    } else if r < 18 {
        DyeColor::LightGray
    } else if r < 28 {
        DyeColor::Gray
    } else if r < 29 {
        DyeColor::Pink
    } else if r < 34 {
        DyeColor::Brown
    } else {
        DyeColor::White
    }
}

/// Sheared state : 4 minutes de cooldown avant re-grow wool.
pub const WOOL_REGROWTH_TICKS: u32 = 4 * 60 * 20; // 4800 ticks

#[derive(Debug, Clone)]
pub struct SheepState {
    pub color: DyeColor,
    pub sheared: bool,
    pub wool_regrow_timer: u32,
}

impl SheepState {
    pub fn new(color: DyeColor) -> Self {
        Self {
            color,
            sheared: false,
            wool_regrow_timer: 0,
        }
    }

    pub fn shear(&mut self) -> bool {
        if self.sheared {
            return false;
        }
        self.sheared = true;
        self.wool_regrow_timer = WOOL_REGROWTH_TICKS;
        true
    }

    pub fn tick(&mut self) {
        if self.sheared && self.wool_regrow_timer > 0 {
            self.wool_regrow_timer -= 1;
            if self.wool_regrow_timer == 0 {
                self.sheared = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shear_starts_cooldown() {
        let mut s = SheepState::new(DyeColor::White);
        assert!(s.shear());
        assert!(s.sheared);
        assert_eq!(s.wool_regrow_timer, WOOL_REGROWTH_TICKS);
    }

    #[test]
    fn cannot_shear_twice() {
        let mut s = SheepState::new(DyeColor::White);
        s.shear();
        assert!(!s.shear());
    }
}
