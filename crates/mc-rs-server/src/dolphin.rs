//! Dolphin — donne Dolphin's Grace effect au joueur nageant à proximité.
//! Lead les joueurs vers treasures (avec fish gift).

#[derive(Debug, Clone)]
pub struct Dolphin {
    pub air_ticks: u32,
    pub happiness_ticks: u32,
    pub treasure_position: Option<(i32, i32, i32)>,
    pub has_been_fed: bool,
}

/// Max air (can stay out of water longer).
pub const MAX_AIR_TICKS: u32 = 4800; // 4 min
/// Happy duration after fish feeding.
pub const HAPPY_DURATION: u32 = 2400; // 2 min
/// Grace range (grants speed in water near dolphin).
pub const GRACE_RANGE: f64 = 10.0;

impl Dolphin {
    pub fn new() -> Self {
        Self {
            air_ticks: MAX_AIR_TICKS,
            happiness_ticks: 0,
            treasure_position: None,
            has_been_fed: false,
        }
    }

    pub fn tick(&mut self, in_water: bool) {
        if in_water {
            self.air_ticks = MAX_AIR_TICKS;
        } else if self.air_ticks > 0 {
            self.air_ticks -= 1;
        }
        if self.happiness_ticks > 0 {
            self.happiness_ticks -= 1;
        }
    }

    pub fn feed_fish(&mut self, treasure: Option<(i32, i32, i32)>) {
        self.happiness_ticks = HAPPY_DURATION;
        self.has_been_fed = true;
        self.treasure_position = treasure;
    }

    pub fn is_happy(&self) -> bool {
        self.happiness_ticks > 0
    }

    pub fn should_drown(&self) -> bool {
        self.air_ticks == 0
    }
}

impl Default for Dolphin {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refills_air_in_water() {
        let mut d = Dolphin::new();
        d.air_ticks = 10;
        d.tick(true);
        assert_eq!(d.air_ticks, MAX_AIR_TICKS);
    }

    #[test]
    fn feeding_makes_happy() {
        let mut d = Dolphin::new();
        d.feed_fish(Some((100, 50, 100)));
        assert!(d.is_happy());
    }
}
