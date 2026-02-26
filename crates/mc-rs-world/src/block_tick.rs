//! Block tick scheduling and random tick processing.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

use rand::prelude::*;

use crate::block_hash::TickBlocks;
use crate::fluid;
use crate::gravity;
use crate::redstone;

// ---------------------------------------------------------------------------
// Scheduled tick queue
// ---------------------------------------------------------------------------

/// A block tick scheduled for a future game tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTick {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub target_tick: u64,
    pub priority: i32,
}

impl Ord for ScheduledTick {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.target_tick
            .cmp(&other.target_tick)
            .then(self.priority.cmp(&other.priority))
    }
}

impl PartialOrd for ScheduledTick {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue for scheduled block ticks.
#[derive(Default)]
pub struct TickScheduler {
    queue: BinaryHeap<Reverse<ScheduledTick>>,
    pending: HashSet<(i32, i32, i32)>,
}

impl TickScheduler {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            pending: HashSet::new(),
        }
    }

    /// Schedule a tick at `(x, y, z)` to fire after `delay` ticks from `current_tick`.
    /// Duplicate positions are silently ignored.
    pub fn schedule(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        delay: u64,
        current_tick: u64,
        priority: i32,
    ) {
        if !self.pending.insert((x, y, z)) {
            return; // already scheduled
        }
        self.queue.push(Reverse(ScheduledTick {
            x,
            y,
            z,
            target_tick: current_tick + delay,
            priority,
        }));
    }

    /// Drain all ticks whose target_tick <= current_tick.
    pub fn drain_ready(&mut self, current_tick: u64) -> Vec<ScheduledTick> {
        let mut ready = Vec::new();
        while let Some(Reverse(ref tick)) = self.queue.peek() {
            if tick.target_tick > current_tick {
                break;
            }
            let Reverse(tick) = self.queue.pop().unwrap();
            self.pending.remove(&(tick.x, tick.y, tick.z));
            ready.push(tick);
        }
        ready
    }

    /// Check whether a tick is already scheduled for a position.
    pub fn is_scheduled(&self, x: i32, y: i32, z: i32) -> bool {
        self.pending.contains(&(x, y, z))
    }

    /// Number of pending ticks.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Scheduled tick dispatch
// ---------------------------------------------------------------------------

/// Result of processing a scheduled tick.
#[derive(Debug, Default)]
pub struct ScheduledTickResult {
    /// Block changes to apply: (x, y, z, new_runtime_id).
    pub changes: Vec<(i32, i32, i32, u32)>,
    /// New ticks to schedule: (x, y, z, delay, priority).
    pub schedule: Vec<(i32, i32, i32, u64, i32)>,
}

