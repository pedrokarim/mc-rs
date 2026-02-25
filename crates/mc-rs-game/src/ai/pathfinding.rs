//! Simplified pathfinding for flat world (direct movement, no obstacles).

/// Move toward a target position on a flat world.
///
/// Returns `(vx, vz)` — the velocity to apply this tick to move toward `(goal_x, goal_z)`
/// at the given `speed` (blocks/tick).
pub fn move_toward_flat(
    current_x: f32,
    current_z: f32,
    goal_x: f32,
    goal_z: f32,
    speed: f32,
) -> (f32, f32) {
    let dx = goal_x - current_x;
    let dz = goal_z - current_z;
    let dist = (dx * dx + dz * dz).sqrt();

    if dist < 0.1 {
        return (0.0, 0.0);
    }

    let norm_x = dx / dist;
    let norm_z = dz / dist;
    (norm_x * speed, norm_z * speed)
}

/// Compute the yaw angle (0..360 degrees) from one position facing another.
///
/// Convention: 0 = south (+Z), 90 = west (-X), 180 = north (-Z), 270 = east (+X).
/// This matches Minecraft Bedrock's yaw convention.
pub fn yaw_toward(from_x: f32, from_z: f32, to_x: f32, to_z: f32) -> f32 {
    let dx = to_x - from_x;
    let dz = to_z - from_z;
    let yaw = (-dx).atan2(dz).to_degrees();
    ((yaw % 360.0) + 360.0) % 360.0
}

/// Placeholder for future A* pathfinding.
///
/// For flat world with no obstacles, returns a direct path (single waypoint).
pub fn find_path(
    _start: (f32, f32, f32),
    goal: (f32, f32, f32),
    _max_distance: f32,
) -> Vec<(f32, f32, f32)> {
    vec![goal]
}

/// Distance between two positions in the XZ plane.
pub fn distance_xz(x1: f32, z1: f32, x2: f32, z2: f32) -> f32 {
    let dx = x2 - x1;
    let dz = z2 - z1;
    (dx * dx + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_toward_north() {
        // Moving toward +Z (south in MC, but positive Z direction)
        let (vx, vz) = move_toward_flat(0.0, 0.0, 0.0, 10.0, 0.25);
        assert!(vx.abs() < 0.001);
        assert!((vz - 0.25).abs() < 0.001);
    }

    #[test]
    fn move_toward_east() {
        let (vx, vz) = move_toward_flat(0.0, 0.0, 10.0, 0.0, 0.25);
        assert!((vx - 0.25).abs() < 0.001);
        assert!(vz.abs() < 0.001);
    }

    #[test]
    fn at_goal_returns_zero() {
        let (vx, vz) = move_toward_flat(5.0, 5.0, 5.0, 5.0, 0.25);
        assert!(vx.abs() < 0.001);
        assert!(vz.abs() < 0.001);
    }

    #[test]
    fn speed_magnitude() {
        let (vx, vz) = move_toward_flat(0.0, 0.0, 3.0, 4.0, 0.5);
        let magnitude = (vx * vx + vz * vz).sqrt();
        assert!((magnitude - 0.5).abs() < 0.001);
    }

    #[test]
    fn yaw_south() {
        // Facing +Z = yaw 0 (south)
        let yaw = yaw_toward(0.0, 0.0, 0.0, 10.0);
        assert!((yaw - 0.0).abs() < 0.1 || (yaw - 360.0).abs() < 0.1);
    }

    #[test]
    fn yaw_west() {
        // Facing -X = yaw 90 (west)
        let yaw = yaw_toward(0.0, 0.0, -10.0, 0.0);
        assert!((yaw - 90.0).abs() < 0.1);
    }

    #[test]
    fn yaw_north() {
        // Facing -Z = yaw 180 (north)
        let yaw = yaw_toward(0.0, 0.0, 0.0, -10.0);
        assert!((yaw - 180.0).abs() < 0.1);
    }

    #[test]
    fn yaw_east() {
        // Facing +X = yaw 270 (east)
        let yaw = yaw_toward(0.0, 0.0, 10.0, 0.0);
        assert!((yaw - 270.0).abs() < 0.1);
    }

    #[test]
    fn distance_xz_basic() {
        assert!((distance_xz(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 0.001);
        assert!((distance_xz(1.0, 1.0, 1.0, 1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn find_path_stub() {
        let path = find_path((0.0, 4.0, 0.0), (10.0, 4.0, 10.0), 50.0);
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], (10.0, 4.0, 10.0));
    }
}
