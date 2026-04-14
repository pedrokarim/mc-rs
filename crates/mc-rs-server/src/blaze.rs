//! Blaze — mob du Nether qui lance fire charges.

#[derive(Debug, Clone)]
pub struct Blaze {
    pub on_fire: bool,
    pub attack_cooldown: u32,
    pub burst_count: u8,
    pub target_entity: Option<u64>,
}

/// Fire charges per burst (3).
pub const BURST_SIZE: u8 = 3;
/// Cooldown between bursts (60 ticks = 3s).
pub const BURST_COOLDOWN: u32 = 60;
/// Cooldown between individual charges in burst (10 ticks).
pub const CHARGE_INTERVAL: u32 = 10;
/// Attack range (48 blocs).
pub const ATTACK_RANGE: f64 = 48.0;

impl Blaze {
    pub fn new() -> Self {
        Self {
            on_fire: false,
            attack_cooldown: 0,
            burst_count: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    pub fn try_attack(&mut self) -> Option<u8> {
        if self.attack_cooldown > 0 {
            return None;
        }
        self.burst_count += 1;
        if self.burst_count >= BURST_SIZE {
            self.burst_count = 0;
            self.attack_cooldown = BURST_COOLDOWN;
        } else {
            self.attack_cooldown = CHARGE_INTERVAL;
        }
        self.on_fire = true;
        Some(self.burst_count)
    }

    /// Damaged by water/snowballs.
    pub fn damaged_by_water() -> bool { true }
    pub fn damaged_by_snowball_amount() -> f32 { 3.0 }

    pub fn drop_rod_chance() -> f32 { 0.5 }
    pub fn drop_glowstone_dust_chance_looting() -> f32 { 0.3 }
}

impl Default for Blaze {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_fires_3() {
        let mut b = Blaze::new();
        let mut total = 0;
        for _ in 0..3 {
            if b.try_attack().is_some() {
                total += 1;
            }
            b.attack_cooldown = 0;
        }
        assert_eq!(total, 3);
    }
}
