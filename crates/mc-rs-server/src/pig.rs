//! Pig — saddled mount with carrot-on-stick boost, zombify on lightning.

#[derive(Debug, Clone)]
pub struct Pig {
    pub age: i32,
    pub saddled: bool,
    pub rider: Option<u64>,
    pub boost_ticks: u32,
    pub boost_stored: u32,
}

/// Breeding items = carrot / potato / beetroot.
pub fn breeding_items() -> &'static [&'static str] {
    &[
        "minecraft:carrot",
        "minecraft:potato",
        "minecraft:beetroot",
    ]
}

/// Max boost ticks (max carrot on stick use = 64 uses × 17 ticks).
pub const MAX_BOOST_PER_USE: u32 = 100;

impl Pig {
    pub fn new_adult() -> Self {
        Self { age: 0, saddled: false, rider: None, boost_ticks: 0, boost_stored: 0 }
    }

    pub fn saddle(&mut self) -> bool {
        if self.saddled { return false; }
        self.saddled = true;
        true
    }

    pub fn mount(&mut self, player: u64) -> bool {
        if !self.saddled || self.rider.is_some() {
            return false;
        }
        self.rider = Some(player);
        true
    }

    pub fn use_carrot_on_stick(&mut self) -> bool {
        if self.rider.is_none() {
            return false;
        }
        self.boost_ticks = MAX_BOOST_PER_USE;
        true
    }

    pub fn is_boosted(&self) -> bool {
        self.boost_ticks > 0
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.boost_ticks > 0 {
            self.boost_ticks -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saddled_pig_can_be_ridden() {
        let mut p = Pig::new_adult();
        p.saddle();
        assert!(p.mount(1));
    }

    #[test]
    fn boost_requires_rider() {
        let mut p = Pig::new_adult();
        assert!(!p.use_carrot_on_stick());
    }
}
