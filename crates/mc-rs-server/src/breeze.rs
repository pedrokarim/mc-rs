//! Breeze — 1.21 wind-based mob du trial chamber.

#[derive(Debug, Clone)]
pub struct Breeze {
    pub attack_cooldown: u32,
    pub jump_cooldown: u32,
    pub sliding_ticks: u32,
    pub target_entity: Option<u64>,
}

/// Wind charge range.
pub const WIND_RANGE: f64 = 24.0;
/// Attack cooldown.
pub const ATTACK_COOLDOWN: u32 = 30;
/// Jump cooldown (reposition).
pub const JUMP_COOLDOWN: u32 = 40;
/// Wind charge damage.
pub const WIND_DAMAGE: f32 = 1.0;

impl Breeze {
    pub fn new() -> Self {
        Self {
            attack_cooldown: 0,
            jump_cooldown: 0,
            sliding_ticks: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 { self.attack_cooldown -= 1; }
        if self.jump_cooldown > 0 { self.jump_cooldown -= 1; }
        if self.sliding_ticks > 0 { self.sliding_ticks -= 1; }
    }

    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0
    }

    pub fn fire_wind(&mut self) {
        self.attack_cooldown = ATTACK_COOLDOWN;
    }

    pub fn can_jump(&self) -> bool {
        self.jump_cooldown == 0
    }

    /// Immune to projectile damage (deflected by wind).
    pub fn deflects_projectiles() -> bool { true }
}

impl Default for Breeze {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_after_fire() {
        let mut b = Breeze::new();
        assert!(b.can_attack());
        b.fire_wind();
        assert!(!b.can_attack());
    }
}
