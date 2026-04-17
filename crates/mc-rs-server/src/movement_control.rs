//! Player movement state tracking (anti-cheat basics).

#[derive(Debug, Clone)]
pub struct MovementState {
    pub last_x: f64,
    pub last_y: f64,
    pub last_z: f64,
    pub last_update_tick: u64,
    pub velocity_sample: f64,
    pub on_ground: bool,
    pub is_flying: bool,
    pub is_swimming: bool,
    pub is_elytra: bool,
}

/// Max horizontal speed per tick (walking).
pub const MAX_WALK_SPEED_PER_TICK: f64 = 0.3;
/// Max speed sprinting.
pub const MAX_SPRINT_SPEED_PER_TICK: f64 = 0.4;
/// Max speed flying (creative).
pub const MAX_FLY_SPEED_PER_TICK: f64 = 0.8;
/// Grace ticks before flagging.
pub const GRACE_TICKS: u64 = 5;

impl MovementState {
    pub fn new(x: f64, y: f64, z: f64, tick: u64) -> Self {
        Self {
            last_x: x,
            last_y: y,
            last_z: z,
            last_update_tick: tick,
            velocity_sample: 0.0,
            on_ground: false,
            is_flying: false,
            is_swimming: false,
            is_elytra: false,
        }
    }

    pub fn update(&mut self, x: f64, y: f64, z: f64, tick: u64) -> f64 {
        let dx = x - self.last_x;
        let dy = y - self.last_y;
        let dz = z - self.last_z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let dt = (tick - self.last_update_tick).max(1);
        self.velocity_sample = dist / dt as f64;
        self.last_x = x;
        self.last_y = y;
        self.last_z = z;
        self.last_update_tick = tick;
        self.velocity_sample
    }

    pub fn is_suspicious(&self, allowed_max: f64) -> bool {
        self.velocity_sample > allowed_max * 1.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_speed() {
        let mut m = MovementState::new(0.0, 0.0, 0.0, 0);
        let v = m.update(1.0, 0.0, 0.0, 1);
        assert!((v - 1.0).abs() < 0.001);
    }
}
