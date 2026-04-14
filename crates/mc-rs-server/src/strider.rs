//! Strider — mob qui peut marcher sur la lava. Peut être monté avec warped fungus on stick.

#[derive(Debug, Clone)]
pub struct Strider {
    pub age: i32,
    pub shivering: bool,
    pub saddled: bool,
    pub rider: Option<u64>,
}

/// Breeding item = warped fungus.
pub const BREEDING_ITEM: u16 = 262;
/// Control item quand saddled = warped fungus on stick.
pub const CONTROL_ITEM: u16 = 263;
/// Temperature threshold pour shivering (strider shiver hors de lava).
pub const SHIVER_DISTANCE_FROM_LAVA: f64 = 2.0;

impl Strider {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            shivering: false,
            saddled: false,
            rider: None,
        }
    }

    pub fn new_baby() -> Self {
        Self {
            age: -24000,
            shivering: false,
            saddled: false,
            rider: None,
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    pub fn tick(&mut self, on_lava: bool) {
        if self.age < 0 {
            self.age += 1;
        }
        self.shivering = !on_lava;
    }

    pub fn saddle(&mut self) -> bool {
        if self.saddled {
            return false;
        }
        self.saddled = true;
        true
    }

    pub fn mount(&mut self, player_id: u64) -> bool {
        if !self.saddled || self.rider.is_some() {
            return false;
        }
        self.rider = Some(player_id);
        true
    }

    pub fn dismount(&mut self) -> Option<u64> {
        self.rider.take()
    }

    pub fn can_be_controlled(&self) -> bool {
        self.saddled && self.rider.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shivers_off_lava() {
        let mut s = Strider::new_adult();
        s.tick(false);
        assert!(s.shivering);
    }

    #[test]
    fn lava_calm() {
        let mut s = Strider::new_adult();
        s.tick(true);
        assert!(!s.shivering);
    }

    #[test]
    fn saddled_once() {
        let mut s = Strider::new_adult();
        assert!(s.saddle());
        assert!(!s.saddle());
    }

    #[test]
    fn mount_needs_saddle() {
        let mut s = Strider::new_adult();
        assert!(!s.mount(1));
        s.saddle();
        assert!(s.mount(1));
    }
}
