//! Bounding boxes / AABB math.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min_x: f64, pub min_y: f64, pub min_z: f64,
    pub max_x: f64, pub max_y: f64, pub max_z: f64,
}

impl AABB {
    pub fn new(x: f64, y: f64, z: f64, width: f64, height: f64) -> Self {
        let hw = width / 2.0;
        Self {
            min_x: x - hw, min_y: y, min_z: z - hw,
            max_x: x + hw, max_y: y + height, max_z: z + hw,
        }
    }

    pub fn expand(&mut self, dx: f64, dy: f64, dz: f64) {
        if dx > 0.0 {
            self.max_x += dx;
        } else {
            self.min_x += dx;
        }
        if dy > 0.0 {
            self.max_y += dy;
        } else {
            self.min_y += dy;
        }
        if dz > 0.0 {
            self.max_z += dz;
        } else {
            self.min_z += dz;
        }
    }

    pub fn contains_point(&self, x: f64, y: f64, z: f64) -> bool {
        x >= self.min_x && x <= self.max_x
            && y >= self.min_y && y <= self.max_y
            && z >= self.min_z && z <= self.max_z
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min_x < other.max_x && self.max_x > other.min_x
            && self.min_y < other.max_y && self.max_y > other.min_y
            && self.min_z < other.max_z && self.max_z > other.min_z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_boxes_intersect() {
        let a = AABB::new(0.0, 0.0, 0.0, 2.0, 2.0);
        let b = AABB::new(1.0, 0.0, 0.0, 2.0, 2.0);
        assert!(a.intersects(&b));
    }

    #[test]
    fn separate_boxes_dont_intersect() {
        let a = AABB::new(0.0, 0.0, 0.0, 1.0, 1.0);
        let b = AABB::new(10.0, 0.0, 0.0, 1.0, 1.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn contains_center() {
        let a = AABB::new(5.0, 0.0, 5.0, 2.0, 2.0);
        assert!(a.contains_point(5.0, 1.0, 5.0));
    }
}
