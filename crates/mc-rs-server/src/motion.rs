//! Entity motion vector.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Motion {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn is_zero(&self) -> bool {
        self.x.abs() < 1e-10 && self.y.abs() < 1e-10 && self.z.abs() < 1e-10
    }

    pub fn apply_drag(&mut self, drag_h: f64, drag_v: f64) {
        self.x *= 1.0 - drag_h;
        self.y *= 1.0 - drag_v;
        self.z *= 1.0 - drag_h;
    }

    pub fn apply_gravity(&mut self, gravity: f64) {
        self.y -= gravity;
    }

    pub fn speed_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn speed(&self) -> f64 {
        self.speed_sq().sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_detection() {
        assert!(Motion::ZERO.is_zero());
    }

    #[test]
    fn gravity_pulls_down() {
        let mut m = Motion::new(0.0, 1.0, 0.0);
        m.apply_gravity(0.08);
        assert!(m.y < 1.0);
    }
}
