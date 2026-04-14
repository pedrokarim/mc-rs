//! Tadpole — baby frog in water.

#[derive(Debug, Clone)]
pub struct Tadpole {
    pub age: i32,
    pub water_temperature: f32, // biome temp determines frog variant on grow
    pub ticks_out_of_water: u32,
}

/// Grow into frog after 24,000 ticks.
pub const GROW_TIME: u32 = 24000;
/// Max time out of water (5 min = 6000 ticks).
pub const MAX_OUT_OF_WATER: u32 = 6000;

impl Tadpole {
    pub fn new(water_temp: f32) -> Self {
        Self {
            age: 0,
            water_temperature: water_temp,
            ticks_out_of_water: 0,
        }
    }

    pub fn tick(&mut self, in_water: bool) -> TadpoleEvent {
        self.age += 1;
        if in_water {
            self.ticks_out_of_water = 0;
        } else {
            self.ticks_out_of_water += 1;
            if self.ticks_out_of_water >= MAX_OUT_OF_WATER {
                return TadpoleEvent::Dying;
            }
        }
        if self.age >= GROW_TIME as i32 {
            return TadpoleEvent::GrowIntoFrog;
        }
        TadpoleEvent::Tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TadpoleEvent {
    Tick,
    GrowIntoFrog,
    Dying,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dies_out_of_water() {
        let mut t = Tadpole::new(0.7);
        t.ticks_out_of_water = MAX_OUT_OF_WATER;
        assert_eq!(t.tick(false), TadpoleEvent::Dying);
    }
}
