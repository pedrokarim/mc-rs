//! Elytra — wings for gliding, fireworks boost.

#[derive(Debug, Clone)]
pub struct ElytraState {
    pub gliding: bool,
    pub durability: u16,
    pub firework_boost_ticks: u32,
}

/// Max elytra durability (432 uses).
pub const MAX_DURABILITY: u16 = 432;
/// Firework rocket boost multiplier (lift per firework flight level).
pub const FIREWORK_LIFT_MULTIPLIER: f64 = 0.15;
/// Elytra damage per 1s gliding.
pub const DAMAGE_PER_SEC: u16 = 1;

impl ElytraState {
    pub fn new() -> Self {
        Self {
            gliding: false,
            durability: MAX_DURABILITY,
            firework_boost_ticks: 0,
        }
    }

    pub fn start_gliding(&mut self) -> bool {
        if self.durability == 0 {
            return false;
        }
        self.gliding = true;
        true
    }

    pub fn stop_gliding(&mut self) {
        self.gliding = false;
    }

    pub fn boost_with_firework(&mut self, flight_level: u8) {
        self.firework_boost_ticks = flight_level as u32 * 20;
    }

    pub fn tick(&mut self) {
        if self.gliding {
            if self.durability > 0 && rand::random::<u32>().is_multiple_of(20) {
                self.durability -= 1;
            }
            if self.durability == 0 {
                self.gliding = false;
            }
        }
        if self.firework_boost_ticks > 0 {
            self.firework_boost_ticks -= 1;
        }
    }

    pub fn is_broken(&self) -> bool {
        self.durability == 0
    }
}

impl Default for ElytraState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_cant_glide() {
        let mut e = ElytraState::new();
        e.durability = 0;
        assert!(!e.start_gliding());
    }

    #[test]
    fn firework_provides_boost() {
        let mut e = ElytraState::new();
        e.boost_with_firework(3);
        assert!(e.firework_boost_ticks > 0);
    }
}
