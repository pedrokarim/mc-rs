//! Entity common tick logic — physics, fall damage, void, air.

#[derive(Debug, Clone)]
pub struct EntityCommon {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub on_ground: bool,
    pub fall_distance: f32,
    pub air_ticks: u16,
    pub fire_ticks: u16,
    pub freeze_ticks: u32,
}

/// Max air (20 sec = 300 ticks).
pub const MAX_AIR: u16 = 300;
/// Drowning damage per 2s.
pub const DROWN_DAMAGE: f32 = 2.0;
/// Void damage level (y < -64 vanilla).
pub const VOID_Y: f64 = -64.0;
/// Fall distance threshold before damage.
pub const FALL_DAMAGE_THRESHOLD: f32 = 3.0;

impl EntityCommon {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x, y, z,
            motion_x: 0.0, motion_y: 0.0, motion_z: 0.0,
            on_ground: false,
            fall_distance: 0.0,
            air_ticks: MAX_AIR,
            fire_ticks: 0,
            freeze_ticks: 0,
        }
    }

    pub fn apply_motion(&mut self) {
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
    }

    pub fn apply_gravity(&mut self, gravity: f64, drag: f64) {
        self.motion_y -= gravity;
        self.motion_x *= drag;
        self.motion_y *= drag;
        self.motion_z *= drag;
    }

    pub fn is_in_void(&self) -> bool {
        self.y < VOID_Y
    }

    pub fn fall_damage(&self) -> f32 {
        (self.fall_distance - FALL_DAMAGE_THRESHOLD).max(0.0)
    }

    pub fn reset_fall(&mut self) {
        if self.on_ground {
            self.fall_distance = 0.0;
        }
    }

    /// Ignite from fire/lava.
    pub fn ignite(&mut self, ticks: u16) {
        self.fire_ticks = self.fire_ticks.max(ticks);
    }

    pub fn extinguish(&mut self) {
        self.fire_ticks = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_detection() {
        let mut e = EntityCommon::new(0.0, -80.0, 0.0);
        assert!(e.is_in_void());
        e.y = 100.0;
        assert!(!e.is_in_void());
    }

    #[test]
    fn fall_damage_threshold() {
        let mut e = EntityCommon::new(0.0, 0.0, 0.0);
        e.fall_distance = 5.0;
        assert!(e.fall_damage() > 0.0);
        e.fall_distance = 1.0;
        assert_eq!(e.fall_damage(), 0.0);
    }
}
