//! Pillager patrols — spawn périodique + raid triggering.

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Patrol {
    pub leader_runtime_id: u64,
    pub members: Vec<u64>,
    pub target_village: Option<[i32; 3]>,
    pub is_leader_carrying_ominous_banner: bool,
}

impl Patrol {
    pub fn new(leader_runtime_id: u64) -> Self {
        Self {
            leader_runtime_id,
            members: vec![leader_runtime_id],
            target_village: None,
            is_leader_carrying_ominous_banner: true,
        }
    }

    pub fn add_member(&mut self, runtime_id: u64) {
        if !self.members.contains(&runtime_id) {
            self.members.push(runtime_id);
        }
    }

    pub fn remove_member(&mut self, runtime_id: u64) {
        self.members.retain(|id| *id != runtime_id);
    }

    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Chance de spawn d'une patrol random par chunk tick.
pub const PATROL_SPAWN_CHANCE: f32 = 0.006; // 0.6%

/// Taille d'une patrol : 3-5 pillagers + 0-2 vindicators.
pub fn random_patrol_size() -> (u32, u32) {
    let mut rng = rand::thread_rng();
    (rng.gen_range(3..=5), rng.gen_range(0..=2))
}

/// Minute intervalle entre patrols dans un chunk. Matches vanilla behavior.
pub const PATROL_CHECK_INTERVAL_TICKS: u32 = 20 * 60; // 60s

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patrol_starts_with_leader() {
        let p = Patrol::new(42);
        assert_eq!(p.size(), 1);
        assert_eq!(p.leader_runtime_id, 42);
    }

    #[test]
    fn add_member_deduplicated() {
        let mut p = Patrol::new(1);
        p.add_member(2);
        p.add_member(2);
        assert_eq!(p.size(), 2);
    }
}
