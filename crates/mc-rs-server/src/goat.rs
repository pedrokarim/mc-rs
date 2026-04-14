//! Goat — mob qui ram + drop horn.

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Goat {
    pub age: i32,
    pub screaming: bool,
    pub horns: (bool, bool), // (left, right)
    pub ram_cooldown: u32,
    pub ram_target: Option<u64>,
}

/// Ram cooldown (30-300 ticks).
pub const RAM_COOLDOWN_MIN: u32 = 30;
pub const RAM_COOLDOWN_MAX: u32 = 300;
/// Screaming goat chance (2% vanilla).
pub const SCREAMING_CHANCE: f32 = 0.02;
/// Breeding item = wheat.
pub const BREEDING_ITEM: u16 = 296;

impl Goat {
    pub fn new(screaming: bool) -> Self {
        Self {
            age: 0,
            screaming,
            horns: (true, true),
            ram_cooldown: 0,
            ram_target: None,
        }
    }

    pub fn roll_screaming() -> bool {
        let mut rng = rand::thread_rng();
        rng.gen::<f32>() < SCREAMING_CHANCE
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.ram_cooldown > 0 {
            self.ram_cooldown -= 1;
        }
    }

    pub fn start_ram(&mut self, target: u64) {
        let mut rng = rand::thread_rng();
        self.ram_cooldown = rng.gen_range(RAM_COOLDOWN_MIN..=RAM_COOLDOWN_MAX);
        self.ram_target = Some(target);
    }

    /// Drop a horn when colliding with stone block.
    pub fn drop_horn(&mut self) -> Option<&'static str> {
        if self.horns.0 {
            self.horns.0 = false;
            return Some(self.horn_variant());
        }
        if self.horns.1 {
            self.horns.1 = false;
            return Some(self.horn_variant());
        }
        None
    }

    fn horn_variant(&self) -> &'static str {
        if self.screaming {
            let variants = ["sing", "admire", "seek", "feel"];
            variants[rand::thread_rng().gen_range(0..variants.len())]
        } else {
            let variants = ["ponder", "yearn", "resist", "call"];
            variants[rand::thread_rng().gen_range(0..variants.len())]
        }
    }

    pub fn can_ram(&self) -> bool {
        self.ram_cooldown == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_up_to_two_horns() {
        let mut g = Goat::new(false);
        assert!(g.drop_horn().is_some());
        assert!(g.drop_horn().is_some());
        assert!(g.drop_horn().is_none());
    }

    #[test]
    fn ram_puts_on_cooldown() {
        let mut g = Goat::new(false);
        assert!(g.can_ram());
        g.start_ram(1);
        assert!(!g.can_ram());
    }
}
