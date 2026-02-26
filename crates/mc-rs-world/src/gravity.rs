//! Gravity block logic for sand, gravel, and red sand.
//!
//! When a gravity block has no solid support below, it falls one block per tick
//! until it lands on a solid surface or reaches the world bottom.

use crate::block_hash::TickBlocks;

/// Tick delay for gravity blocks (2 game ticks = 100 ms, fast falling).
pub const GRAVITY_TICK_DELAY: u64 = 2;

/// Result of processing a gravity tick.
#[derive(Debug, Default)]
pub struct GravityUpdate {
    /// Block changes to apply: (x, y, z, new_runtime_id).
    pub changes: Vec<(i32, i32, i32, u32)>,
    /// New ticks to schedule: (x, y, z, delay, priority).
    pub schedule: Vec<(i32, i32, i32, u64, i32)>,
}

/// Process a scheduled gravity tick at `(x, y, z)`.
///
/// If the block is a gravity block and has air, fluid, or non-solid below,
/// it moves down one position and schedules another tick.
pub fn process_gravity_tick(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> GravityUpdate {
    let mut update = GravityUpdate::default();

    let rid = match get_block(x, y, z) {
        Some(r) => r,
        None => return update,
    };

    if !tb.is_gravity_block(rid) {
        return update;
    }

    // Check block below
    let below_rid = match get_block(x, y - 1, z) {
        Some(r) => r,
        None => return update, // at world bottom or unloaded
    };

    // Can fall if below is air, fluid, or non-solid (not another gravity block resting)
    if below_rid == tb.air || tb.is_fluid(below_rid) || !is_solid(below_rid) {
        // Move block down: remove from current position, place at y-1
        update.changes.push((x, y, z, tb.air));
        update.changes.push((x, y - 1, z, rid));
        // Schedule another tick at the new position to continue falling
        update.schedule.push((x, y - 1, z, GRAVITY_TICK_DELAY, 0));
    }

    update
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tb() -> TickBlocks {
        TickBlocks::compute()
    }

    #[test]
    fn sand_falls_in_air() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.sand)
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        // Should remove sand from y=10 and place at y=9
        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.air));
        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.sand));
        // Should schedule tick at y=9
        assert!(update
            .schedule
            .iter()
            .any(|&(x, y, z, _, _)| x == 0 && y == 9 && z == 0));
    }

    #[test]
    fn sand_stops_on_solid() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.sand)
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        // No changes — sand is resting on stone
        assert!(update.changes.is_empty());
        assert!(update.schedule.is_empty());
    }

    #[test]
    fn sand_falls_into_water() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.sand)
                } else if x == 0 && y == 9 && z == 0 {
                    Some(tb.water[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        // Sand replaces water
        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.air));
        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.sand));
    }

    #[test]
    fn gravel_falls() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.gravel)
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.gravel));
    }

    #[test]
    fn red_sand_falls() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.red_sand)
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.red_sand));
    }

    #[test]
    fn non_gravity_block_returns_empty() {
        let tb = make_tb();
        let update = process_gravity_tick(
            0,
            10,
            0,
            &tb,
            |_, _, _| Some(tb.stone),
            |rid| rid == tb.stone,
        );

        assert!(update.changes.is_empty());
        assert!(update.schedule.is_empty());
    }
}
