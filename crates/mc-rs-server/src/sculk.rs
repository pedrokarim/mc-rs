//! Sculk system — port conceptuel de blocks Sculk (Warden feature 1.19+).
//!
//! Sculk Sensor détecte vibrations → émet signal redstone.
//! Sculk Shrieker alerte le Warden après 4 activations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrationSource {
    Walk,
    Jump,
    Land,
    BlockBreak,
    BlockPlace,
    ProjectileImpact,
    EntityDeath,
    Explosion,
    ChestOpen,
    Eat,
    Splash,
    Thunder,
    ItemPickup,
    EntityHurt,
    Drink,
}

impl VibrationSource {
    /// Distance max de détection (blocs).
    pub fn detection_range(&self) -> u32 {
        match self {
            Self::Explosion | Self::Thunder => 16,
            Self::BlockBreak | Self::BlockPlace | Self::ProjectileImpact => 8,
            _ => 6,
        }
    }

    /// Signal redstone émis (0-15).
    pub fn redstone_signal(&self) -> u8 {
        match self {
            Self::Walk | Self::Jump => 3,
            Self::Land => 5,
            Self::BlockBreak | Self::BlockPlace => 7,
            Self::Explosion => 15,
            Self::Thunder => 13,
            Self::ProjectileImpact => 10,
            Self::EntityDeath | Self::EntityHurt => 8,
            Self::ChestOpen => 4,
            Self::Eat | Self::Drink => 6,
            Self::Splash => 4,
            Self::ItemPickup => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SculkSensorState {
    pub position: [i32; 3],
    pub current_signal: u8,
    pub cooldown_ticks: u32,
}

impl SculkSensorState {
    pub fn new(position: [i32; 3]) -> Self {
        Self {
            position,
            current_signal: 0,
            cooldown_ticks: 0,
        }
    }

    /// Ticks de cooldown après activation (vanilla = 40 ticks).
    pub fn trigger(&mut self, source: VibrationSource) {
        self.current_signal = source.redstone_signal();
        self.cooldown_ticks = 40;
    }

    pub fn tick(&mut self) {
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
            if self.cooldown_ticks == 0 {
                self.current_signal = 0;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SculkShriekerState {
    pub position: [i32; 3],
    pub activations: u32,
    pub last_activation_tick: u64,
    pub can_summon_warden: bool,
}

impl SculkShriekerState {
    pub fn new(position: [i32; 3], can_summon_warden: bool) -> Self {
        Self {
            position,
            activations: 0,
            last_activation_tick: 0,
            can_summon_warden,
        }
    }

    /// Active le shrieker. Retourne true si doit summon le Warden (4e activation).
    pub fn activate(&mut self, current_tick: u64) -> bool {
        self.activations += 1;
        self.last_activation_tick = current_tick;
        self.can_summon_warden && self.activations >= 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosion_max_signal() {
        assert_eq!(VibrationSource::Explosion.redstone_signal(), 15);
    }

    #[test]
    fn sensor_cooldown_40_ticks() {
        let mut s = SculkSensorState::new([0, 64, 0]);
        s.trigger(VibrationSource::BlockBreak);
        assert_eq!(s.cooldown_ticks, 40);
        assert_eq!(s.current_signal, 7);
    }

    #[test]
    fn shrieker_summons_warden_on_4th() {
        let mut sh = SculkShriekerState::new([0, 64, 0], true);
        assert!(!sh.activate(1));
        assert!(!sh.activate(2));
        assert!(!sh.activate(3));
        assert!(sh.activate(4));
    }
}
