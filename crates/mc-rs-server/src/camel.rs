//! Camel — mob grande qui dash + 2 joueurs peuvent monter.

#[derive(Debug, Clone)]
pub struct Camel {
    pub age: i32,
    pub sitting: bool,
    pub dashing: bool,
    pub dash_cooldown: u32,
    pub last_dash_tick: u64,
    pub riders: Vec<u64>, // up to 2 players
    pub saddled: bool,
}

/// Camel can carry 2 riders.
pub const MAX_RIDERS: usize = 2;
/// Dash cooldown (~45 ticks).
pub const DASH_COOLDOWN: u32 = 45;
/// Breeding item = cactus.
pub const BREEDING_ITEM: u16 = 81;

impl Camel {
    pub fn new_adult() -> Self {
        Self {
            age: 0,
            sitting: false,
            dashing: false,
            dash_cooldown: 0,
            last_dash_tick: 0,
            riders: Vec::with_capacity(MAX_RIDERS),
            saddled: false,
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.dash_cooldown > 0 {
            self.dash_cooldown -= 1;
        } else {
            self.dashing = false;
        }
    }

    pub fn add_rider(&mut self, player_id: u64) -> bool {
        if self.riders.len() >= MAX_RIDERS {
            return false;
        }
        if !self.saddled && self.riders.is_empty() {
            return false;
        }
        self.riders.push(player_id);
        self.sitting = false;
        true
    }

    pub fn remove_rider(&mut self, player_id: u64) {
        self.riders.retain(|&p| p != player_id);
    }

    pub fn sit(&mut self) {
        self.sitting = true;
    }

    pub fn stand(&mut self) {
        self.sitting = false;
    }

    pub fn try_dash(&mut self) -> bool {
        if self.dash_cooldown > 0 || self.sitting {
            return false;
        }
        self.dashing = true;
        self.dash_cooldown = DASH_COOLDOWN;
        true
    }

    pub fn can_be_ridden(&self) -> bool {
        self.saddled && !self.sitting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_riders_max() {
        let mut c = Camel::new_adult();
        c.saddled = true;
        assert!(c.add_rider(1));
        assert!(c.add_rider(2));
        assert!(!c.add_rider(3));
    }

    #[test]
    fn dash_then_cooldown() {
        let mut c = Camel::new_adult();
        assert!(c.try_dash());
        assert!(!c.try_dash());
    }

    #[test]
    fn sit_blocks_dash() {
        let mut c = Camel::new_adult();
        c.sit();
        assert!(!c.try_dash());
    }
}
