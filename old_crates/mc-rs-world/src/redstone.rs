//! Basic redstone logic: wire signal propagation, torch inversion, repeater delay.
//!
//! Wire propagation uses a Dijkstra-like BFS from power sources through connected
//! wire blocks (horizontal 4-directional). Torches and repeaters use scheduled ticks.

use std::collections::{BinaryHeap, HashSet, VecDeque};

use crate::block_hash::{TickBlocks, TORCH_DIRS};

/// Tick delay for redstone torch state changes (1 redstone tick = 2 game ticks).
pub const TORCH_TICK_DELAY: u64 = 2;

/// Base tick delay per repeater delay level (2 game ticks per level).
pub const REPEATER_BASE_DELAY: u64 = 2;

/// Result of a redstone update (wire recalculation or torch/repeater tick).
#[derive(Debug, Default)]
pub struct RedstoneUpdate {
    /// Block changes to apply: (x, y, z, new_runtime_id).
    pub changes: Vec<(i32, i32, i32, u32)>,
    /// New ticks to schedule: (x, y, z, delay, priority).
    pub schedule: Vec<(i32, i32, i32, u64, i32)>,
}

/// Horizontal neighbor offsets (4-directional).
const H_NEIGHBORS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// All 6 neighbor offsets.
const ALL_NEIGHBORS: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 1, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];

// ---------------------------------------------------------------------------
// Wire signal propagation
// ---------------------------------------------------------------------------

