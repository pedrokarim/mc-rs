//! Villager — full model (profession, trades, level, reputation).

use crate::trading::VillagerProfession;

#[derive(Debug, Clone)]
pub struct Villager {
    pub profession: VillagerProfession,
    pub level: u8, // 1-5
    pub experience: u32,
    pub is_baby: bool,
    pub has_job_site: Option<(i32, i32, i32)>,
    pub bed_position: Option<(i32, i32, i32)>,
    pub restock_cooldown: u32,
    pub trades_used_today: u32,
    pub age: i32,
}

/// XP needed per level: 10, 70, 150, 250.
pub fn xp_to_next_level(current_level: u8) -> u32 {
    match current_level {
        1 => 10,
        2 => 70,
        3 => 150,
        4 => 250,
        _ => u32::MAX,
    }
}

/// Villager restocks once per day (24000 ticks).
pub const RESTOCK_INTERVAL: u32 = 24_000;
/// Max restocks per day (2).
pub const MAX_RESTOCKS_PER_DAY: u32 = 2;

impl Villager {
    pub fn new(profession: VillagerProfession) -> Self {
        Self {
            profession,
            level: 1,
            experience: 0,
            is_baby: false,
            has_job_site: None,
            bed_position: None,
            restock_cooldown: 0,
            trades_used_today: 0,
            age: 0,
        }
    }

    pub fn add_xp(&mut self, amount: u32) {
        self.experience += amount;
        while self.experience >= xp_to_next_level(self.level) && self.level < 5 {
            self.experience -= xp_to_next_level(self.level);
            self.level += 1;
        }
    }

    pub fn can_trade(&self) -> bool {
        self.trades_used_today < 12 // Vanilla cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_up_after_10_xp() {
        let mut v = Villager::new(VillagerProfession::Farmer);
        v.add_xp(15);
        assert_eq!(v.level, 2);
    }
}
