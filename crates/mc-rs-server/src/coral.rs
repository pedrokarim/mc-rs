//! Coral — live / dead variants, 5 colors.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoralColor {
    Tube,   // Blue
    Brain,  // Pink
    Bubble, // Purple
    Fire,   // Red
    Horn,   // Yellow
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoralForm {
    Block,
    Coral,      // Single coral
    CoralFan,   // Ground-mounted
    CoralWall,  // Wall-mounted fan
}

#[derive(Debug, Clone)]
pub struct Coral {
    pub color: CoralColor,
    pub form: CoralForm,
    pub alive: bool,
    pub ticks_out_of_water: u32,
}

/// Coral dies if out of water > ~20 ticks.
pub const DEATH_TICKS_OUT_OF_WATER: u32 = 100;

impl Coral {
    pub fn new(color: CoralColor, form: CoralForm, alive: bool) -> Self {
        Self { color, form, alive, ticks_out_of_water: 0 }
    }

    pub fn tick(&mut self, in_water: bool) {
        if in_water {
            self.ticks_out_of_water = 0;
        } else if self.alive {
            self.ticks_out_of_water += 1;
            if self.ticks_out_of_water >= DEATH_TICKS_OUT_OF_WATER {
                self.alive = false;
            }
        }
    }

    /// Only live corals drop themselves with silk touch.
    pub fn requires_silk_touch(&self) -> bool {
        self.alive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dies_out_of_water() {
        let mut c = Coral::new(CoralColor::Tube, CoralForm::Block, true);
        for _ in 0..=DEATH_TICKS_OUT_OF_WATER {
            c.tick(false);
        }
        assert!(!c.alive);
    }

    #[test]
    fn stays_alive_in_water() {
        let mut c = Coral::new(CoralColor::Tube, CoralForm::Block, true);
        for _ in 0..1000 {
            c.tick(true);
        }
        assert!(c.alive);
    }
}
