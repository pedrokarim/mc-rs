//! Fluid flow logic for water and lava.
//!
//! Pure functions — no networking or async. The caller (connection.rs) applies
//! the returned block changes and schedules new ticks.

use crate::block_hash::{FluidType, TickBlocks};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of processing a fluid tick at one position.
#[derive(Debug, Default)]
pub struct FluidUpdate {
    /// Block changes to apply: (x, y, z, new_runtime_id).
    pub changes: Vec<(i32, i32, i32, u32)>,
    /// New ticks to schedule: (x, y, z, delay, priority).
    pub schedule: Vec<(i32, i32, i32, u64, i32)>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Tick delay for water flow (5 game ticks = 250 ms).
const WATER_TICK_DELAY: u64 = 5;
/// Tick delay for lava flow in the overworld (30 game ticks = 1.5 s).
const LAVA_TICK_DELAY: u64 = 30;
/// Maximum horizontal spread distance for water (depth 1-7).
const WATER_MAX_DEPTH: u8 = 7;
/// Maximum horizontal spread distance for lava (depth 1-4).
const LAVA_MAX_DEPTH: u8 = 4;

/// Falling fluid uses liquid_depth = 8.
const FALLING_DEPTH: u8 = 8;

/// Get the tick delay for a fluid type.
pub fn tick_delay(fluid: FluidType) -> u64 {
    match fluid {
        FluidType::Water => WATER_TICK_DELAY,
        FluidType::Lava => LAVA_TICK_DELAY,
    }
}

/// Get the maximum horizontal spread depth for a fluid type.
fn max_depth(fluid: FluidType) -> u8 {
    match fluid {
        FluidType::Water => WATER_MAX_DEPTH,
        FluidType::Lava => LAVA_MAX_DEPTH,
    }
}

// ---------------------------------------------------------------------------
// Internal context to avoid passing many arguments
// ---------------------------------------------------------------------------

/// Bundles shared state for fluid tick processing.
struct FluidCtx<'a, G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool> {
    tb: &'a TickBlocks,
    get_block: G,
    is_solid: S,
    fluid: FluidType,
    delay: u64,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Process a scheduled fluid tick at `(x, y, z)`.
///
/// Reads the block at the position, determines the fluid type and depth,
/// then computes how the fluid should flow or dry up.
pub fn process_fluid_tick(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> FluidUpdate {
    let rid = match get_block(x, y, z) {
        Some(r) => r,
        None => return FluidUpdate::default(),
    };

    // Determine fluid type and depth
    let (fluid, depth) = if let Some(d) = tb.water_depth(rid) {
        (FluidType::Water, d)
    } else if let Some(d) = tb.lava_depth(rid) {
        (FluidType::Lava, d)
    } else {
        return FluidUpdate::default();
    };

    let ctx = FluidCtx {
        tb,
        get_block,
        is_solid,
        fluid,
        delay: tick_delay(fluid),
    };

    let mut update = FluidUpdate::default();

    if depth == 0 {
        process_source(&ctx, x, y, z, &mut update);
    } else if depth == FALLING_DEPTH {
        process_falling(&ctx, x, y, z, &mut update);
    } else {
        process_flowing(&ctx, x, y, z, depth, &mut update);
    }

    // Check fluid interactions for all changes
    let interactions: Vec<_> = update
        .changes
        .iter()
        .filter_map(|&(cx, cy, cz, new_rid)| {
            check_fluid_interaction(cx, cy, cz, new_rid, ctx.tb, &ctx.get_block)
        })
        .collect();
    for change in interactions {
        update.changes.push(change);
    }

    update
}

// ---------------------------------------------------------------------------
// Core fluid algorithm
// ---------------------------------------------------------------------------

/// Process a source block (depth 0). Sources never dry up.
fn process_source<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
    update: &mut FluidUpdate,
) {
    // Try to flow downward first
    if try_flow_down(ctx, x, y, z, update) {
        return;
    }
    // Can't go down → spread horizontally with depth 1
    spread_horizontal(ctx, x, y, z, 1, update);
}

/// Process a falling block (depth 8).
fn process_falling<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
    update: &mut FluidUpdate,
) {
    // Check if there is still a source or falling block feeding us from above
    if !has_feeder_above(ctx, x, y, z) {
        // No feeder above → dry up
        update.changes.push((x, y, z, ctx.tb.air));
        schedule_neighbors(x, y, z, ctx.delay, update);
        return;
    }

    // Try to continue falling
    if try_flow_down(ctx, x, y, z, update) {
        return;
    }

    // Hit solid below → spread horizontally
    spread_horizontal(ctx, x, y, z, 1, update);
}

