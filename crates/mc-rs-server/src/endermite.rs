//! Endermite — small hostile mob from ender pearls.

#[derive(Debug, Clone)]
pub struct Endermite {
    pub age: u32,
}

/// Despawn after ~40 seconds (natural despawn).
pub const LIFETIME_TICKS: u32 = 800;

/// Damage.
pub const DAMAGE: f32 = 2.0;
/// HP.
pub const HP: f32 = 8.0;
/// Movement speed.
pub const SPEED: f32 = 0.25;

/// Endermen attack endermites.
pub fn endermen_hostile() -> bool {
    true
}

impl Endermite {
    pub fn new() -> Self {
        Self { age: 0 }
    }

    pub fn tick(&mut self) -> bool {
        self.age += 1;
        self.age >= LIFETIME_TICKS
    }
}

impl Default for Endermite {
    fn default() -> Self {
        Self::new()
    }
}

/// Ender pearl endermite spawn chance (5%).
pub const SPAWN_FROM_PEARL_CHANCE: f32 = 0.05;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_after_lifetime() {
        let mut e = Endermite::new();
        e.age = LIFETIME_TICKS;
        assert!(e.tick());
    }
}
