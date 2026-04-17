//! Fireball — large (ghast) + small (blaze / fire charge).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireballKind {
    Large,  // Ghast fireball (exploding)
    Small,  // Blaze / fire charge (no explosion, ignite)
    Dragon, // Ender dragon breath (AOE on hit)
}

#[derive(Debug, Clone)]
pub struct Fireball {
    pub kind: FireballKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub shooter: u64,
    pub explosion_power: f32,
    pub age: u32,
}

/// Ghast fireball power.
pub const GHAST_POWER: f32 = 1.0;
/// Damage on contact (small fireball).
pub const SMALL_DIRECT_DAMAGE: f32 = 5.0;
/// Despawn after 120 ticks if no hit.
pub const DESPAWN_TICKS: u32 = 120;

impl Fireball {
    pub fn new_ghast(x: f64, y: f64, z: f64, mx: f64, my: f64, mz: f64, shooter: u64) -> Self {
        Self {
            kind: FireballKind::Large,
            x,
            y,
            z,
            motion_x: mx,
            motion_y: my,
            motion_z: mz,
            shooter,
            explosion_power: GHAST_POWER,
            age: 0,
        }
    }

    pub fn new_small(x: f64, y: f64, z: f64, mx: f64, my: f64, mz: f64, shooter: u64) -> Self {
        Self {
            kind: FireballKind::Small,
            x,
            y,
            z,
            motion_x: mx,
            motion_y: my,
            motion_z: mz,
            shooter,
            explosion_power: 0.0,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        // Fireballs don't have gravity, but have slight drag.
        self.motion_x *= 0.95;
        self.motion_y *= 0.95;
        self.motion_z *= 0.95;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.age += 1;
    }

    /// Fireballs can be deflected by hits (ping-ponged).
    pub fn deflect(&mut self, dir_x: f64, dir_y: f64, dir_z: f64) {
        self.motion_x = dir_x;
        self.motion_y = dir_y;
        self.motion_z = dir_z;
    }

    pub fn is_expired(&self) -> bool {
        self.age >= DESPAWN_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghast_has_power() {
        let f = Fireball::new_ghast(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1);
        assert_eq!(f.explosion_power, GHAST_POWER);
    }

    #[test]
    fn small_no_power() {
        let f = Fireball::new_small(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1);
        assert_eq!(f.explosion_power, 0.0);
    }
}
