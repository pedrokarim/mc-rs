//! Generic AI state machine for mobs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Wandering,
    ChasingTarget,
    AttackingTarget,
    Fleeing,
    Breeding,
    EatingGrass,
    Sleeping,
    Panicking,
    Sitting,
    Following,
    Returning,
}

#[derive(Debug, Clone)]
pub struct AiContext {
    pub state: AiState,
    pub state_duration: u32,
    pub state_target: Option<u64>,
    pub home_position: Option<(i32, i32, i32)>,
    pub pathfinding_cooldown: u32,
}

impl AiContext {
    pub fn new() -> Self {
        Self {
            state: AiState::Idle,
            state_duration: 0,
            state_target: None,
            home_position: None,
            pathfinding_cooldown: 0,
        }
    }

    pub fn transition(&mut self, new_state: AiState) {
        if self.state != new_state {
            self.state = new_state;
            self.state_duration = 0;
        }
    }

    pub fn tick(&mut self) {
        self.state_duration += 1;
        if self.pathfinding_cooldown > 0 {
            self.pathfinding_cooldown -= 1;
        }
    }
}

impl Default for AiContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Stuck detection — if position didn't change for N ticks.
pub const STUCK_THRESHOLD_TICKS: u32 = 100;
/// Wander range.
pub const WANDER_RANGE: f64 = 16.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_resets_duration() {
        let mut c = AiContext::new();
        c.state_duration = 10;
        c.transition(AiState::Wandering);
        assert_eq!(c.state_duration, 0);
    }
}
