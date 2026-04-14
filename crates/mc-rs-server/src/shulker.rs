//! Shulker — stationnaire dans End City, teleport + projectile levitate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShulkerColor {
    Default,
    White, Orange, Magenta, LightBlue, Yellow, Lime, Pink, Gray,
    LightGray, Cyan, Purple, Blue, Brown, Green, Red, Black,
}

#[derive(Debug, Clone)]
pub struct Shulker {
    pub color: ShulkerColor,
    pub open_ticks: u32,
    pub attack_cooldown: u32,
    pub attached_face: u8, // 0=down, 1=up, 2=north, 3=south, 4=west, 5=east
}

/// Open duration before shoot (20 ticks).
pub const OPEN_DURATION: u32 = 20;
/// Attack cooldown (40-80 ticks randomized).
pub const ATTACK_COOLDOWN_MIN: u32 = 40;
pub const ATTACK_COOLDOWN_MAX: u32 = 80;
/// Levitation duration (10s = 200 ticks).
pub const LEVITATION_DURATION: u32 = 200;
/// Sight range (16 blocs).
pub const SIGHT_RANGE: f64 = 16.0;

impl Shulker {
    pub fn new(color: ShulkerColor) -> Self {
        Self {
            color,
            open_ticks: 0,
            attack_cooldown: 0,
            attached_face: 1,
        }
    }

    pub fn open(&mut self) {
        self.open_ticks = OPEN_DURATION;
    }

    pub fn is_open(&self) -> bool {
        self.open_ticks > 0
    }

    pub fn tick(&mut self) {
        if self.open_ticks > 0 {
            self.open_ticks -= 1;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    pub fn can_attack(&self) -> bool {
        self.attack_cooldown == 0
    }

    pub fn start_attack(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.attack_cooldown = rng.gen_range(ATTACK_COOLDOWN_MIN..=ATTACK_COOLDOWN_MAX);
        self.open();
    }

    /// Teleport if damaged + no peek.
    pub fn teleport_chance_damage() -> f32 { 0.25 }

    /// Damage from shulker bullet.
    pub fn bullet_damage() -> f32 { 4.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_then_close() {
        let mut s = Shulker::new(ShulkerColor::Default);
        s.open();
        assert!(s.is_open());
        for _ in 0..=OPEN_DURATION {
            s.tick();
        }
        assert!(!s.is_open());
    }
}
