//! Dragon fight state machine — spawn dragon, pillars, egg drop.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragonFightPhase {
    PreDragon,
    SpawningDragon,
    DragonAlive,
    DragonDying,
    DragonDead,
    RespawnInProgress,
}

#[derive(Debug, Clone)]
pub struct DragonFight {
    pub phase: DragonFightPhase,
    pub dragon_entity_id: Option<u64>,
    pub crystals: Vec<u64>,
    pub previously_killed: bool,
    pub egg_dropped: bool,
    pub respawn_tick: u32,
}

/// Respawn charge duration (~200 ticks = 10s).
pub const RESPAWN_CHARGE: u32 = 200;

impl DragonFight {
    pub fn new() -> Self {
        Self {
            phase: DragonFightPhase::PreDragon,
            dragon_entity_id: None,
            crystals: Vec::new(),
            previously_killed: false,
            egg_dropped: false,
            respawn_tick: 0,
        }
    }

    pub fn spawn_dragon(&mut self, entity_id: u64) {
        self.dragon_entity_id = Some(entity_id);
        self.phase = DragonFightPhase::DragonAlive;
    }

    pub fn on_dragon_death(&mut self) {
        self.dragon_entity_id = None;
        if !self.previously_killed {
            // First kill drops egg.
            self.egg_dropped = true;
        }
        self.previously_killed = true;
        self.phase = DragonFightPhase::DragonDead;
    }

    pub fn start_respawn(&mut self) {
        if self.phase == DragonFightPhase::DragonDead {
            self.phase = DragonFightPhase::RespawnInProgress;
            self.respawn_tick = 0;
            self.egg_dropped = false;
        }
    }

    pub fn tick(&mut self) {
        if self.phase == DragonFightPhase::RespawnInProgress {
            self.respawn_tick += 1;
            if self.respawn_tick >= RESPAWN_CHARGE {
                self.phase = DragonFightPhase::SpawningDragon;
            }
        }
    }

    /// First kill drops egg + gateway. Later kills only portal.
    pub fn drops_egg(&self) -> bool {
        self.egg_dropped
    }
}

impl Default for DragonFight {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_kill_drops_egg() {
        let mut f = DragonFight::new();
        f.phase = DragonFightPhase::DragonAlive;
        f.on_dragon_death();
        assert!(f.drops_egg());
    }

    #[test]
    fn second_kill_no_egg() {
        let mut f = DragonFight::new();
        f.phase = DragonFightPhase::DragonAlive;
        f.on_dragon_death();
        f.start_respawn();
        f.phase = DragonFightPhase::DragonAlive;
        f.on_dragon_death();
        assert!(!f.drops_egg());
    }
}
