//! Ender pearl — teleport thrown, 5 dmg on landing.

#[derive(Debug, Clone)]
pub struct EnderPearl {
    pub launcher_id: u64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub age: u32,
}

/// Damage to player on teleport (5 vanilla).
pub const TELEPORT_DAMAGE: f32 = 5.0;
/// Endermite spawn chance (5% vanilla).
pub const ENDERMITE_CHANCE: f32 = 0.05;

impl EnderPearl {
    pub fn new(launcher: u64, mx: f64, my: f64, mz: f64) -> Self {
        Self {
            launcher_id: launcher,
            motion_x: mx,
            motion_y: my,
            motion_z: mz,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        self.motion_y -= 0.03; // gravity
        self.motion_x *= 0.99;
        self.motion_z *= 0.99;
        self.age += 1;
    }
}

/// Chorus fruit teleport — 5-8 blocks randomized.
pub fn chorus_fruit_teleport_range() -> f64 {
    8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pearl_gains_age() {
        let mut p = EnderPearl::new(0, 0.0, 1.0, 0.0);
        let start_age = p.age;
        p.tick();
        assert!(p.age > start_age);
    }
}
