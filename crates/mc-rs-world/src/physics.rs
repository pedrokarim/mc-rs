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

// -----------------------------------------------------------------------
// Anti-cheat constants
// -----------------------------------------------------------------------

/// Maximum distance (blocks) a player can reach to interact with blocks.
pub const BLOCK_REACH: f32 = 7.0;

/// Maximum distance (blocks) for PvP/PvE attacks.
pub const ATTACK_REACH: f32 = 6.0;

/// Minimum ticks between consecutive block breaks.
pub const MIN_BREAK_INTERVAL: u64 = 2;

/// Minimum ticks between consecutive block placements.
pub const MIN_PLACE_INTERVAL: u64 = 2;

/// Minimum ticks between consecutive attacks.
pub const MIN_ATTACK_INTERVAL: u64 = 2;

/// Minimum ticks between consecutive commands.
pub const MIN_COMMAND_INTERVAL: u64 = 10;

/// Maximum total actions per second (all categories).
pub const MAX_ACTIONS_PER_SECOND: u16 = 30;

/// Number of speed violations before auto-kick.
pub const SPEED_KICK_THRESHOLD: u32 = 10;

/// Number of fly violations before auto-kick.
pub const FLY_KICK_THRESHOLD: u32 = 5;

/// Number of no-clip violations before auto-kick.
pub const NOCLIP_KICK_THRESHOLD: u32 = 5;

/// Number of reach violations before auto-kick.
pub const REACH_KICK_THRESHOLD: u32 = 20;

/// Number of rate-limit violations before auto-kick.
pub const RATE_LIMIT_KICK_THRESHOLD: u32 = 50;

/// Ticks between violation decay passes (10 seconds).
pub const VIOLATION_DECAY_INTERVAL: u64 = 200;

/// Airborne ticks before forced fly-kick regardless of velocity (10 seconds).
pub const MAX_AIRBORNE_KICK: u32 = 200;

/// Tracks anti-cheat violation counts per category.
#[derive(Debug, Clone, Default)]
pub struct ViolationTracker {
    pub speed: u32,
    pub fly: u32,
    pub noclip: u32,
    pub reach: u32,
    pub rate_limit: u32,
}

impl ViolationTracker {
    /// Decrement each violation counter by 1 (min 0). Called periodically.
    pub fn decay(&mut self) {
        self.speed = self.speed.saturating_sub(1);
        self.fly = self.fly.saturating_sub(1);
        self.noclip = self.noclip.saturating_sub(1);
        self.reach = self.reach.saturating_sub(1);
        self.rate_limit = self.rate_limit.saturating_sub(1);
    }

    /// Check if any category has exceeded its kick threshold.
    /// Returns the violation reason if the player should be kicked.
    pub fn should_kick(&self) -> Option<&'static str> {
        if self.speed >= SPEED_KICK_THRESHOLD {
            return Some("Speed hack detected");
        }
        if self.fly >= FLY_KICK_THRESHOLD {
            return Some("Fly hack detected");
        }
        if self.noclip >= NOCLIP_KICK_THRESHOLD {
            return Some("No-clip detected");
        }
        if self.reach >= REACH_KICK_THRESHOLD {
            return Some("Reach hack detected");
        }
        if self.rate_limit >= RATE_LIMIT_KICK_THRESHOLD {
            return Some("Too many actions");
        }
        None
    }
}

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

    #[test]
    fn anti_cheat_constants() {
        assert!((BLOCK_REACH - 7.0).abs() < f32::EPSILON);
        assert!((ATTACK_REACH - 6.0).abs() < f32::EPSILON);
        assert_eq!(MIN_BREAK_INTERVAL, 2);
        assert_eq!(MIN_PLACE_INTERVAL, 2);
        assert_eq!(MIN_ATTACK_INTERVAL, 2);
        assert_eq!(MIN_COMMAND_INTERVAL, 10);
        assert_eq!(MAX_ACTIONS_PER_SECOND, 30);
        assert_eq!(VIOLATION_DECAY_INTERVAL, 200);
        assert_eq!(MAX_AIRBORNE_KICK, 200);
    }

    #[test]
    fn violation_tracker_default() {
        let tracker = ViolationTracker::default();
        assert_eq!(tracker.speed, 0);
        assert_eq!(tracker.fly, 0);
        assert_eq!(tracker.noclip, 0);
        assert_eq!(tracker.reach, 0);
        assert_eq!(tracker.rate_limit, 0);
        assert!(tracker.should_kick().is_none());
    }

    #[test]
    fn violation_tracker_decay() {
        let mut tracker = ViolationTracker {
            speed: 3,
            fly: 1,
            noclip: 0,
            reach: 5,
            rate_limit: 2,
        };
        tracker.decay();
        assert_eq!(tracker.speed, 2);
        assert_eq!(tracker.fly, 0);
        assert_eq!(tracker.noclip, 0); // stays at 0
        assert_eq!(tracker.reach, 4);
        assert_eq!(tracker.rate_limit, 1);
    }

    #[test]
    fn violation_tracker_should_kick_speed() {
        let mut tracker = ViolationTracker::default();
        tracker.speed = SPEED_KICK_THRESHOLD;
        assert_eq!(tracker.should_kick(), Some("Speed hack detected"));
    }

    #[test]
    fn violation_tracker_should_kick_fly() {
        let mut tracker = ViolationTracker::default();
        tracker.fly = FLY_KICK_THRESHOLD;
        assert_eq!(tracker.should_kick(), Some("Fly hack detected"));
    }

    #[test]
    fn violation_tracker_should_kick_noclip() {
        let mut tracker = ViolationTracker::default();
        tracker.noclip = NOCLIP_KICK_THRESHOLD;
        assert_eq!(tracker.should_kick(), Some("No-clip detected"));
    }

    #[test]
    fn violation_tracker_should_kick_reach() {
        let mut tracker = ViolationTracker::default();
        tracker.reach = REACH_KICK_THRESHOLD;
        assert_eq!(tracker.should_kick(), Some("Reach hack detected"));
    }

    #[test]
    fn violation_tracker_should_kick_rate_limit() {
        let mut tracker = ViolationTracker::default();
        tracker.rate_limit = RATE_LIMIT_KICK_THRESHOLD;
        assert_eq!(tracker.should_kick(), Some("Too many actions"));
    }

    #[test]
    fn violation_tracker_below_threshold_no_kick() {
        let tracker = ViolationTracker {
            speed: SPEED_KICK_THRESHOLD - 1,
            fly: FLY_KICK_THRESHOLD - 1,
            noclip: NOCLIP_KICK_THRESHOLD - 1,
            reach: REACH_KICK_THRESHOLD - 1,
            rate_limit: RATE_LIMIT_KICK_THRESHOLD - 1,
        };
        assert!(tracker.should_kick().is_none());
    }
}
