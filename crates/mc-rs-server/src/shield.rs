//! Shield — block 100% frontal damage, 5-tick cooldown if axe disable.

#[derive(Debug, Clone)]
pub struct Shield {
    pub raised: bool,
    pub raise_ticks: u32,
    pub disabled_ticks: u32,
    pub durability: u16,
    pub pattern_bytes: Option<Vec<u8>>, // banner pattern NBT
}

/// Activation delay (5 ticks before block).
pub const ACTIVATION_DELAY: u32 = 5;
/// Axe disable duration (5s = 100 ticks).
pub const AXE_DISABLE_DURATION: u32 = 100;
/// Max durability.
pub const MAX_DURABILITY: u16 = 336;

impl Shield {
    pub fn new() -> Self {
        Self {
            raised: false,
            raise_ticks: 0,
            disabled_ticks: 0,
            durability: MAX_DURABILITY,
            pattern_bytes: None,
        }
    }

    pub fn raise(&mut self) -> bool {
        if self.disabled_ticks > 0 {
            return false;
        }
        self.raised = true;
        self.raise_ticks = ACTIVATION_DELAY;
        true
    }

    pub fn lower(&mut self) {
        self.raised = false;
        self.raise_ticks = 0;
    }

    pub fn is_active(&self) -> bool {
        self.raised && self.raise_ticks == 0
    }

    /// Axe hit disables shield.
    pub fn disable(&mut self) {
        self.disabled_ticks = AXE_DISABLE_DURATION;
        self.raised = false;
    }

    pub fn tick(&mut self) {
        if self.raise_ticks > 0 {
            self.raise_ticks -= 1;
        }
        if self.disabled_ticks > 0 {
            self.disabled_ticks -= 1;
        }
    }

    /// Absorb N damage, consumes durability.
    pub fn absorb_damage(&mut self, damage: f32) -> f32 {
        if !self.is_active() {
            return damage;
        }
        self.durability = self.durability.saturating_sub(damage.ceil() as u16);
        0.0
    }
}

impl Default for Shield {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axe_disables_shield() {
        let mut s = Shield::new();
        s.disable();
        assert!(!s.raise());
    }

    #[test]
    fn active_absorbs_damage() {
        let mut s = Shield::new();
        s.raise();
        s.raise_ticks = 0;
        assert_eq!(s.absorb_damage(5.0), 0.0);
    }
}
