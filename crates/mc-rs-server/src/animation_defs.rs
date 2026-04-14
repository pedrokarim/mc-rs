//! Animation definitions for entities (arm swing, eat, block, sleep, etc.)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAnimation {
    NoAction = 0,
    SwingArm = 1,
    WakeUp = 2,
    CriticalHit = 3,
    MagicCriticalHit = 4,
    RowRight = 128,
    RowLeft = 129,
    EatingItem = 9,
    Respawn = 32,
}

/// Duration in ticks for swings.
pub const SWING_DURATION: u32 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swing_arm_1() {
        assert_eq!(ActorAnimation::SwingArm as i32, 1);
    }
}
