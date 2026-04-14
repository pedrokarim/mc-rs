//! Position/coordinate utilities.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: i32,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x, y, z,
            yaw: 0.0, pitch: 0.0,
            dimension: 0,
        }
    }

    /// Block position (floor).
    pub fn to_block(&self) -> (i32, i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
    }

    /// Chunk position (floor div 16).
    pub fn to_chunk(&self) -> (i32, i32) {
        (
            (self.x.floor() as i32).div_euclid(16),
            (self.z.floor() as i32).div_euclid(16),
        )
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_sq(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_floor() {
        let p = Position::new(15.5, 64.7, -8.2);
        assert_eq!(p.to_block(), (15, 64, -9));
    }

    #[test]
    fn chunk_conversion() {
        let p = Position::new(0.0, 0.0, 0.0);
        assert_eq!(p.to_chunk(), (0, 0));
        let p2 = Position::new(-1.0, 0.0, 0.0);
        assert_eq!(p2.to_chunk(), (-1, 0));
    }
}
