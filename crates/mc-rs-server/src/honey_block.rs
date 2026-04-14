//! Honey block — sticky, slow, slide.

/// Movement speed multiplier on top of honey.
pub const MOVEMENT_SPEED: f32 = 0.4;
/// Fall damage absorption (80%).
pub const FALL_DAMAGE_REDUCTION: f32 = 0.8;
/// Jump strength reduction.
pub const JUMP_REDUCTION: f32 = 0.5;
/// Slide down walls when touching honey.
pub const SLIDE_SPEED: f64 = 0.05;

/// Honey + piston → pushes diagonally-connected blocks.
pub fn sticks_diagonally_with_pistons() -> bool { true }
/// But honey does NOT stick to slime blocks.
pub fn sticks_to_slime() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slows_movement() {
        assert!(MOVEMENT_SPEED < 1.0);
    }

    #[test]
    fn honey_not_sticking_to_slime() {
        assert!(!sticks_to_slime());
    }
}
