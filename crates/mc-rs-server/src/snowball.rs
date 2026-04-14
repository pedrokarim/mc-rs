//! Snowball — thrown projectile, 3 dmg on blaze.

#[derive(Debug, Clone)]
pub struct Snowball {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub thrower: u64,
    pub age: u32,
}

/// Damage vs blaze (3).
pub const BLAZE_DAMAGE: f32 = 3.0;
/// Damage vs other entities (0 — just kb).
pub const NORMAL_DAMAGE: f32 = 0.0;
/// Drag (0.99 per tick).
pub const DRAG: f64 = 0.99;
/// Gravity.
pub const GRAVITY: f64 = 0.03;
/// Despawn after 5 min (6000 ticks).
pub const DESPAWN_TICKS: u32 = 6000;

impl Snowball {
    pub fn new(x: f64, y: f64, z: f64, thrower: u64, mx: f64, my: f64, mz: f64) -> Self {
        Self { x, y, z, motion_x: mx, motion_y: my, motion_z: mz, thrower, age: 0 }
    }

    pub fn tick(&mut self) {
        self.motion_y -= GRAVITY;
        self.motion_x *= DRAG;
        self.motion_y *= DRAG;
        self.motion_z *= DRAG;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.age += 1;
    }
}

/// Egg acts similarly but chance spawns chicken.
pub const EGG_CHICKEN_CHANCE: f32 = 1.0 / 8.0;
pub const EGG_BABY_CHANCE: f32 = 1.0 / 32.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snowball_gravity_falls() {
        let mut s = Snowball::new(0.0, 100.0, 0.0, 0, 0.0, 0.0, 0.0);
        let y_start = s.y;
        for _ in 0..10 {
            s.tick();
        }
        assert!(s.y < y_start);
    }
}
