//! Warden — port PMMP + Wiki. Mob sculk-based avec anger system.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WardenState {
    Emerging, // Spawning depuis sculk shrieker
    Roaring,  // Sonic roar
    Pursuing, // Chasing target
    Digging,  // Returning to ground
    Sniffing, // Seeking
    Idle,
}

#[derive(Debug, Clone)]
pub struct AngerEntry {
    pub target_id: u64,
    pub level: u16,
}

#[derive(Debug, Clone)]
pub struct Warden {
    pub state: WardenState,
    pub anger: Vec<AngerEntry>,
    pub active_target: Option<u64>,
}

/// Max anger level.
pub const MAX_ANGER: u16 = 150;
/// Anger threshold pour attack.
pub const ATTACK_THRESHOLD: u16 = 80;
/// Anger decay par seconde.
pub const ANGER_DECAY_PER_SEC: u16 = 1;
/// Anger tick pour un disturbance sound.
pub const DISTURBANCE_ANGER: u16 = 10;
/// Anger tick pour mob kill sighted.
pub const KILL_ANGER: u16 = 35;
/// Anger tick pour projectile hit.
pub const PROJECTILE_ANGER: u16 = 10;
/// Anger tick pour melee hit.
pub const MELEE_ANGER: u16 = 35;

impl Warden {
    pub fn new() -> Self {
        Self {
            state: WardenState::Emerging,
            anger: Vec::new(),
            active_target: None,
        }
    }

    pub fn add_anger(&mut self, target_id: u64, amount: u16) {
        if let Some(e) = self.anger.iter_mut().find(|e| e.target_id == target_id) {
            e.level = (e.level + amount).min(MAX_ANGER);
        } else {
            self.anger.push(AngerEntry {
                target_id,
                level: amount.min(MAX_ANGER),
            });
        }
        self.update_target();
    }

    pub fn decay_all(&mut self) {
        for entry in &mut self.anger {
            entry.level = entry.level.saturating_sub(ANGER_DECAY_PER_SEC);
        }
        self.anger.retain(|e| e.level > 0);
        self.update_target();
    }

    fn update_target(&mut self) {
        self.active_target = self
            .anger
            .iter()
            .max_by_key(|e| e.level)
            .filter(|e| e.level >= ATTACK_THRESHOLD)
            .map(|e| e.target_id);
    }

    pub fn should_attack(&self) -> bool {
        self.active_target.is_some()
    }

    pub fn anger_for(&self, target: u64) -> u16 {
        self.anger
            .iter()
            .find(|e| e.target_id == target)
            .map(|e| e.level)
            .unwrap_or(0)
    }
}

impl Default for Warden {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anger_builds_up() {
        let mut w = Warden::new();
        w.add_anger(42, DISTURBANCE_ANGER);
        assert_eq!(w.anger_for(42), 10);
    }

    #[test]
    fn anger_decays() {
        let mut w = Warden::new();
        w.add_anger(42, 5);
        w.decay_all();
        assert_eq!(w.anger_for(42), 4);
    }

    #[test]
    fn below_threshold_no_attack() {
        let mut w = Warden::new();
        w.add_anger(42, 50);
        assert!(!w.should_attack());
    }

    #[test]
    fn over_threshold_attacks() {
        let mut w = Warden::new();
        w.add_anger(42, 100);
        assert!(w.should_attack());
        assert_eq!(w.active_target, Some(42));
    }
}