/// Recalculate redstone wire signal levels near position `(x, y, z)`.
///
/// Called after a power source changes (lever toggle, torch flip, block break/place).
/// Finds all connected wire via horizontal flood-fill, then propagates signal
/// from power sources using a Dijkstra-like BFS.
///
/// Returns wire state changes and any scheduled ticks for affected torches/repeaters.
pub fn recalculate_wire_from(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    _is_solid: impl Fn(u32) -> bool,
) -> RedstoneUpdate {
    let mut update = RedstoneUpdate::default();

    // 1. Seed: find wire blocks adjacent to (x, y, z) and at (x, y, z)
    let mut wire_set: HashSet<(i32, i32, i32)> = HashSet::new();
    let mut fill_queue: VecDeque<(i32, i32, i32)> = VecDeque::new();

    let seed_positions = [
        (x, y, z),
        (x + 1, y, z),
        (x - 1, y, z),
        (x, y + 1, z),
        (x, y - 1, z),
        (x, y, z + 1),
        (x, y, z - 1),
    ];
    for (sx, sy, sz) in seed_positions {
        if let Some(rid) = get_block(sx, sy, sz) {
            if tb.is_wire(rid) && wire_set.insert((sx, sy, sz)) {
                fill_queue.push_back((sx, sy, sz));
            }
        }
    }

    // 2. Flood-fill through horizontally connected wire
    while let Some((wx, wy, wz)) = fill_queue.pop_front() {
        for (dx, dz) in H_NEIGHBORS {
            let (nx, nz) = (wx + dx, wz + dz);
            if let Some(rid) = get_block(nx, wy, nz) {
                if tb.is_wire(rid) && wire_set.insert((nx, wy, nz)) {
                    fill_queue.push_back((nx, wy, nz));
                }
            }
        }
    }

    if wire_set.is_empty() {
        return update;
    }

    // 3. For each wire, compute power from adjacent non-wire sources and seed the BFS
    // Store (signal, x, y, z) — max-heap by signal
    let mut signal_map: Vec<((i32, i32, i32), u8)> =
        wire_set.iter().map(|&pos| (pos, 0u8)).collect();
    let sig_idx: std::collections::HashMap<(i32, i32, i32), usize> = signal_map
        .iter()
        .enumerate()
        .map(|(i, &(pos, _))| (pos, i))
        .collect();
    let mut heap: BinaryHeap<(u8, i32, i32, i32)> = BinaryHeap::new();

    for &(wx, wy, wz) in &wire_set {
        let mut max_power: u8 = 0;

        // Check all 6 neighbors for power sources
        for (dx, dy, dz) in ALL_NEIGHBORS {
            let (nx, ny, nz) = (wx + dx, wy + dy, wz + dz);
            if let Some(rid) = get_block(nx, ny, nz) {
                // Direct power sources (lever on, torch lit, redstone block)
                let p = tb.power_output(rid);
                if p > max_power {
                    max_power = p;
                }
                // Powered repeater outputting towards this wire
                if tb.is_repeater_powered(rid) {
                    if let Some(dir) = tb.repeater_direction(rid) {
                        let (ox, oz) = repeater_output_delta(dir);
                        if nx + ox == wx && nz + oz == wz && ny == wy {
                            max_power = 15;
                        }
                    }
                }
            }
        }

        if max_power > 0 {
            if let Some(&idx) = sig_idx.get(&(wx, wy, wz)) {
                signal_map[idx].1 = max_power;
            }
            heap.push((max_power, wx, wy, wz));
        }
    }

    // 4. Dijkstra-like BFS: propagate from highest signal
    while let Some((power, px, py, pz)) = heap.pop() {
        if let Some(&idx) = sig_idx.get(&(px, py, pz)) {
            if power < signal_map[idx].1 {
                continue; // already found a better path
            }
        }
        if power <= 1 {
            continue; // can't propagate further
        }
        let next_power = power - 1;
        for (dx, dz) in H_NEIGHBORS {
            let (nx, nz) = (px + dx, pz + dz);
            if let Some(&idx) = sig_idx.get(&(nx, py, nz)) {
                if next_power > signal_map[idx].1 {
                    signal_map[idx].1 = next_power;
                    heap.push((next_power, nx, py, nz));
                }
            }
        }
    }

    // 5. Generate changes for wire that changed signal level
    let mut changed_wires: HashSet<(i32, i32, i32)> = HashSet::new();
    for &((wx, wy, wz), new_signal) in &signal_map {
        if let Some(rid) = get_block(wx, wy, wz) {
            let old_signal = tb.wire_signal(rid).unwrap_or(0);
            if new_signal != old_signal {
                update
                    .changes
                    .push((wx, wy, wz, tb.redstone_wire[new_signal as usize]));
                changed_wires.insert((wx, wy, wz));
            }
        }
    }

    // 6. Schedule torch/repeater ticks for components whose power input may have changed.
    //    A torch/repeater is affected if its attachment/input block is adjacent to a changed wire.
    //    So we check: changed_wire neighbors → each of THEIR neighbors for torch/repeater.
    let mut affected_blocks: HashSet<(i32, i32, i32)> = HashSet::new();
    for &(wx, wy, wz) in &changed_wires {
        for (dx, dy, dz) in ALL_NEIGHBORS {
            affected_blocks.insert((wx + dx, wy + dy, wz + dz));
        }
    }
    for &(bx, by, bz) in &affected_blocks {
        for (dx, dy, dz) in ALL_NEIGHBORS {
            let (nx, ny, nz) = (bx + dx, by + dy, bz + dz);
            if let Some(rid) = get_block(nx, ny, nz) {
                if tb.is_torch(rid) {
                    update.schedule.push((nx, ny, nz, TORCH_TICK_DELAY, 0));
                }
                if tb.is_repeater(rid) {
                    let delay = tb.repeater_delay(rid).unwrap_or(0) as u64;
                    update
                        .schedule
                        .push((nx, ny, nz, (delay + 1) * REPEATER_BASE_DELAY, 0));
                }
            }
        }
    }

    // Deduplicate schedules
    let mut seen = HashSet::new();
    update
        .schedule
        .retain(|&(sx, sy, sz, _, _)| seen.insert((sx, sy, sz)));

    update
}

// ---------------------------------------------------------------------------
// Scheduled tick processing for torch and repeater
// ---------------------------------------------------------------------------

