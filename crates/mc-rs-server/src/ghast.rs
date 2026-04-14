//! Ghast — mob du Nether, lance fireballs.

#[derive(Debug, Clone)]
pub struct Ghast {
    pub attack_cooldown: u32,
    pub charging: bool,
    pub charge_ticks: u32,
    pub target_entity: Option<u64>,
}

/// Charging duration (20 ticks).
pub const CHARGE_DURATION: u32 = 20;
/// Attack cooldown between shots (80-100 ticks randomized).
pub const ATTACK_COOLDOWN_MIN: u32 = 80;
pub const ATTACK_COOLDOWN_MAX: u32 = 100;
/// Sight range (64 blocs).
pub const SIGHT_RANGE: f64 = 64.0;
/// Fireball deflectable by hitting, deal huge dmg.
pub const FIREBALL_DAMAGE: f32 = 17.0;

impl Ghast {
    pub fn new() -> Self {
        Self {
            attack_cooldown: 0,
            charging: false,
            charge_ticks: 0,
            target_entity: None,
        }
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
        if self.charging {
            self.charge_ticks += 1;
        }
    }

    pub fn start_charging(&mut self, target: u64) {
        if self.attack_cooldown > 0 {
            return;
        }
        self.target_entity = Some(target);
        self.charging = true;
        self.charge_ticks = 0;
    }

    pub fn try_fire(&mut self) -> bool {
        if !self.charging || self.charge_ticks < CHARGE_DURATION {
            return false;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.attack_cooldown = rng.gen_range(ATTACK_COOLDOWN_MIN..=ATTACK_COOLDOWN_MAX);
        self.charging = false;
        self.charge_ticks = 0;
        true
    }

    /// Fireball explosion power (1.0 vanilla).
    pub fn fireball_power() -> f32 { 1.0 }
}

impl Default for Ghast {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_charge_before_fire() {
        let mut g = Ghast::new();
        g.start_charging(42);
        assert!(!g.try_fire());
    }

    #[test]
    fn fires_after_charge() {
        let mut g = Ghast::new();
        g.start_charging(42);
        for _ in 0..CHARGE_DURATION {
            g.tick();
        }
        assert!(g.try_fire());
    }
}
