//! Thrown potion — splash + lingering.

#[derive(Debug, Clone)]
pub struct ThrownPotion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub lingering: bool,
    pub effects: Vec<ThrownEffect>,
    pub age: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct ThrownEffect {
    pub kind: &'static str,
    pub amplifier: u8,
    pub duration: u32,
}

/// Splash radius (4 blocks vanilla).
pub const SPLASH_RADIUS: f64 = 4.0;
/// Splash damage (Harming) center vs edge ratio.
pub const SPLASH_EDGE_EFFECT: f32 = 0.25;

/// Lingering cloud produced on landing.
pub fn lingering_cloud_duration() -> u32 {
    200 // ~10s
}

impl ThrownPotion {
    pub fn new(x: f64, y: f64, z: f64, effects: Vec<ThrownEffect>, lingering: bool) -> Self {
        Self {
            x, y, z,
            motion_x: 0.0, motion_y: 0.0, motion_z: 0.0,
            lingering,
            effects,
            age: 0,
        }
    }

    pub fn tick(&mut self) {
        self.motion_y -= 0.04;
        self.x += self.motion_x;
        self.y += self.motion_y;
        self.z += self.motion_z;
        self.age += 1;
    }

    /// Apply effect scale depending on distance from center.
    pub fn effect_scale_at_distance(dist: f64) -> f32 {
        let ratio = 1.0 - (dist / SPLASH_RADIUS) as f32;
        ratio.clamp(SPLASH_EDGE_EFFECT, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_full_effect() {
        assert_eq!(ThrownPotion::effect_scale_at_distance(0.0), 1.0);
    }

    #[test]
    fn edge_minimum() {
        assert_eq!(
            ThrownPotion::effect_scale_at_distance(SPLASH_RADIUS * 2.0),
            SPLASH_EDGE_EFFECT
        );
    }
}
