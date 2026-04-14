//! Player statistics tracking (stats.json-like).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    pub blocks_broken: HashMap<String, u64>,
    pub blocks_placed: HashMap<String, u64>,
    pub items_crafted: HashMap<String, u64>,
    pub items_used: HashMap<String, u64>,
    pub items_picked_up: HashMap<String, u64>,
    pub items_dropped: HashMap<String, u64>,
    pub mobs_killed: HashMap<String, u64>,
    pub killed_by: HashMap<String, u64>,
    pub custom: HashMap<String, u64>, // jump, play_time, walk_cm, etc.
}

pub mod custom_keys {
    pub const JUMP: &str = "custom.minecraft.jump";
    pub const PLAY_TIME: &str = "custom.minecraft.play_time";
    pub const WALK_CM: &str = "custom.minecraft.walk_one_cm";
    pub const SPRINT_CM: &str = "custom.minecraft.sprint_one_cm";
    pub const FLY_CM: &str = "custom.minecraft.fly_one_cm";
    pub const DEATHS: &str = "custom.minecraft.deaths";
    pub const DAMAGE_DEALT: &str = "custom.minecraft.damage_dealt";
    pub const DAMAGE_TAKEN: &str = "custom.minecraft.damage_taken";
    pub const TIME_SINCE_DEATH: &str = "custom.minecraft.time_since_death";
    pub const BOAT_CM: &str = "custom.minecraft.boat_one_cm";
    pub const HORSE_CM: &str = "custom.minecraft.horse_one_cm";
    pub const SWIM_CM: &str = "custom.minecraft.swim_one_cm";
    pub const SLEEP: &str = "custom.minecraft.sleep_in_bed";
}

impl PlayerStats {
    pub fn inc_custom(&mut self, key: &str, amount: u64) {
        *self.custom.entry(key.to_string()).or_insert(0) += amount;
    }

    pub fn inc_block_broken(&mut self, block: &str) {
        *self.blocks_broken.entry(block.to_string()).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_block_broken() {
        let mut s = PlayerStats::default();
        s.inc_block_broken("minecraft:stone");
        assert_eq!(s.blocks_broken.get("minecraft:stone"), Some(&1));
    }
}