/// Process a scheduled tick at `(x, y, z)`. Dispatches to the appropriate
/// handler based on the block type at that position.
pub fn process_scheduled_tick(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> ScheduledTickResult {
    // Check if the block at this position is a fluid
    if let Some(rid) = get_block(x, y, z) {
        if tb.is_fluid(rid) {
            let fu = fluid::process_fluid_tick(x, y, z, tb, &get_block, &is_solid);
            return ScheduledTickResult {
                changes: fu.changes,
                schedule: fu.schedule,
            };
        }
    }

    // Gravity blocks (sand, gravel, red sand)
    if let Some(rid) = get_block(x, y, z) {
        if tb.is_gravity_block(rid) {
            let gu = gravity::process_gravity_tick(x, y, z, tb, &get_block, &is_solid);
            return ScheduledTickResult {
                changes: gu.changes,
                schedule: gu.schedule,
            };
        }
    }

    // Redstone components (torch, repeater)
    if let Some(rid) = get_block(x, y, z) {
        if tb.is_torch(rid) || tb.is_repeater(rid) {
            let ru = redstone::process_redstone_tick(x, y, z, tb, &get_block, &is_solid);
            return ScheduledTickResult {
                changes: ru.changes,
                schedule: ru.schedule,
            };
        }
    }

    ScheduledTickResult::default()
}

// ---------------------------------------------------------------------------
// Random tick processing
// ---------------------------------------------------------------------------

/// Process a random tick on a block. Returns a list of block changes (x, y, z, new_rid).
///
/// `get_block` returns the runtime ID at world coordinates, or None if unloaded.
/// `is_solid` returns whether a runtime ID is a solid block.
pub fn process_random_tick(
    runtime_id: u32,
    wx: i32,
    wy: i32,
    wz: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> Vec<(i32, i32, i32, u32)> {
    // Grass block: if solid block above, turn to dirt
    if runtime_id == tb.grass_block {
        if let Some(above) = get_block(wx, wy + 1, wz) {
            if is_solid(above) {
                return vec![(wx, wy, wz, tb.dirt)];
            }
        }
        // Grass spread: pick one random horizontal neighbor
        return try_grass_spread(wx, wy, wz, tb, &get_block, &is_solid);
    }

    // Crops: increment growth stage
    if let Some((crop, growth)) = tb.crop_growth(runtime_id) {
        let max = TickBlocks::crop_max_growth(crop);
        if growth < max {
            // Require farmland below
            if let Some(below) = get_block(wx, wy - 1, wz) {
                let is_farmland = tb.farmland.contains(&below);
                if is_farmland {
                    return vec![(wx, wy, wz, tb.crop_at_growth(crop, growth + 1))];
                }
            }
        }
        return Vec::new();
    }

    // Leaf decay: check for nearby logs within Manhattan distance 4
    if tb.is_leaf(runtime_id) {
        if !has_log_nearby(wx, wy, wz, 4, tb, &get_block) {
            return vec![(wx, wy, wz, tb.air)];
        }
        return Vec::new();
    }

    Vec::new()
}

/// Try to spread grass to a random adjacent dirt block.
fn try_grass_spread(
    wx: i32,
    wy: i32,
    wz: i32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: &impl Fn(u32) -> bool,
) -> Vec<(i32, i32, i32, u32)> {
    // Pick one of the 4 horizontal + 2 vertical neighbors at random
    let neighbors = [
        (wx - 1, wy, wz),
        (wx + 1, wy, wz),
        (wx, wy, wz - 1),
        (wx, wy, wz + 1),
        (wx - 1, wy - 1, wz),
        (wx + 1, wy - 1, wz),
        (wx, wy - 1, wz - 1),
        (wx, wy - 1, wz + 1),
        (wx - 1, wy + 1, wz),
        (wx + 1, wy + 1, wz),
        (wx, wy + 1, wz - 1),
        (wx, wy + 1, wz + 1),
    ];

    let mut rng = thread_rng();
    let idx = rng.gen_range(0..neighbors.len());
    let (nx, ny, nz) = neighbors[idx];

    // Target must be dirt
    if let Some(nrid) = get_block(nx, ny, nz) {
        if nrid != tb.dirt {
            return Vec::new();
        }
        // Must not have a solid block 2 above the target (simplified light check)
        if let Some(above) = get_block(nx, ny + 1, nz) {
            if is_solid(above) {
                return Vec::new();
            }
        }
        return vec![(nx, ny, nz, tb.grass_block)];
    }
    Vec::new()
}

/// Check if there is a log block within Manhattan distance `radius` of (wx, wy, wz).
fn has_log_nearby(
    wx: i32,
    wy: i32,
    wz: i32,
    radius: i32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
) -> bool {
    for dx in -radius..=radius {
        let rem = radius - dx.abs();
        for dy in -rem..=rem {
            let rem2 = rem - dy.abs();
            for dz in -rem2..=rem2 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                if let Some(rid) = get_block(wx + dx, wy + dy, wz + dz) {
                    if tb.is_log(rid) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_basic() {
        let mut s = TickScheduler::new();
        s.schedule(0, 0, 0, 5, 100, 0);
        s.schedule(1, 0, 0, 10, 100, 0);
        assert_eq!(s.len(), 2);

        // At tick 104: nothing ready
        let ready = s.drain_ready(104);
        assert!(ready.is_empty());

        // At tick 105: first tick ready
        let ready = s.drain_ready(105);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].x, 0);

        // At tick 110: second tick ready
        let ready = s.drain_ready(110);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].x, 1);
        assert!(s.is_empty());
    }

    #[test]
    fn scheduler_dedup() {
        let mut s = TickScheduler::new();
        s.schedule(5, 10, 15, 1, 0, 0);
        s.schedule(5, 10, 15, 2, 0, 0); // duplicate position
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn scheduler_priority_ordering() {
        let mut s = TickScheduler::new();
        s.schedule(0, 0, 0, 5, 0, 10); // low priority (high number)
        s.schedule(1, 0, 0, 5, 0, 1); // high priority (low number)
        let ready = s.drain_ready(5);
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].x, 1); // high priority first
        assert_eq!(ready[1].x, 0);
    }

    #[test]
    fn scheduler_is_scheduled() {
        let mut s = TickScheduler::new();
        assert!(!s.is_scheduled(0, 0, 0));
        s.schedule(0, 0, 0, 5, 0, 0);
        assert!(s.is_scheduled(0, 0, 0));
        s.drain_ready(5);
        assert!(!s.is_scheduled(0, 0, 0));
    }

    fn make_tick_blocks() -> TickBlocks {
        TickBlocks::compute()
    }

    #[test]
    fn random_tick_grass_covered_becomes_dirt() {
        let tb = make_tick_blocks();
        // Grass block with solid block above -> dirt
        let changes = process_random_tick(
            tb.grass_block,
            0,
            64,
            0,
            &tb,
            |_x, y, _z| {
                if y == 65 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );
        assert_eq!(changes, vec![(0, 64, 0, tb.dirt)]);
    }

    #[test]
    fn random_tick_grass_uncovered_no_change_to_dirt() {
        let tb = make_tick_blocks();
        // Grass block with air above -> may spread, but never becomes dirt itself
        let changes = process_random_tick(
            tb.grass_block,
            0,
            64,
            0,
            &tb,
            |_, _, _| Some(tb.air),
            |_| false,
        );
        // Should not contain the original position becoming dirt
        for (x, y, z, rid) in &changes {
            if *x == 0 && *y == 64 && *z == 0 {
                assert_ne!(*rid, tb.dirt, "grass should not become dirt without cover");
            }
        }
    }

    #[test]
    fn random_tick_crop_grows() {
        let tb = make_tick_blocks();
        // Wheat at growth 3 with farmland below
        let changes = process_random_tick(
            tb.wheat[3],
            0,
            65,
            0,
            &tb,
            |_x, y, _z| {
                if y == 64 {
                    Some(tb.farmland[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );
        assert_eq!(changes, vec![(0, 65, 0, tb.wheat[4])]);
    }

    #[test]
    fn random_tick_crop_max_growth_no_change() {
        let tb = make_tick_blocks();
        // Wheat at max growth (7)
        let changes = process_random_tick(
            tb.wheat[7],
            0,
            65,
            0,
            &tb,
            |_x, y, _z| {
                if y == 64 {
                    Some(tb.farmland[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );
        assert!(changes.is_empty());
    }

    #[test]
    fn random_tick_crop_no_farmland_no_change() {
        let tb = make_tick_blocks();
        // Wheat at growth 0 with dirt below (not farmland)
        let changes = process_random_tick(
            tb.wheat[0],
            0,
            65,
            0,
            &tb,
            |_x, y, _z| {
                if y == 64 {
                    Some(tb.dirt)
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );
        assert!(changes.is_empty());
    }

    #[test]
    fn random_tick_beetroot_grows() {
        let tb = make_tick_blocks();
        let changes = process_random_tick(
            tb.beetroot[1],
            0,
            65,
            0,
            &tb,
            |_x, y, _z| {
                if y == 64 {
                    Some(tb.farmland[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );
        assert_eq!(changes, vec![(0, 65, 0, tb.beetroot[2])]);
    }

    #[test]
    fn random_tick_leaf_decay() {
        let tb = make_tick_blocks();
        // Oak leaves with no logs nearby -> decay to air
        let changes = process_random_tick(
            tb.oak_leaves,
            0,
            70,
            0,
            &tb,
            |_, _, _| Some(tb.air), // no logs anywhere
            |_| false,
        );
        assert_eq!(changes, vec![(0, 70, 0, tb.air)]);
    }

    #[test]
    fn random_tick_leaf_near_log_survives() {
        let tb = make_tick_blocks();
        // Oak leaves with a log 2 blocks away -> survives
        let changes = process_random_tick(
            tb.oak_leaves,
            0,
            70,
            0,
            &tb,
            |x, y, _z| {
                if x == 2 && y == 70 {
                    Some(tb.oak_log)
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );
        assert!(changes.is_empty());
    }

    #[test]
    fn random_tick_unrelated_block_no_change() {
        let tb = make_tick_blocks();
        let changes =
            process_random_tick(tb.stone, 0, 64, 0, &tb, |_, _, _| Some(tb.air), |_| false);
        assert!(changes.is_empty());
    }
}
