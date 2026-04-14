//! Trial Spawner — 1.21 spawner that ejects rewards.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialSpawnerState {
    Inactive,
    WaitingForPlayers,
    Active,
    WaitingForRewards,
    Cooldown,
}

#[derive(Debug, Clone)]
pub struct TrialSpawner {
    pub state: TrialSpawnerState,
    pub ominous: bool,
    pub mob_id: &'static str,
    pub waves_done: u32,
    pub max_waves: u32,
    pub current_mobs: Vec<u64>,
    pub target_mob_count: u32,
    pub players_activated: Vec<u64>,
    pub cooldown_ticks: u32,
}

/// Cooldown between activations (30 min = 36000 ticks).
pub const COOLDOWN_AFTER_COMPLETION: u32 = 36_000;
/// Mobs per player activation scale.
pub const MOBS_PER_PLAYER: u32 = 2;

impl TrialSpawner {
    pub fn new(mob_id: &'static str, ominous: bool) -> Self {
        Self {
            state: TrialSpawnerState::Inactive,
            ominous,
            mob_id,
            waves_done: 0,
            max_waves: if ominous { 4 } else { 2 },
            current_mobs: Vec::new(),
            target_mob_count: 0,
            players_activated: Vec::new(),
            cooldown_ticks: 0,
        }
    }

    pub fn activate(&mut self, player_count: u32) {
        if self.state != TrialSpawnerState::Inactive {
            return;
        }
        self.state = TrialSpawnerState::Active;
        self.target_mob_count = player_count * MOBS_PER_PLAYER;
    }

    pub fn on_mob_death(&mut self, mob_id: u64) {
        self.current_mobs.retain(|&m| m != mob_id);
        if self.current_mobs.is_empty() {
            self.waves_done += 1;
            if self.waves_done >= self.max_waves {
                self.state = TrialSpawnerState::WaitingForRewards;
            } else {
                self.state = TrialSpawnerState::Active;
            }
        }
    }

    pub fn tick(&mut self) {
        if self.state == TrialSpawnerState::Cooldown {
            if self.cooldown_ticks > 0 {
                self.cooldown_ticks -= 1;
            } else {
                self.state = TrialSpawnerState::Inactive;
                self.waves_done = 0;
                self.players_activated.clear();
            }
        }
    }

    pub fn finish_trial(&mut self) {
        self.state = TrialSpawnerState::Cooldown;
        self.cooldown_ticks = COOLDOWN_AFTER_COMPLETION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ominous_has_more_waves() {
        let t = TrialSpawner::new("zombie", true);
        let n = TrialSpawner::new("zombie", false);
        assert!(t.max_waves > n.max_waves);
    }
}
