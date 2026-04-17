//! Sculk systems — sculk, sculk vein, sculk catalyst, sculk shrieker.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SculkKind {
    Basic,      // Plain sculk block
    Vein,       // 1-6 faces attached
    Catalyst,   // Spreads sculk on mob death
    Sensor,     // Detects vibrations
    Shrieker,   // Summons warden
    Calibrated, // Filtered sensor
}

#[derive(Debug, Clone)]
pub struct SculkShrieker {
    pub can_summon: bool,
    pub warning_level: u8,
    pub cooldown_ticks: u32,
    pub last_player: Option<u64>,
}

/// Shrieker cooldown between activations (60 ticks vanilla).
pub const SHRIEKER_COOLDOWN: u32 = 60;
/// Warning levels: 1 to 4 triggers warden.
pub const MAX_WARNING: u8 = 4;

impl SculkShrieker {
    pub fn new(can_summon: bool) -> Self {
        Self {
            can_summon,
            warning_level: 0,
            cooldown_ticks: 0,
            last_player: None,
        }
    }

    pub fn activate(&mut self, player: u64) -> bool {
        if self.cooldown_ticks > 0 {
            return false;
        }
        // Only increment if same player or new.
        self.warning_level = (self.warning_level + 1).min(MAX_WARNING);
        self.cooldown_ticks = SHRIEKER_COOLDOWN;
        self.last_player = Some(player);
        true
    }

    pub fn should_summon_warden(&self) -> bool {
        self.can_summon && self.warning_level >= MAX_WARNING
    }

    pub fn tick(&mut self) {
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
    }

    /// Warning level decays over time (1200 ticks = 1 min between decay).
    pub fn warning_decay_interval() -> u32 {
        1200
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_blocks_activate() {
        let mut s = SculkShrieker::new(true);
        s.activate(1);
        assert!(!s.activate(1));
    }

    #[test]
    fn reach_max_summons() {
        let mut s = SculkShrieker::new(true);
        for _ in 0..MAX_WARNING {
            s.cooldown_ticks = 0;
            s.activate(1);
        }
        assert!(s.should_summon_warden());
    }
}
