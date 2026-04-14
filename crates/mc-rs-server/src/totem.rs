//! Totem of Undying — prevents death, consumes totem, apply absorption + regen.

#[derive(Debug, Clone, Copy)]
pub struct TotemActivation {
    pub consumed: bool,
    pub regeneration_duration: u32,
    pub absorption_duration: u32,
    pub fire_resistance_duration: u32,
}

/// Regen effect (40 sec, amplifier 1).
pub const REGEN_DURATION: u32 = 40 * 20;
pub const REGEN_AMPLIFIER: u8 = 1;
/// Absorption (5 sec, amplifier 1).
pub const ABSORPTION_DURATION: u32 = 5 * 20;
pub const ABSORPTION_AMPLIFIER: u8 = 1;
/// Fire resistance (40 sec, amplifier 0).
pub const FIRE_RES_DURATION: u32 = 40 * 20;

/// Totem activates from offhand or main hand.
pub fn activate_totem() -> TotemActivation {
    TotemActivation {
        consumed: true,
        regeneration_duration: REGEN_DURATION,
        absorption_duration: ABSORPTION_DURATION,
        fire_resistance_duration: FIRE_RES_DURATION,
    }
}

/// HP set to 1 after activation.
pub const HEALTH_AFTER_ACTIVATION: f32 = 1.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_provides_effects() {
        let a = activate_totem();
        assert!(a.consumed);
        assert!(a.regeneration_duration > 0);
    }
}
