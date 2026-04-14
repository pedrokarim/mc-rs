//! Raid system — port conceptuel. Pillager raid déclenché par Bad Omen effect
//! près d'un village. 5 waves de mobs pillager/vindicator/ravager.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidStatus {
    Ongoing,
    Victory,
    Loss,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Raid {
    pub id: u64,
    pub center: [i32; 3],
    pub current_wave: u32,
    pub total_waves: u32,
    pub status: RaidStatus,
    pub bad_omen_level: u8,
    pub spawned_mobs: Vec<u64>, // runtime IDs
    pub heroes: Vec<u64>,
}

impl Raid {
    pub fn new(id: u64, center: [i32; 3], bad_omen_level: u8) -> Self {
        let total_waves = 5 + bad_omen_level.saturating_sub(1) as u32;
        Self {
            id,
            center,
            current_wave: 0,
            total_waves,
            status: RaidStatus::Ongoing,
            bad_omen_level,
            spawned_mobs: Vec::new(),
            heroes: Vec::new(),
        }
    }

    pub fn next_wave(&mut self) -> Option<u32> {
        if self.status != RaidStatus::Ongoing {
            return None;
        }
        self.current_wave += 1;
        if self.current_wave > self.total_waves {
            self.status = RaidStatus::Victory;
            return None;
        }
        Some(self.current_wave)
    }

    /// Distribution typique par wave : pillagers + vindicators + optional ravager.
    pub fn wave_composition(&self, wave: u32) -> Vec<(&'static str, u32)> {
        match wave {
            1 => vec![("pillager", 3)],
            2 => vec![("pillager", 3), ("vindicator", 1)],
            3 => vec![("pillager", 3), ("vindicator", 2)],
            4 => vec![("pillager", 2), ("vindicator", 3), ("ravager", 1)],
            _ => vec![("pillager", 4), ("vindicator", 3), ("ravager", 1)],
        }
    }

    pub fn add_hero(&mut self, player_runtime_id: u64) {
        if !self.heroes.contains(&player_runtime_id) {
            self.heroes.push(player_runtime_id);
        }
    }
}

#[derive(Default)]
pub struct RaidManager {
    pub raids: HashMap<u64, Raid>,
    next_id: u64,
}

impl RaidManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_raid(&mut self, center: [i32; 3], bad_omen_level: u8) -> &mut Raid {
        self.next_id += 1;
        let id = self.next_id;
        let raid = Raid::new(id, center, bad_omen_level);
        self.raids.insert(id, raid);
        self.raids.get_mut(&id).unwrap()
    }

    pub fn end(&mut self, id: u64, status: RaidStatus) {
        if let Some(r) = self.raids.get_mut(&id) {
            r.status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid_5_waves_default() {
        let r = Raid::new(1, [0, 64, 0], 1);
        assert_eq!(r.total_waves, 5);
    }

    #[test]
    fn bad_omen_3_extends_waves() {
        let r = Raid::new(1, [0, 64, 0], 3);
        assert_eq!(r.total_waves, 7);
    }

    #[test]
    fn wave_progression_to_victory() {
        let mut r = Raid::new(1, [0, 64, 0], 1);
        for _ in 0..5 {
            assert!(r.next_wave().is_some());
        }
        assert!(r.next_wave().is_none());
        assert_eq!(r.status, RaidStatus::Victory);
    }

    #[test]
    fn heroes_deduplicated() {
        let mut r = Raid::new(1, [0, 64, 0], 1);
        r.add_hero(42);
        r.add_hero(42);
        r.add_hero(43);
        assert_eq!(r.heroes.len(), 2);
    }
}
