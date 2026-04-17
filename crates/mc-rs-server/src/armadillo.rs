//! Armadillo — 1.21 mob qui roll into ball + drops scute.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmadilloState {
    Idle,
    Rolling,
    Unrolling,
    Scared,
}

#[derive(Debug, Clone)]
pub struct Armadillo {
    pub state: ArmadilloState,
    pub state_ticks: u32,
    pub scute_cooldown: u32,
    pub age: i32,
}

/// Scute drop cooldown (5 min).
pub const SCUTE_COOLDOWN: u32 = 5 * 60 * 20;
/// Roll duration (3s).
pub const ROLL_DURATION: u32 = 3 * 20;
/// Scared timeout before unroll (2s).
pub const SCARED_TIMEOUT: u32 = 2 * 20;

impl Armadillo {
    pub fn new() -> Self {
        Self {
            state: ArmadilloState::Idle,
            state_ticks: 0,
            scute_cooldown: 0,
            age: 0,
        }
    }

    pub fn scare(&mut self) {
        self.state = ArmadilloState::Scared;
        self.state_ticks = SCARED_TIMEOUT;
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.scute_cooldown > 0 {
            self.scute_cooldown -= 1;
        }
        if self.state_ticks > 0 {
            self.state_ticks -= 1;
            if self.state_ticks == 0 {
                self.state = ArmadilloState::Idle;
            }
        }
    }

    pub fn try_drop_scute(&mut self) -> bool {
        if self.scute_cooldown > 0 {
            return false;
        }
        self.scute_cooldown = SCUTE_COOLDOWN;
        true
    }

    pub fn is_rolled(&self) -> bool {
        matches!(self.state, ArmadilloState::Rolling | ArmadilloState::Scared)
    }

    /// Immune to damage while rolled.
    pub fn damage_immune_rolled() -> bool {
        true
    }
}

impl Default for Armadillo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scared_is_rolled() {
        let mut a = Armadillo::new();
        a.scare();
        assert!(a.is_rolled());
    }

    #[test]
    fn scute_cooldown_enforced() {
        let mut a = Armadillo::new();
        assert!(a.try_drop_scute());
        assert!(!a.try_drop_scute());
    }
}
