//! Ravager — bêteau illager de raids.

#[derive(Debug, Clone)]
pub struct Ravager {
    pub attack_cooldown: u32,
    pub stun_ticks: u32,
    pub roar_ticks: u32,
    pub target_entity: Option<u64>,
    pub rider: Option<u64>, // illager rider
}

/// HP (100).
pub const HP_MAX: f32 = 100.0;
/// Melee damage.
pub const MELEE_DAMAGE: f32 = 12.0;
/// Stun duration when shield blocks attack.
pub const STUN_DURATION: u32 = 80;
/// Roar cooldown.
pub const ROAR_COOLDOWN: u32 = 100;
/// Knockback roar.
pub const ROAR_KNOCKBACK: f32 = 2.0;

impl Ravager {
    pub fn new() -> Self {
        Self {
            attack_cooldown: 0,
            stun_ticks: 0,
            roar_ticks: 0,
            target_entity: None,
            rider: None,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
        if self.stun_ticks > 0 {
            self.stun_ticks -= 1;
        }
        if self.roar_ticks > 0 {
            self.roar_ticks -= 1;
        }
    }

    pub fn is_stunned(&self) -> bool {
        self.stun_ticks > 0
    }

    pub fn on_shield_block(&mut self) {
        self.stun_ticks = STUN_DURATION;
    }

    pub fn roar(&mut self) -> bool {
        if self.roar_ticks > 0 || self.is_stunned() {
            return false;
        }
        self.roar_ticks = ROAR_COOLDOWN;
        true
    }

    /// Ravagers can destroy crops + leaves.
    pub fn destroys_leaves() -> bool {
        true
    }
}

impl Default for Ravager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stun_prevents_roar() {
        let mut r = Ravager::new();
        r.on_shield_block();
        assert!(!r.roar());
    }
}