/// Process a flowing block (depth 1-7).
fn process_flowing<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
    depth: u8,
    update: &mut FluidUpdate,
) {
    // Check infinite water source: 2+ horizontal water sources → become source
    if ctx.fluid == FluidType::Water
        && check_infinite_water_source(x, y, z, ctx.tb, &ctx.get_block)
    {
        update
            .changes
            .push((x, y, z, fluid_rid(ctx.fluid, 0, ctx.tb)));
        // Re-schedule self as source to propagate further
        update.schedule.push((x, y, z, ctx.delay, 0));
        return;
    }

    // Calculate what our effective level should be
    let effective = compute_effective_level(ctx, x, y, z);

    match effective {
        None => {
            // No feeder → dry up
            update.changes.push((x, y, z, ctx.tb.air));
            schedule_neighbors(x, y, z, ctx.delay, update);
        }
        Some(eff) if eff != depth => {
            // Level changed → update and re-schedule
            update
                .changes
                .push((x, y, z, fluid_rid(ctx.fluid, eff, ctx.tb)));
            update.schedule.push((x, y, z, ctx.delay, 0));
        }
        Some(_) => {
            // Level is correct — try to flow further
            if try_flow_down(ctx, x, y, z, update) {
                return;
            }
            // Spread horizontally if not at max depth
            if depth < max_depth(ctx.fluid) {
                spread_horizontal(ctx, x, y, z, depth + 1, update);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Try to place a falling block below. Returns true if successful.
fn try_flow_down<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
    update: &mut FluidUpdate,
) -> bool {
    let below_y = y - 1;
    if let Some(below_rid) = (ctx.get_block)(x, below_y, z) {
        if can_fluid_replace(below_rid, ctx.fluid, ctx.tb, &ctx.is_solid) {
            let falling_rid = fluid_rid(ctx.fluid, FALLING_DEPTH, ctx.tb);
            update.changes.push((x, below_y, z, falling_rid));
            update.schedule.push((x, below_y, z, ctx.delay, 0));
            return true;
        }
    }
    false
}

/// Spread horizontally to 4 neighbors at the given depth.
fn spread_horizontal<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
    new_depth: u8,
    update: &mut FluidUpdate,
) {
    let neighbors = [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)];
    let target_rid = fluid_rid(ctx.fluid, new_depth, ctx.tb);

    for (nx, nz) in neighbors {
        if let Some(nrid) = (ctx.get_block)(nx, y, nz) {
            if can_fluid_replace(nrid, ctx.fluid, ctx.tb, &ctx.is_solid) {
                update.changes.push((nx, y, nz, target_rid));
                update.schedule.push((nx, y, nz, ctx.delay, 0));
            }
        }
    }
}

/// Schedule ticks for the 6 neighbors of a position.
fn schedule_neighbors(x: i32, y: i32, z: i32, delay: u64, update: &mut FluidUpdate) {
    let neighbors = [
        (x - 1, y, z),
        (x + 1, y, z),
        (x, y - 1, z),
        (x, y + 1, z),
        (x, y, z - 1),
        (x, y, z + 1),
    ];
    for (nx, ny, nz) in neighbors {
        update.schedule.push((nx, ny, nz, delay, 0));
    }
}

/// Check whether there is a valid feeder (source or falling) directly above.
fn has_feeder_above<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    if let Some(above_rid) = (ctx.get_block)(x, y + 1, z) {
        if let Some(above_depth) = fluid_depth(above_rid, ctx.fluid, ctx.tb) {
            return above_depth == 0 || above_depth == FALLING_DEPTH;
        }
    }
    false
}

