//! AreaEffectCloud — port PMMP `src/entity/object/AreaEffectCloud.php`.
//! Nuage d'effet (lingering potion) qui applique un effet aux entités proches.

use crate::effects::EffectKind;

#[derive(Debug, Clone)]
pub struct AreaEffectCloud {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub radius: f32,
    pub radius_on_use: f32,
    pub radius_per_tick: f32,
    pub duration: i32,
    pub wait_time: i32,
    pub reapplication_delay: i32,
    pub effect: Option<(EffectKind, u8, i32)>, // (kind, amplifier, duration)
    pub age: i32,
}

impl AreaEffectCloud {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            radius: 3.0,
            radius_on_use: -0.5,
            radius_per_tick: -0.005,
            duration: 600,
            wait_time: 10,
            reapplication_delay: 20,
            effect: None,
            age: 0,
        }
    }

    pub fn with_effect(mut self, kind: EffectKind, amplifier: u8, duration: i32) -> Self {
        self.effect = Some((kind, amplifier, duration));
        self
    }

    pub fn tick(&mut self) {
        self.age += 1;
        if self.age > self.wait_time {
            self.radius += self.radius_per_tick;
        }
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.duration || self.radius <= 0.0
    }

    pub fn affects(&self, ex: f64, ey: f64, ez: f64) -> bool {
        if self.age < self.wait_time {
            return false;
        }
        let dx = ex - self.x;
        let dy = ey - self.y;
        let dz = ez - self.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq <= (self.radius * self.radius) as f64
    }

    pub fn on_apply(&mut self) {
        self.radius += self.radius_on_use;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_expires_after_duration() {
        let mut c = AreaEffectCloud::new(0.0, 0.0, 0.0);
        c.duration = 5;
        for _ in 0..10 {
            c.tick();
        }
        assert!(c.is_expired());
    }

    #[test]
    fn cloud_within_wait_no_apply() {
        let c = AreaEffectCloud::new(0.0, 0.0, 0.0);
        assert!(!c.affects(0.0, 0.0, 0.0));
    }
}
