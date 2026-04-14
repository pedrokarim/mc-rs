//! Turtle — breed avec seagrass, lay eggs sur sand près home beach, drops scute.

#[derive(Debug, Clone)]
pub struct Turtle {
    pub age: i32,
    pub home_position: (i32, i32, i32),
    pub has_egg: bool,
    pub laying_egg: bool,
    pub travel_pos: Option<(i32, i32, i32)>,
    pub went_home: bool,
}

/// Breeding item = seagrass.
pub const BREEDING_ITEM: u16 = 385;
/// Turtles drop 0-1 scute when baby becomes adult.
pub const SCUTE_DROP: u16 = 570;
/// Turtle egg hatch takes up to 3 nights.
pub const HATCH_NIGHTS_MAX: u32 = 3;

impl Turtle {
    pub fn new(home: (i32, i32, i32)) -> Self {
        Self {
            age: 0,
            home_position: home,
            has_egg: false,
            laying_egg: false,
            travel_pos: None,
            went_home: true,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
    }

    pub fn impregnate(&mut self) {
        self.has_egg = true;
        self.went_home = false;
        self.travel_pos = Some(self.home_position);
    }

    pub fn lay_eggs(&mut self) -> bool {
        if !self.has_egg {
            return false;
        }
        self.has_egg = false;
        self.laying_egg = true;
        self.went_home = true;
        true
    }

    /// Eggs take up to 3 nights to hatch.
    pub fn egg_hatch_night_chance() -> f32 {
        0.33
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lay_requires_egg() {
        let mut t = Turtle::new((0, 0, 0));
        assert!(!t.lay_eggs());
        t.impregnate();
        assert!(t.lay_eggs());
    }
}
