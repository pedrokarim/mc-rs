//! Player physics constants and AABB collision helpers.

/// Player hitbox width (Bedrock: 0.6 blocks).
pub const PLAYER_WIDTH: f32 = 0.6;

/// Player hitbox height when standing (Bedrock: 1.8 blocks).
pub const PLAYER_HEIGHT: f32 = 1.8;

/// Eye offset above feet (Bedrock: 1.62 blocks).
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;

/// Maximum vertical distance per tick (terminal velocity + margin).
pub const MAX_FALL_PER_TICK: f32 = 4.0;

/// Maximum airborne ticks before anti-fly kicks in (survival, ~4 seconds).
pub const MAX_AIRBORNE_TICKS: u32 = 80;

/// Half the player width, used for AABB calculations.
const HALF_WIDTH: f32 = PLAYER_WIDTH / 2.0;

/// Axis-aligned bounding box for a player.
#[derive(Debug, Clone, Copy)]
pub struct PlayerAabb {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl PlayerAabb {
    /// Build the AABB from an eye position (Bedrock convention: position.y = eyes).
    pub fn from_eye_position(x: f32, y: f32, z: f32) -> Self {
        let feet_y = y - PLAYER_EYE_HEIGHT;
        Self {
            min_x: x - HALF_WIDTH,
            max_x: x + HALF_WIDTH,
            min_y: feet_y,
            max_y: feet_y + PLAYER_HEIGHT,
            min_z: z - HALF_WIDTH,
            max_z: z + HALF_WIDTH,
        }
    }

    /// Iterate all block positions that intersect this AABB.
    ///
    /// A small epsilon (0.001) is subtracted from max bounds so that a player
    /// standing exactly on the edge of a block does not collide with the next one.
    pub fn intersecting_blocks(&self) -> impl Iterator<Item = (i32, i32, i32)> {
        const EPS: f32 = 0.001;
        let bx_min = self.min_x.floor() as i32;
        let bx_max = (self.max_x - EPS).floor() as i32;
        let by_min = self.min_y.floor() as i32;
        let by_max = (self.max_y - EPS).floor() as i32;
        let bz_min = self.min_z.floor() as i32;
        let bz_max = (self.max_z - EPS).floor() as i32;

        let mut results = Vec::new();
        for bx in bx_min..=bx_max {
            for by in by_min..=by_max {
                for bz in bz_min..=bz_max {
                    results.push((bx, by, bz));
                }
            }
        }
        results.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_dimensions() {
        assert!((PLAYER_WIDTH - 0.6).abs() < f32::EPSILON);
        assert!((PLAYER_HEIGHT - 1.8).abs() < f32::EPSILON);
        assert!((PLAYER_EYE_HEIGHT - 1.62).abs() < 0.001);
    }

    #[test]
    fn max_fall_exceeds_terminal_velocity() {
        // Bedrock terminal velocity ≈ 3.92 blocks/tick
        // MAX_FALL_PER_TICK must be larger to avoid false positives
        let terminal_velocity: f32 = 3.92;
        assert!(MAX_FALL_PER_TICK > terminal_velocity);
    }

    #[test]
    fn aabb_from_eye_at_spawn() {
        // Spawn eye position: (0.5, 5.62, 0.5), feet at Y=4.0
        let aabb = PlayerAabb::from_eye_position(0.5, 5.62, 0.5);
        assert!((aabb.min_x - 0.2).abs() < 0.001);
        assert!((aabb.max_x - 0.8).abs() < 0.001);
        assert!((aabb.min_y - 4.0).abs() < 0.001);
        assert!((aabb.max_y - 5.8).abs() < 0.001);
        assert!((aabb.min_z - 0.2).abs() < 0.001);
        assert!((aabb.max_z - 0.8).abs() < 0.001);
    }

    #[test]
    fn intersecting_blocks_center_of_block() {
        // Player centered in block (0,4,0) — eye at (0.5, 5.62, 0.5), feet at 4.0
        let aabb = PlayerAabb::from_eye_position(0.5, 5.62, 0.5);
        let blocks: Vec<_> = aabb.intersecting_blocks().collect();
        // Feet at Y=4.0, head at Y=5.8
        // X: 0.2..0.8 → block 0
        // Y: 4.0..5.8 → blocks 4, 5 (5.8 - eps → 5)
        // Z: 0.2..0.8 → block 0
        assert!(blocks.contains(&(0, 4, 0)));
        assert!(blocks.contains(&(0, 5, 0)));
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn intersecting_blocks_straddling_x() {
        // Player at X boundary: x=1.0 → AABB x: 0.7..1.3 → blocks 0 and 1
        let aabb = PlayerAabb::from_eye_position(1.0, 5.62, 0.5);
        let blocks: Vec<_> = aabb.intersecting_blocks().collect();
        let xs: std::collections::HashSet<i32> = blocks.iter().map(|b| b.0).collect();
        assert!(xs.contains(&0));
        assert!(xs.contains(&1));
    }

    #[test]
    fn intersecting_blocks_negative_coords() {
        // Player at negative coordinates: (-0.5, 5.62, -0.5)
        let aabb = PlayerAabb::from_eye_position(-0.5, 5.62, -0.5);
        let blocks: Vec<_> = aabb.intersecting_blocks().collect();
        // X: -0.8..-0.2 → block -1
        // Z: -0.8..-0.2 → block -1
        assert!(blocks.iter().all(|b| b.0 == -1 && b.2 == -1));
    }

    #[test]
    fn exact_block_edge_no_extra_collision() {
        // Player exactly at x=0.3, AABB: 0.0..0.6 → max - eps = 0.599 → floor = 0
        // Should only intersect block X=0, not X=1
        let aabb = PlayerAabb::from_eye_position(0.3, 5.62, 0.3);
        let blocks: Vec<_> = aabb.intersecting_blocks().collect();
        assert!(blocks.iter().all(|b| b.0 == 0 && b.2 == 0));
    }
}
