//! Sculk sensor — detects vibrations.

#[derive(Debug, Clone)]
pub struct SculkSensor {
    pub active: bool,
    pub active_ticks: u32,
    pub cooldown: u32,
    pub last_vibration_power: u8,
    pub wave_frequency_filter: Option<u8>, // Calibrated sensor filter
}

/// Active duration.
pub const ACTIVE_DURATION: u32 = 40;
/// Cooldown.
pub const COOLDOWN: u32 = 40;
/// Max vibration range.
pub const VIBRATION_RANGE: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrationFrequency {
    Step = 1,
    FluidSplash = 2,
    BlockBreak = 3,
    BlockPlace = 4,
    ProjectileHit = 5,
    Explosion = 6,
    EntityDamaged = 7,
    ItemUse = 8,
    EntityDeath = 9,
    Detonate = 10,
    Prime = 11,
    Attack = 12,
    ShearBlock = 13,
    GoatHorn = 14,
    Drink = 15,
}

impl SculkSensor {
    pub fn new() -> Self {
        Self {
            active: false,
            active_ticks: 0,
            cooldown: 0,
            last_vibration_power: 0,
            wave_frequency_filter: None,
        }
    }

    pub fn receive_vibration(&mut self, freq: VibrationFrequency, distance: f64) -> bool {
        if self.cooldown > 0 {
            return false;
        }
        if let Some(filter) = self.wave_frequency_filter {
            if filter != freq as u8 {
                return false;
            }
        }
        self.active = true;
        self.active_ticks = ACTIVE_DURATION;
        // Power inversely proportional to distance.
        self.last_vibration_power = ((1.0 - (distance / VIBRATION_RANGE).min(1.0)) * 15.0) as u8;
        true
    }

    pub fn tick(&mut self) {
        if self.active_ticks > 0 {
            self.active_ticks -= 1;
            if self.active_ticks == 0 {
                self.active = false;
                self.cooldown = COOLDOWN;
            }
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
    }
}

impl Default for SculkSensor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_blocks_unmatched() {
        let mut s = SculkSensor::new();
        s.wave_frequency_filter = Some(VibrationFrequency::Step as u8);
        assert!(!s.receive_vibration(VibrationFrequency::Explosion, 1.0));
    }

    #[test]
    fn close_vibration_strong() {
        let mut s = SculkSensor::new();
        s.receive_vibration(VibrationFrequency::Step, 1.0);
        assert!(s.last_vibration_power > 10);
    }
}
