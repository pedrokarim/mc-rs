//! Trident — throwable + riptide/loyalty/channeling/impaling.

#[derive(Debug, Clone)]
pub struct Trident {
    pub charge_ticks: u32,
    pub durability: u16,
    pub riptide_level: u8,
    pub loyalty_level: u8,
    pub impaling_level: u8,
    pub channeling: bool,
}

/// Full charge (10 ticks).
pub const FULL_CHARGE: u32 = 10;
/// Melee damage.
pub const MELEE_DAMAGE: f32 = 9.0;
/// Thrown damage.
pub const THROWN_DAMAGE: f32 = 8.0;
/// Max durability.
pub const MAX_DURABILITY: u16 = 250;

impl Trident {
    pub fn new() -> Self {
        Self {
            charge_ticks: 0,
            durability: MAX_DURABILITY,
            riptide_level: 0,
            loyalty_level: 0,
            impaling_level: 0,
            channeling: false,
        }
    }

    pub fn is_fully_charged(&self) -> bool {
        self.charge_ticks >= FULL_CHARGE
    }

    /// Riptide propels player — level N gives N*3 blocks impulse.
    pub fn riptide_impulse(&self) -> f64 {
        self.riptide_level as f64 * 3.0
    }

    /// Returns true if in rain/water (riptide activates).
    pub fn can_riptide(in_water: bool, in_rain: bool) -> bool {
        in_water || in_rain
    }

    /// Impaling bonus damage vs water mobs.
    pub fn impaling_bonus_damage(&self, target_in_water: bool) -> f32 {
        if target_in_water {
            self.impaling_level as f32 * 2.5
        } else {
            0.0
        }
    }

    /// Channeling summons lightning during thunderstorm.
    pub fn can_channel(&self, in_thunder: bool) -> bool {
        self.channeling && in_thunder
    }
}

impl Default for Trident {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riptide_scales_linear() {
        let mut t = Trident::new();
        t.riptide_level = 3;
        assert_eq!(t.riptide_impulse(), 9.0);
    }

    #[test]
    fn channeling_needs_thunder() {
        let mut t = Trident::new();
        t.channeling = true;
        assert!(!t.can_channel(false));
        assert!(t.can_channel(true));
    }
}
