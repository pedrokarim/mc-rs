//! Sheep — wool color, shearing, grass eating.

use crate::dyes::DyeColor as SheepColor;

#[derive(Debug, Clone)]
pub struct Sheep {
    pub color: SheepColor,
    pub age: i32,
    pub shorn: bool,
    pub regrow_ticks: u32,
}

/// Regrow wool chance per random tick if on grass (100% in 1-5 min eating).
pub const EAT_GRASS_CHANCE: f32 = 0.04;
/// Base wool drops.
pub const WOOL_DROP_MIN: u32 = 1;
pub const WOOL_DROP_MAX: u32 = 3;

impl Sheep {
    pub fn new(color: SheepColor) -> Self {
        Self {
            color,
            age: 0,
            shorn: false,
            regrow_ticks: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
    }

    pub fn shear(&mut self) -> Option<(SheepColor, u32)> {
        if self.shorn {
            return None;
        }
        self.shorn = true;
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(WOOL_DROP_MIN..=WOOL_DROP_MAX);
        Some((self.color, count))
    }

    pub fn eat_grass(&mut self) {
        self.shorn = false;
    }

    pub fn dye(&mut self, color: SheepColor) {
        if !self.shorn {
            self.color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shearing_drops_wool() {
        let mut s = Sheep::new(SheepColor::White);
        assert!(s.shear().is_some());
        assert!(s.shear().is_none());
    }

    #[test]
    fn grass_regrows() {
        let mut s = Sheep::new(SheepColor::White);
        s.shorn = true;
        s.eat_grass();
        assert!(!s.shorn);
    }
}