/// Compute the effective level this flowing block should have based on neighbors.
///
/// Returns `None` if the block should dry up (no valid feeder).
fn compute_effective_level<G: Fn(i32, i32, i32) -> Option<u32>, S: Fn(u32) -> bool>(
    ctx: &FluidCtx<G, S>,
    x: i32,
    y: i32,
    z: i32,
) -> Option<u8> {
    // If there is a source or falling block above, we should be depth 1
    if has_feeder_above(ctx, x, y, z) {
        return Some(1);
    }

    // Check 4 horizontal neighbors for the lowest depth feeder
    let neighbors = [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)];
    let mut min_neighbor_depth: Option<u8> = None;

    for (nx, nz) in neighbors {
        if let Some(nrid) = (ctx.get_block)(nx, y, nz) {
            if let Some(nd) = fluid_depth(nrid, ctx.fluid, ctx.tb) {
                // Sources (0) and falling (8) feed at depth 1
                let effective = if nd == 0 || nd == FALLING_DEPTH {
                    0
                } else {
                    nd
                };
                match min_neighbor_depth {
                    None => min_neighbor_depth = Some(effective),
                    Some(prev) if effective < prev => min_neighbor_depth = Some(effective),
                    _ => {}
                }
            }
        }
    }

    // Our depth = min_neighbor + 1
    match min_neighbor_depth {
        Some(min) if min < max_depth(ctx.fluid) => Some(min + 1),
        _ => None, // no feeder or at max depth
    }
}

