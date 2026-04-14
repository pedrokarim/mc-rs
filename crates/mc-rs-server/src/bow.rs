//! Bow — charge-based damage/speed.

#[derive(Debug, Clone)]
pub struct Bow {
    pub charge_ticks: u32,
    pub durability: u16,
}

/// Full charge time (20 ticks = 1s).
pub const FULL_CHARGE: u32 = 20;
/// Max durability.
pub const MAX_DURABILITY: u16 = 384;
/// Max velocity of fully-drawn arrow (3.0).
pub const MAX_ARROW_VELOCITY: f64 = 3.0;

impl Bow {
    pub fn new() -> Self {
        Self { charge_ticks: 0, durability: MAX_DURABILITY }
    }

    pub fn start_draw(&mut self) {
        self.charge_ticks = 0;
    }

    pub fn draw_tick(&mut self) {
        self.charge_ticks = (self.charge_ticks + 1).min(FULL_CHARGE);
    }

    /// Arrow power is 0-1 based on charge.
    pub fn charge_ratio(&self) -> f32 {
        (self.charge_ticks as f32 / FULL_CHARGE as f32).min(1.0)
    }

    pub fn arrow_velocity(&self) -> f64 {
        MAX_ARROW_VELOCITY * self.charge_ratio() as f64
    }

    pub fn release(&mut self) -> Option<f32> {
        if self.charge_ticks == 0 {
            return None;
        }
        let ratio = self.charge_ratio();
        self.durability = self.durability.saturating_sub(1);
        self.charge_ticks = 0;
        Some(ratio)
    }

    /// Critical chance when fully drawn.
    pub fn is_critical(&self) -> bool {
        self.charge_ticks >= FULL_CHARGE
    }
}

impl Default for Bow {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_draw_half_ratio() {
        let mut b = Bow::new();
        b.charge_ticks = FULL_CHARGE / 2;
        assert!((b.charge_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn full_draw_critical() {
        let mut b = Bow::new();
        b.charge_ticks = FULL_CHARGE;
        assert!(b.is_critical());
    }
}
