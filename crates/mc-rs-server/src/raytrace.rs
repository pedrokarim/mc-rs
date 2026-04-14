//! Ray tracing — block / entity hit detection.

use crate::vector_math::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitFace {
    Down, Up, North, South, West, East,
}

#[derive(Debug, Clone)]
pub struct HitResult {
    pub block_x: i32,
    pub block_y: i32,
    pub block_z: i32,
    pub face: HitFace,
    pub distance: f64,
}

/// Fast voxel traversal (Amanatides & Woo 1987).
pub fn cast_ray(
    origin: Vec3,
    direction: Vec3,
    max_distance: f64,
    is_solid: impl Fn(i32, i32, i32) -> bool,
) -> Option<HitResult> {
    let dir = direction.normalize();

    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;

    let step_x = dir.x.signum() as i32;
    let step_y = dir.y.signum() as i32;
    let step_z = dir.z.signum() as i32;

    let t_delta_x = (1.0 / dir.x.abs()).min(1e9);
    let t_delta_y = (1.0 / dir.y.abs()).min(1e9);
    let t_delta_z = (1.0 / dir.z.abs()).min(1e9);

    let next_boundary_x = if step_x > 0 { x as f64 + 1.0 } else { x as f64 };
    let next_boundary_y = if step_y > 0 { y as f64 + 1.0 } else { y as f64 };
    let next_boundary_z = if step_z > 0 { z as f64 + 1.0 } else { z as f64 };

    let mut t_max_x = if dir.x != 0.0 { (next_boundary_x - origin.x) / dir.x } else { f64::INFINITY };
    let mut t_max_y = if dir.y != 0.0 { (next_boundary_y - origin.y) / dir.y } else { f64::INFINITY };
    let mut t_max_z = if dir.z != 0.0 { (next_boundary_z - origin.z) / dir.z } else { f64::INFINITY };

    let mut distance = 0.0;
    let mut last_face = HitFace::Up;

    while distance < max_distance {
        if is_solid(x, y, z) {
            return Some(HitResult {
                block_x: x,
                block_y: y,
                block_z: z,
                face: last_face,
                distance,
            });
        }
        if t_max_x < t_max_y && t_max_x < t_max_z {
            x += step_x;
            distance = t_max_x;
            t_max_x += t_delta_x;
            last_face = if step_x > 0 { HitFace::West } else { HitFace::East };
        } else if t_max_y < t_max_z {
            y += step_y;
            distance = t_max_y;
            t_max_y += t_delta_y;
            last_face = if step_y > 0 { HitFace::Down } else { HitFace::Up };
        } else {
            z += step_z;
            distance = t_max_z;
            t_max_z += t_delta_z;
            last_face = if step_z > 0 { HitFace::North } else { HitFace::South };
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_block_in_path() {
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0);
        let hit = cast_ray(origin, direction, 10.0, |x, _, _| x == 3);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().block_x, 3);
    }

    #[test]
    fn misses_beyond_distance() {
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0);
        let hit = cast_ray(origin, direction, 2.0, |x, _, _| x == 10);
        assert!(hit.is_none());
    }
}