/// Process a scheduled redstone tick at `(x, y, z)`.
///
/// Handles torch inversion and repeater state changes. After toggling the
/// component, recalculates connected wire with the updated state overlaid.
pub fn process_redstone_tick(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> RedstoneUpdate {
    let rid = match get_block(x, y, z) {
        Some(r) => r,
        None => return RedstoneUpdate::default(),
    };

    if tb.is_torch(rid) {
        return process_torch_tick(x, y, z, rid, tb, &get_block, &is_solid);
    }

    if tb.is_repeater(rid) {
        return process_repeater_tick(x, y, z, rid, tb, &get_block, &is_solid);
    }

    RedstoneUpdate::default()
}

/// Process a torch tick: check if the torch should flip.
fn process_torch_tick(
    x: i32,
    y: i32,
    z: i32,
    rid: u32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: &impl Fn(u32) -> bool,
) -> RedstoneUpdate {
    let mut update = RedstoneUpdate::default();

    let dir_idx = match tb.torch_direction(rid) {
        Some(d) => d,
        None => return update,
    };

    // Find attachment block position
    let (ax, ay, az) = attachment_pos(x, y, z, dir_idx);

    // Check if attachment block is powered
    let attach_powered = is_block_powered((ax, ay, az), (x, y, z), tb, get_block);

    let is_lit = tb.is_torch_lit(rid);

    if is_lit && attach_powered {
        // Turn off: lit → unlit
        if let Some(new_rid) = tb.toggle_torch(rid) {
            update.changes.push((x, y, z, new_rid));
            // Recalculate wire with the torch change overlaid
            let wire_update = recalculate_wire_from(
                x,
                y,
                z,
                tb,
                |bx, by, bz| {
                    if bx == x && by == y && bz == z {
                        Some(new_rid)
                    } else {
                        get_block(bx, by, bz)
                    }
                },
                is_solid,
            );
            update.changes.extend(wire_update.changes);
            update.schedule.extend(wire_update.schedule);
        }
    } else if !is_lit && !attach_powered {
        // Turn on: unlit → lit
        if let Some(new_rid) = tb.toggle_torch(rid) {
            update.changes.push((x, y, z, new_rid));
            let wire_update = recalculate_wire_from(
                x,
                y,
                z,
                tb,
                |bx, by, bz| {
                    if bx == x && by == y && bz == z {
                        Some(new_rid)
                    } else {
                        get_block(bx, by, bz)
                    }
                },
                is_solid,
            );
            update.changes.extend(wire_update.changes);
            update.schedule.extend(wire_update.schedule);
        }
    }

    update
}

/// Process a repeater tick: check if the repeater output should change.
fn process_repeater_tick(
    x: i32,
    y: i32,
    z: i32,
    rid: u32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: &impl Fn(u32) -> bool,
) -> RedstoneUpdate {
    let mut update = RedstoneUpdate::default();

    let dir = match tb.repeater_direction(rid) {
        Some(d) => d,
        None => return update,
    };

    // Check input side
    let (ix, iz) = repeater_input_delta(dir);
    let (input_x, input_z) = (x + ix, z + iz);

    let input_powered = if let Some(input_rid) = get_block(input_x, y, input_z) {
        tb.power_output(input_rid) > 0
            || tb.wire_signal(input_rid).is_some_and(|s| s > 0)
            || tb.is_repeater_powered(input_rid)
    } else {
        false
    };

    let is_powered = tb.is_repeater_powered(rid);

    if input_powered && !is_powered {
        // Turn on
        if let Some(new_rid) = tb.toggle_repeater(rid) {
            update.changes.push((x, y, z, new_rid));
            let wire_update = recalculate_wire_from(
                x,
                y,
                z,
                tb,
                |bx, by, bz| {
                    if bx == x && by == y && bz == z {
                        Some(new_rid)
                    } else {
                        get_block(bx, by, bz)
                    }
                },
                is_solid,
            );
            update.changes.extend(wire_update.changes);
            update.schedule.extend(wire_update.schedule);
        }
    } else if !input_powered && is_powered {
        // Turn off
        if let Some(new_rid) = tb.toggle_repeater(rid) {
            update.changes.push((x, y, z, new_rid));
            let wire_update = recalculate_wire_from(
                x,
                y,
                z,
                tb,
                |bx, by, bz| {
                    if bx == x && by == y && bz == z {
                        Some(new_rid)
                    } else {
                        get_block(bx, by, bz)
                    }
                },
                is_solid,
            );
            update.changes.extend(wire_update.changes);
            update.schedule.extend(wire_update.schedule);
        }
    }

    update
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the attachment block position for a torch based on its direction index.
fn attachment_pos(x: i32, y: i32, z: i32, dir_idx: usize) -> (i32, i32, i32) {
    // TORCH_DIRS: ["east", "north", "south", "top", "unknown", "west"]
    match TORCH_DIRS[dir_idx] {
        "top" => (x, y - 1, z),   // attached below
        "east" => (x - 1, y, z),  // attached to the west
        "west" => (x + 1, y, z),  // attached to the east
        "north" => (x, y, z + 1), // attached to the south
        "south" => (x, y, z - 1), // attached to the north
        _ => (x, y - 1, z),       // "unknown" — treat as top
    }
}

/// Check if a block at `(x, y, z)` is receiving redstone power.
/// `exclude` position is ignored (to avoid self-powering torch/repeater).
fn is_block_powered(
    pos: (i32, i32, i32),
    exclude: (i32, i32, i32),
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
) -> bool {
    let (x, y, z) = pos;
    for (dx, dy, dz) in ALL_NEIGHBORS {
        let (nx, ny, nz) = (x + dx, y + dy, z + dz);
        if (nx, ny, nz) == exclude {
            continue; // skip the component itself
        }
        if let Some(rid) = get_block(nx, ny, nz) {
            // Wire with signal > 0
            if tb.wire_signal(rid).is_some_and(|s| s > 0) {
                return true;
            }
            // Power sources (lever on, torch lit, redstone block)
            if tb.power_output(rid) > 0 {
                return true;
            }
            // Powered repeater outputting towards this position
            if tb.is_repeater_powered(rid) {
                if let Some(dir) = tb.repeater_direction(rid) {
                    let (ox, oz) = repeater_output_delta(dir);
                    if nx + ox == x && nz + oz == z && ny == y {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Get the input side offset for a repeater based on direction.
/// Direction: 0=south, 1=west, 2=north, 3=east.
/// Input is the side the repeater receives signal from.
fn repeater_input_delta(direction: u8) -> (i32, i32) {
    match direction {
        0 => (0, -1), // facing south, input from north (z-1)
        1 => (1, 0),  // facing west, input from east (x+1)
        2 => (0, 1),  // facing north, input from south (z+1)
        3 => (-1, 0), // facing east, input from west (x-1)
        _ => (0, 0),
    }
}

/// Get the output side offset for a repeater based on direction.
fn repeater_output_delta(direction: u8) -> (i32, i32) {
    match direction {
        0 => (0, 1),  // facing south, output to south (z+1)
        1 => (-1, 0), // facing west, output to west (x-1)
        2 => (0, -1), // facing north, output to north (z-1)
        3 => (1, 0),  // facing east, output to east (x+1)
        _ => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tb() -> TickBlocks {
        TickBlocks::compute()
    }

    // Helper: build a world from a list of (x, y, z, rid) tuples.
    fn world_from(
        blocks: &[(i32, i32, i32, u32)],
        default: u32,
    ) -> impl Fn(i32, i32, i32) -> Option<u32> + '_ {
        move |x, y, z| {
            for &(bx, by, bz, rid) in blocks {
                if bx == x && by == y && bz == z {
                    return Some(rid);
                }
            }
            Some(default)
        }
    }

    #[test]
    fn wire_signal_from_lever() {
        let tb = make_tb();
        // Lever ON at (0, 0, 0), wire at (1, 0, 0)
        let lever_on = tb.lever[0][1]; // any direction, open_bit=1
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![(0, 0, 0, lever_on), (1, 0, 0, wire_0)];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Wire at (1,0,0) should be set to signal 15
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, z, rid)| x == 1 && z == 0 && rid == tb.redstone_wire[15]));
    }

    #[test]
    fn wire_signal_decay() {
        let tb = make_tb();
        // Lever ON at (0, 0, 0), wire at (1..4, 0, 0)
        let lever_on = tb.lever[0][1];
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![
            (0, 0, 0, lever_on),
            (1, 0, 0, wire_0),
            (2, 0, 0, wire_0),
            (3, 0, 0, wire_0),
        ];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // wire at x=1 → 15, x=2 → 14, x=3 → 13
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 1 && rid == tb.redstone_wire[15]));
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 2 && rid == tb.redstone_wire[14]));
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 3 && rid == tb.redstone_wire[13]));
    }

    #[test]
    fn wire_max_distance() {
        let tb = make_tb();
        // Lever ON at (0,0,0), wire at (1..16,0,0) — 15 wires + 1 beyond
        let lever_on = tb.lever[0][1];
        let wire_0 = tb.redstone_wire[0];
        let mut blocks = vec![(0, 0, 0, lever_on)];
        for i in 1..=16 {
            blocks.push((i, 0, 0, wire_0));
        }
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Wire at x=15 → signal 1, wire at x=16 → signal 0 (no change from default 0)
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 15 && rid == tb.redstone_wire[1]));
        // x=16 should NOT be in changes (stays at 0)
        assert!(!result.changes.iter().any(|&(x, _, _, _)| x == 16));
    }

    #[test]
    fn wire_multiple_sources() {
        let tb = make_tb();
        // Two levers at (0,0,0) and (4,0,0), wire at (1..4, 0, 0)
        let lever_on = tb.lever[0][1];
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![
            (0, 0, 0, lever_on),
            (1, 0, 0, wire_0),
            (2, 0, 0, wire_0),
            (3, 0, 0, wire_0),
            (4, 0, 0, lever_on),
        ];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Wire at x=2 gets max(15-2, 15-2) = 13 from either source
        // Wire at x=1 gets 15 from left lever, 15-3=12 from right — max is 15
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 1 && rid == tb.redstone_wire[15]));
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 2 && rid == tb.redstone_wire[14]));
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 3 && rid == tb.redstone_wire[15]));
    }

    #[test]
    fn wire_source_removed() {
        let tb = make_tb();
        // Lever OFF at (0,0,0), wire at (1,0,0) with signal 15 (should drop to 0)
        let lever_off = tb.lever[0][0]; // open_bit=0
        let wire_15 = tb.redstone_wire[15];
        let blocks = vec![(0, 0, 0, lever_off), (1, 0, 0, wire_15)];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Wire should drop to 0
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 1 && rid == tb.redstone_wire[0]));
    }

    #[test]
    fn wire_isolated_stays_zero() {
        let tb = make_tb();
        // Wire at (5,0,5) with no power source nearby
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![(5, 0, 5, wire_0)];
        let result = recalculate_wire_from(5, 0, 5, &tb, world_from(&blocks, tb.air), |_| false);
        // No changes — wire stays at 0
        assert!(result.changes.is_empty());
    }

    #[test]
    fn lever_toggle_hashes() {
        let tb = make_tb();
        // Each lever direction has distinct on/off hashes
        for dir in 0..8 {
            let off = tb.lever[dir][0];
            let on = tb.lever[dir][1];
            assert_ne!(off, on, "lever dir {dir}: off and on should differ");
            assert_eq!(tb.toggle_lever(off), Some(on));
            assert_eq!(tb.toggle_lever(on), Some(off));
            assert!(tb.is_lever(off));
            assert!(tb.is_lever(on));
            assert!(!tb.is_lever_on(off));
            assert!(tb.is_lever_on(on));
        }
    }

    #[test]
    fn torch_on_unpowered_block() {
        let tb = make_tb();
        // Torch (lit, top) at (0, 1, 0), stone at (0, 0, 0) with no power
        let torch = tb.torch_lit[3]; // "top" is index 3
        let blocks = vec![(0, 1, 0, torch), (0, 0, 0, tb.stone)];
        let result = process_redstone_tick(0, 1, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Torch should stay lit — no changes
        assert!(result.changes.is_empty());
    }

    #[test]
    fn torch_turns_off_when_powered() {
        let tb = make_tb();
        // Torch (lit, top) at (0, 2, 0), wire with signal 15 at (0, 1, 1), stone at (0, 1, 0)
        // Wire powers the stone block → torch should turn off
        let torch = tb.torch_lit[3]; // "top"
        let wire_15 = tb.redstone_wire[15];
        let blocks = vec![(0, 2, 0, torch), (0, 1, 0, tb.stone), (0, 1, 1, wire_15)];
        let result = process_redstone_tick(0, 2, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Torch should turn off
        assert!(result
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 2 && z == 0 && tb.is_torch_unlit(rid)));
    }

    #[test]
    fn torch_turns_on_when_unpowered() {
        let tb = make_tb();
        // Unlit torch (top) at (0, 2, 0), stone at (0, 1, 0) with no power around
        let torch_unlit = tb.torch_unlit[3]; // "top"
        let blocks = vec![(0, 2, 0, torch_unlit), (0, 1, 0, tb.stone)];
        let result = process_redstone_tick(0, 2, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Torch should turn on
        assert!(result
            .changes
            .iter()
            .any(|&(x, y, z, rid)| x == 0 && y == 2 && z == 0 && tb.is_torch_lit(rid)));
    }

    #[test]
    fn repeater_turns_on_with_input() {
        let tb = make_tb();
        // Unpowered repeater (dir=0, delay=0) at (0,0,0), wire with signal 15 at input side (0,0,-1)
        let rep = tb.repeater_off[0][0]; // dir=0 (south), delay=0
        let wire_15 = tb.redstone_wire[15];
        let blocks = vec![
            (0, 0, 0, rep),
            (0, 0, -1, wire_15), // input from north
        ];
        let result = process_redstone_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Repeater should turn on
        assert!(result
            .changes
            .iter()
            .any(|&(x, y, z, _)| x == 0 && y == 0 && z == 0));
        let new_rid = result
            .changes
            .iter()
            .find(|&&(x, _, z, _)| x == 0 && z == 0)
            .unwrap()
            .3;
        assert!(tb.is_repeater_powered(new_rid));
    }

    #[test]
    fn repeater_boosts_signal() {
        let tb = make_tb();
        // Powered repeater (dir=0, delay=0) at (0,0,0), wire at output side (0,0,1)
        let rep_on = tb.repeater_on[0][0]; // dir=0 (south), delay=0, powered
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![
            (0, 0, 0, rep_on),
            (0, 0, 1, wire_0), // output to south
        ];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Wire at output should get signal 15
        assert!(result
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 1 && rid == tb.redstone_wire[15]));
    }

    #[test]
    fn repeater_direction_io() {
        // Verify input/output deltas
        assert_eq!(repeater_input_delta(0), (0, -1)); // south: input from north
        assert_eq!(repeater_output_delta(0), (0, 1)); // south: output to south
        assert_eq!(repeater_input_delta(2), (0, 1)); // north: input from south
        assert_eq!(repeater_output_delta(2), (0, -1)); // north: output to north
    }

    #[test]
    fn redstone_block_powers_wire() {
        let tb = make_tb();
        // Redstone block at (0,0,0), wire at (1,0,0)
        let wire_0 = tb.redstone_wire[0];
        let blocks = vec![(0, 0, 0, tb.redstone_block), (1, 0, 0, wire_0)];
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 1 && rid == tb.redstone_wire[15]));
    }

    #[test]
    fn circuit_lever_wire_torch() {
        let tb = make_tb();
        // Lever ON → wire → stone block → torch on top
        // lever(0,0,0) → wire(1,0,0) → wire(2,0,0) → stone(3,0,0) → torch(3,1,0)
        let lever_on = tb.lever[0][1];
        let wire_0 = tb.redstone_wire[0];
        let torch_lit = tb.torch_lit[3]; // "top"
        let blocks = vec![
            (0, 0, 0, lever_on),
            (1, 0, 0, wire_0),
            (2, 0, 0, wire_0),
            (3, 0, 0, tb.stone),
            (3, 1, 0, torch_lit),
        ];
        // Recalculate wire from lever position
        let result = recalculate_wire_from(0, 0, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Wire at (1,0,0) → 15, (2,0,0) → 14
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 1 && rid == tb.redstone_wire[15]));
        assert!(result
            .changes
            .iter()
            .any(|&(x, _, _, rid)| x == 2 && rid == tb.redstone_wire[14]));
        // Should schedule a torch tick since wire adjacent to stone changed
        assert!(result
            .schedule
            .iter()
            .any(|&(x, y, _, _, _)| x == 3 && y == 1));
    }

    #[test]
    fn cycle_repeater_delay() {
        let tb = make_tb();
        let rep = tb.repeater_off[0][0]; // delay=0
        let next = tb.cycle_repeater_delay(rep).unwrap();
        assert_eq!(tb.repeater_delay(next), Some(1));
        let next2 = tb.cycle_repeater_delay(next).unwrap();
        assert_eq!(tb.repeater_delay(next2), Some(2));
        let next3 = tb.cycle_repeater_delay(next2).unwrap();
        assert_eq!(tb.repeater_delay(next3), Some(3));
        let next4 = tb.cycle_repeater_delay(next3).unwrap();
        assert_eq!(tb.repeater_delay(next4), Some(0)); // wraps around
    }
}
