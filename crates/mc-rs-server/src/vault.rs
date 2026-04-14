//! Vault — 1.21 trial chamber reward block.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultState {
    Inactive,
    Active,
    Unlocking,
    Ejecting,
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub state: VaultState,
    pub required_key: &'static str, // e.g. trial_key / ominous_trial_key
    pub ominous: bool,
    pub players_rewarded: Vec<u64>,
    pub display_item: Option<u16>,
}

/// Default key.
pub const TRIAL_KEY: &str = "minecraft:trial_key";
/// Ominous variant key.
pub const OMINOUS_KEY: &str = "minecraft:ominous_trial_key";

impl Vault {
    pub fn new(ominous: bool) -> Self {
        Self {
            state: VaultState::Inactive,
            required_key: if ominous { OMINOUS_KEY } else { TRIAL_KEY },
            ominous,
            players_rewarded: Vec::new(),
            display_item: None,
        }
    }

    /// Insert key, reward player once.
    pub fn unlock_for(&mut self, player: u64, key: &str) -> bool {
        if key != self.required_key {
            return false;
        }
        if self.players_rewarded.contains(&player) {
            return false;
        }
        self.players_rewarded.push(player);
        self.state = VaultState::Unlocking;
        true
    }

    pub fn tick(&mut self) {
        match self.state {
            VaultState::Unlocking => self.state = VaultState::Ejecting,
            VaultState::Ejecting => self.state = VaultState::Active,
            _ => {}
        }
    }

    /// Activate when player in range.
    pub fn activate(&mut self) {
        if self.state == VaultState::Inactive {
            self.state = VaultState::Active;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cant_unlock_with_wrong_key() {
        let mut v = Vault::new(false);
        assert!(!v.unlock_for(1, OMINOUS_KEY));
    }

    #[test]
    fn player_rewarded_only_once() {
        let mut v = Vault::new(false);
        assert!(v.unlock_for(1, TRIAL_KEY));
        assert!(!v.unlock_for(1, TRIAL_KEY));
    }
}
