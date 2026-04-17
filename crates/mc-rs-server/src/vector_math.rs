//! Vector math utilities.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn distance(&self, other: &Vec3) -> f64 {
        self.subtract(other).length()
    }

    pub fn distance_sq(&self, other: &Vec3) -> f64 {
        self.subtract(other).length_sq()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            return Self::ZERO;
        }
        Self {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }

    pub fn dot(&self, other: &Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn add(&self, other: &Vec3) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn subtract(&self, other: &Vec3) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    pub fn scale(&self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    /// Yaw/pitch to direction vector.
    pub fn from_yaw_pitch(yaw: f64, pitch: f64) -> Self {
        let yaw_rad = yaw.to_radians();
        let pitch_rad = pitch.to_radians();
        let cos_p = pitch_rad.cos();
        Self {
            x: -yaw_rad.sin() * cos_p,
            y: -pitch_rad.sin(),
            z: yaw_rad.cos() * cos_p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_unit_length() {
        let v = Vec3::new(5.0, 0.0, 0.0).normalize();
        assert!((v.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cross_perpendicular() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(&b);
        assert!((c.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn distance_correct() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 0.0, 4.0);
        assert_eq!(a.distance(&b), 5.0);
    }
}