/// Check if 2+ horizontal water sources are adjacent (infinite water source).
fn check_infinite_water_source(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
) -> bool {
    let neighbors = [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)];
    let mut source_count = 0;
    for (nx, nz) in neighbors {
        if let Some(nrid) = get_block(nx, y, nz) {
            if tb.water_depth(nrid) == Some(0) {
                source_count += 1;
                if source_count >= 2 {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a block at `(cx, cy, cz)` being set to `new_rid` causes a fluid interaction.
fn check_fluid_interaction(
    cx: i32,
    cy: i32,
    cz: i32,
    new_rid: u32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
) -> Option<(i32, i32, i32, u32)> {
    let existing = get_block(cx, cy, cz)?;

    // Water flowing into lava
    if tb.is_water(new_rid) && tb.is_lava(existing) {
        let lava_depth = tb.lava_depth(existing).unwrap_or(0);
        return if lava_depth == 0 {
            Some((cx, cy, cz, tb.obsidian))
        } else {
            Some((cx, cy, cz, tb.cobblestone))
        };
    }

    // Lava flowing into water
    if tb.is_lava(new_rid) && tb.is_water(existing) {
        return Some((cx, cy, cz, tb.cobblestone));
    }

    None
}

/// Whether a block can be replaced by a fluid flow.
fn can_fluid_replace(
    rid: u32,
    fluid: FluidType,
    tb: &TickBlocks,
    is_solid: &impl Fn(u32) -> bool,
) -> bool {
    if rid == tb.air {
        return true;
    }
    if is_solid(rid) {
        return false;
    }
    // Same fluid type: flowing blocks (not source/falling) can be replaced
    if let Some(d) = fluid_depth(rid, fluid, tb) {
        return d > 0 && d != FALLING_DEPTH;
    }
    // Opposing fluid → will be handled by interaction check
    if tb.is_fluid(rid) {
        return true;
    }
    // Other non-solid blocks (plants, etc.) can be replaced
    true
}

/// Get the runtime ID for a fluid at a given depth.
fn fluid_rid(fluid: FluidType, depth: u8, tb: &TickBlocks) -> u32 {
    match fluid {
        FluidType::Water => tb.water[depth as usize],
        FluidType::Lava => tb.lava[depth as usize],
    }
}

/// Get the depth of a fluid block, or None if not the expected fluid type.
fn fluid_depth(rid: u32, fluid: FluidType, tb: &TickBlocks) -> Option<u8> {
    match fluid {
        FluidType::Water => tb.water_depth(rid),
        FluidType::Lava => tb.lava_depth(rid),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tb() -> TickBlocks {
        TickBlocks::compute()
    }

    #[test]
    fn water_source_flows_down() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update.changes.iter().any(|&(x, y, z, rid)| {
            x == 0 && y == 9 && z == 0 && rid == tb.water[FALLING_DEPTH as usize]
        }));
        assert!(update
            .schedule
            .iter()
            .any(|&(x, y, z, _, _)| x == 0 && y == 9 && z == 0));
    }

    #[test]
    fn water_source_spreads_horizontally_on_solid() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        let h_changes: Vec<_> = update
            .changes
            .iter()
            .filter(|&&(_, y, _, rid)| y == 10 && rid == tb.water[1])
            .collect();
        assert_eq!(h_changes.len(), 4);
    }

    #[test]
    fn flowing_water_dries_without_feeder() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[3])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.air));
    }

    #[test]
    fn flowing_water_spreads_with_feeder() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[2])
                } else if x == -1 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        // Effective level: min neighbor is 0 (source) → should be 1
        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.water[1]));
    }

    #[test]
    fn flowing_water_propagates_deeper() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[1])
                } else if x == -1 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        // Should spread to east/north/south at depth 2
        let deeper: Vec<_> = update
            .changes
            .iter()
            .filter(|&&(_, y, _, rid)| y == 10 && rid == tb.water[2])
            .collect();
        assert_eq!(deeper.len(), 3);
    }

    #[test]
    fn max_depth_water_does_not_spread() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[7])
                } else if x == -1 && y == 10 && z == 0 {
                    Some(tb.water[6])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        let h_spreads: Vec<_> = update
            .changes
            .iter()
            .filter(|&&(_, y, _, _)| y == 10)
            .collect();
        assert!(h_spreads.is_empty());
    }

    #[test]
    fn lava_spreads_with_correct_delay() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.lava[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .schedule
            .iter()
            .all(|&(_, _, _, delay, _)| delay == LAVA_TICK_DELAY));
    }

    #[test]
    fn lava_max_spread_is_4() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.lava[4])
                } else if x == -1 && y == 10 && z == 0 {
                    Some(tb.lava[3])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        let h_spreads: Vec<_> = update
            .changes
            .iter()
            .filter(|&&(_, y, _, _)| y == 10)
            .collect();
        assert!(h_spreads.is_empty());
    }

    #[test]
    fn infinite_water_source() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[1])
                } else if x == -1 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if x == 1 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.water[0]));
    }

    #[test]
    fn water_on_lava_source_makes_obsidian() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if x == 0 && y == 9 && z == 0 {
                    Some(tb.lava[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.obsidian));
    }

    #[test]
    fn water_on_flowing_lava_makes_cobblestone() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else if x == 0 && y == 9 && z == 0 {
                    Some(tb.lava[3])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.cobblestone));
    }

    #[test]
    fn lava_on_water_makes_cobblestone() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.lava[0])
                } else if x == 0 && y == 9 && z == 0 {
                    Some(tb.water[0])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 9 && z == 0 && rid == tb.cobblestone));
    }

    #[test]
    fn falling_water_hits_solid_spreads_horizontal() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[FALLING_DEPTH as usize])
                } else if x == 0 && y == 11 && z == 0 {
                    Some(tb.water[0])
                } else if y == 9 {
                    Some(tb.stone)
                } else {
                    Some(tb.air)
                }
            },
            |rid| rid == tb.stone,
        );

        let h_changes: Vec<_> = update
            .changes
            .iter()
            .filter(|&&(_, y, _, rid)| y == 10 && rid == tb.water[1])
            .collect();
        assert_eq!(h_changes.len(), 4);
    }

    #[test]
    fn falling_water_dries_without_feeder_above() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[FALLING_DEPTH as usize])
                } else {
                    Some(tb.air)
                }
            },
            |_| false,
        );

        assert!(update
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 10 && z == 0 && rid == tb.air));
    }

    #[test]
    fn water_blocked_by_solid() {
        let tb = make_tb();
        let update = process_fluid_tick(
            0,
            10,
            0,
            &tb,
            |x, y, z| {
                if x == 0 && y == 10 && z == 0 {
                    Some(tb.water[0])
                } else {
                    Some(tb.stone)
                }
            },
            |rid| rid == tb.stone,
        );

        assert!(update.changes.is_empty());
    }

    #[test]
    fn non_fluid_block_returns_empty() {
        let tb = make_tb();
        let update = process_fluid_tick(0, 10, 0, &tb, |_, _, _| Some(tb.stone), |_| true);
        assert!(update.changes.is_empty());
        assert!(update.schedule.is_empty());
    }
}
