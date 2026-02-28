//! Piston mechanics: extension, retraction, and push chain calculation.
//!
//! When a piston receives a redstone signal, it schedules an extension tick.
//! When it loses the signal, it schedules a retraction tick. Pistons can push
//! up to 12 blocks. Sticky pistons pull one block on retraction.

use crate::block_hash::TickBlocks;

/// Maximum number of blocks a piston can push.
pub const PISTON_PUSH_LIMIT: usize = 12;

/// Tick delay for piston extension/retraction (2 game ticks = 1 redstone tick).
pub const PISTON_TICK_DELAY: u64 = 2;

/// Result of processing a piston tick.
#[derive(Debug, Default)]
pub struct PistonUpdate {
    /// Block changes to apply: (x, y, z, new_runtime_id).
    pub changes: Vec<(i32, i32, i32, u32)>,
    /// New ticks to schedule: (x, y, z, delay, priority).
    pub schedule: Vec<(i32, i32, i32, u64, i32)>,
    /// Positions where fluid/redstone/gravity updates should be triggered.
    pub neighbor_updates: Vec<(i32, i32, i32)>,
}

/// Convert facing_direction (0-5) to (dx, dy, dz) offset.
/// 0=down, 1=up, 2=south(z+), 3=north(z-), 4=east(x+), 5=west(x-)
pub fn facing_delta(facing: u8) -> (i32, i32, i32) {
    match facing {
        0 => (0, -1, 0),
        1 => (0, 1, 0),
        2 => (0, 0, 1),
        3 => (0, 0, -1),
        4 => (1, 0, 0),
        5 => (-1, 0, 0),
        _ => (0, 0, 0),
    }
}

/// Determine piston facing_direction from player pitch and yaw.
///
/// Piston faces towards the direction the player is looking.
/// If pitch > 45, faces down (0). If pitch < -45, faces up (1).
/// Otherwise horizontal from yaw.
pub fn facing_from_look(pitch: f32, yaw: f32) -> u8 {
    if pitch > 45.0 {
        return 0; // down
    }
    if pitch < -45.0 {
        return 1; // up
    }
    let y = yaw.rem_euclid(360.0);
    if (315.0..360.0).contains(&y) || y < 45.0 {
        2 // south
    } else if (45.0..135.0).contains(&y) {
        5 // west
    } else if (135.0..225.0).contains(&y) {
        3 // north
    } else {
        4 // east
    }
}

/// Get the output side offset for a repeater based on direction (duplicated from redstone).
fn repeater_output_delta(direction: u8) -> (i32, i32) {
    match direction {
        0 => (0, 1),
        1 => (-1, 0),
        2 => (0, -1),
        3 => (1, 0),
        _ => (0, 0),
    }
}

/// Check if a piston at (x, y, z) is receiving redstone power from any adjacent block.
pub fn is_piston_powered(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
) -> bool {
    let neighbors = [
        (x + 1, y, z),
        (x - 1, y, z),
        (x, y + 1, z),
        (x, y - 1, z),
        (x, y, z + 1),
        (x, y, z - 1),
    ];
    for (nx, ny, nz) in neighbors {
        if let Some(rid) = get_block(nx, ny, nz) {
            // Wire with signal > 0
            if let Some(s) = tb.wire_signal(rid) {
                if s > 0 {
                    return true;
                }
            }
            // Power sources (lever on, torch lit, redstone block)
            if tb.power_output(rid) > 0 {
                return true;
            }
            // Powered repeater outputting towards this piston
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

/// Calculate the chain of blocks to push in the piston's facing direction.
///
/// Returns `Some(positions)` with the list of (x, y, z, runtime_id) to move,
/// or `None` if the push is blocked (immovable block or > 12 blocks).
pub fn calculate_push_chain(
    piston_x: i32,
    piston_y: i32,
    piston_z: i32,
    facing: u8,
    tb: &TickBlocks,
    get_block: &impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: &impl Fn(u32) -> bool,
) -> Option<Vec<(i32, i32, i32, u32)>> {
    let (dx, dy, dz) = facing_delta(facing);
    let mut chain = Vec::new();

    let mut cx = piston_x + dx;
    let mut cy = piston_y + dy;
    let mut cz = piston_z + dz;

    loop {
        let rid = get_block(cx, cy, cz)?;

        // Air or non-solid → end of chain, push is possible
        if rid == tb.air || !is_solid(rid) {
            break;
        }

        // Immovable block → push blocked
        if tb.is_immovable(rid) {
            return None;
        }

        chain.push((cx, cy, cz, rid));

        if chain.len() > PISTON_PUSH_LIMIT {
            return None;
        }

        cx += dx;
        cy += dy;
        cz += dz;
    }

    Some(chain)
}

/// Process a scheduled piston tick at `(x, y, z)`.
///
/// Determines whether the piston should extend or retract based on current
/// power state, and performs the appropriate block movements.
pub fn process_piston_tick(
    x: i32,
    y: i32,
    z: i32,
    tb: &TickBlocks,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_solid: impl Fn(u32) -> bool,
) -> PistonUpdate {
    let mut update = PistonUpdate::default();

    let rid = match get_block(x, y, z) {
        Some(r) => r,
        None => return update,
    };

    if !tb.is_piston(rid) {
        return update;
    }

    let facing = match tb.piston_facing(rid) {
        Some(f) => f,
        None => return update,
    };

    let is_sticky = tb.is_sticky_piston(rid);
    let (dx, dy, dz) = facing_delta(facing);
    let front_x = x + dx;
    let front_y = y + dy;
    let front_z = z + dz;

    let powered = is_piston_powered(x, y, z, tb, &get_block);

    // Check if piston is currently extended (matching arm in front)
    let is_extended = get_block(front_x, front_y, front_z).is_some_and(|frid| {
        let is_matching_arm = if is_sticky {
            tb.sticky_piston_arm.contains(&frid)
        } else {
            tb.piston_arm.contains(&frid)
        };
        is_matching_arm && tb.piston_facing(frid) == Some(facing)
    });

    if powered && !is_extended {
        // --- EXTEND ---
        if let Some(chain) = calculate_push_chain(x, y, z, facing, tb, &get_block, &is_solid) {
            // Move blocks from back to front (reverse) to avoid overwriting
            for &(bx, by, bz, block_rid) in chain.iter().rev() {
                update
                    .changes
                    .push((bx + dx, by + dy, bz + dz, block_rid));
                update.neighbor_updates.push((bx, by, bz));
                update.neighbor_updates.push((bx + dx, by + dy, bz + dz));
            }
            // Place the piston arm
            let arm_rid = if is_sticky {
                tb.sticky_piston_arm[facing as usize]
            } else {
                tb.piston_arm[facing as usize]
            };
            update.changes.push((front_x, front_y, front_z, arm_rid));
            update.neighbor_updates.push((front_x, front_y, front_z));
        }
    } else if !powered && is_extended {
        // --- RETRACT ---
        update.changes.push((front_x, front_y, front_z, tb.air));
        update.neighbor_updates.push((front_x, front_y, front_z));

        // Sticky piston: pull the block behind the arm
        if is_sticky {
            let pull_x = front_x + dx;
            let pull_y = front_y + dy;
            let pull_z = front_z + dz;
            if let Some(pull_rid) = get_block(pull_x, pull_y, pull_z) {
                if pull_rid != tb.air
                    && is_solid(pull_rid)
                    && !tb.is_immovable(pull_rid)
                    && !tb.is_piston(pull_rid)
                {
                    update.changes.push((pull_x, pull_y, pull_z, tb.air));
                    update.changes.push((front_x, front_y, front_z, pull_rid));
                    update.neighbor_updates.push((pull_x, pull_y, pull_z));
                }
            }
        }
    }

    update
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tb() -> TickBlocks {
        TickBlocks::compute()
    }

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

    // --- Hash tests ---

    #[test]
    fn piston_hashes_distinct() {
        let tb = make_tb();
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert_ne!(tb.piston[i], tb.piston[j]);
                assert_ne!(tb.sticky_piston[i], tb.sticky_piston[j]);
                assert_ne!(tb.piston_arm[i], tb.piston_arm[j]);
                assert_ne!(tb.sticky_piston_arm[i], tb.sticky_piston_arm[j]);
            }
        }
    }

    #[test]
    fn piston_facing_lookup() {
        let tb = make_tb();
        for i in 0..6u8 {
            assert_eq!(tb.piston_facing(tb.piston[i as usize]), Some(i));
            assert_eq!(tb.piston_facing(tb.sticky_piston[i as usize]), Some(i));
            assert_eq!(tb.piston_facing(tb.piston_arm[i as usize]), Some(i));
            assert_eq!(tb.piston_facing(tb.sticky_piston_arm[i as usize]), Some(i));
        }
    }

    #[test]
    fn is_piston_checks() {
        let tb = make_tb();
        assert!(tb.is_piston(tb.piston[0]));
        assert!(tb.is_piston(tb.sticky_piston[3]));
        assert!(!tb.is_piston(tb.piston_arm[0]));
        assert!(!tb.is_piston(tb.air));
    }

    #[test]
    fn is_immovable_blocks() {
        let tb = make_tb();
        assert!(tb.is_immovable(tb.obsidian));
        assert!(tb.is_immovable(tb.bedrock));
        assert!(tb.is_immovable(tb.enchanting_table_tick));
        assert!(tb.is_immovable(tb.piston_arm[2]));
        assert!(!tb.is_immovable(tb.stone));
        assert!(!tb.is_immovable(tb.dirt));
    }

    // --- Facing direction tests ---

    #[test]
    fn facing_delta_values() {
        assert_eq!(facing_delta(0), (0, -1, 0)); // down
        assert_eq!(facing_delta(1), (0, 1, 0)); // up
        assert_eq!(facing_delta(2), (0, 0, 1)); // south
        assert_eq!(facing_delta(3), (0, 0, -1)); // north
        assert_eq!(facing_delta(4), (1, 0, 0)); // east
        assert_eq!(facing_delta(5), (-1, 0, 0)); // west
    }

    #[test]
    fn facing_from_look_up_down() {
        assert_eq!(facing_from_look(-60.0, 0.0), 1); // looking up → faces up
        assert_eq!(facing_from_look(60.0, 0.0), 0); // looking down → faces down
    }

    #[test]
    fn facing_from_look_horizontal() {
        assert_eq!(facing_from_look(0.0, 0.0), 2); // south
        assert_eq!(facing_from_look(0.0, 90.0), 5); // west
        assert_eq!(facing_from_look(0.0, 180.0), 3); // north
        assert_eq!(facing_from_look(0.0, 270.0), 4); // east
    }

    // --- Push chain tests ---

    #[test]
    fn push_chain_empty_air() {
        let tb = make_tb();
        let blocks = vec![(0, 0, 0, tb.piston[2])];
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|_| false);
        assert_eq!(chain.unwrap().len(), 0);
    }

    #[test]
    fn push_chain_one_block() {
        let tb = make_tb();
        let blocks = vec![(0, 0, 0, tb.piston[2]), (0, 0, 1, tb.stone)];
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|rid| {
            rid == tb.stone
        });
        let chain = chain.unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0], (0, 0, 1, tb.stone));
    }

    #[test]
    fn push_chain_multiple_blocks() {
        let tb = make_tb();
        let blocks = vec![
            (0, 0, 0, tb.piston[2]),
            (0, 0, 1, tb.stone),
            (0, 0, 2, tb.stone),
            (0, 0, 3, tb.stone),
        ];
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|rid| {
            rid == tb.stone
        });
        assert_eq!(chain.unwrap().len(), 3);
    }

    #[test]
    fn push_chain_blocked_by_obsidian() {
        let tb = make_tb();
        let blocks = vec![(0, 0, 0, tb.piston[2]), (0, 0, 1, tb.obsidian)];
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|rid| {
            rid == tb.obsidian
        });
        assert!(chain.is_none());
    }

    #[test]
    fn push_chain_exceeds_limit() {
        let tb = make_tb();
        let mut blocks = vec![(0, 0, 0, tb.piston[2])];
        for i in 1..=13 {
            blocks.push((0, 0, i, tb.stone));
        }
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|rid| {
            rid == tb.stone
        });
        assert!(chain.is_none());
    }

    #[test]
    fn push_chain_exactly_at_limit() {
        let tb = make_tb();
        let mut blocks = vec![(0, 0, 0, tb.piston[2])];
        for i in 1..=12 {
            blocks.push((0, 0, i, tb.stone));
        }
        let chain = calculate_push_chain(0, 0, 0, 2, &tb, &world_from(&blocks, tb.air), &|rid| {
            rid == tb.stone
        });
        assert_eq!(chain.unwrap().len(), 12);
    }

    // --- Extend/retract tick tests ---

    #[test]
    fn piston_extends_when_powered() {
        let tb = make_tb();
        let lever_on = tb.lever[0][1];
        let blocks = vec![
            (0, 0, 0, tb.piston[2]),
            (0, 0, -1, lever_on),
            (0, 0, 1, tb.stone),
        ];
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Should place arm at (0,0,1) and move stone to (0,0,2)
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 1 && rid == tb.piston_arm[2]));
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 2 && rid == tb.stone));
    }

    #[test]
    fn piston_retracts_when_unpowered() {
        let tb = make_tb();
        let blocks = vec![(0, 0, 0, tb.piston[2]), (0, 0, 1, tb.piston_arm[2])];
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        // Should remove arm at (0,0,1) → air
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 1 && rid == tb.air));
    }

    #[test]
    fn sticky_piston_pulls_on_retract() {
        let tb = make_tb();
        let blocks = vec![
            (0, 0, 0, tb.sticky_piston[2]),
            (0, 0, 1, tb.sticky_piston_arm[2]),
            (0, 0, 2, tb.stone),
        ];
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        // Stone pulled to (0,0,1), old position (0,0,2) → air
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 1 && rid == tb.stone));
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 2 && rid == tb.air));
    }

    #[test]
    fn sticky_piston_no_pull_immovable() {
        let tb = make_tb();
        let blocks = vec![
            (0, 0, 0, tb.sticky_piston[2]),
            (0, 0, 1, tb.sticky_piston_arm[2]),
            (0, 0, 2, tb.obsidian),
        ];
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.obsidian
        });
        // Arm removed but obsidian NOT pulled
        assert!(update
            .changes
            .iter()
            .any(|&(_, _, z, rid)| z == 1 && rid == tb.air));
        assert!(!update.changes.iter().any(|&(_, _, z, _)| z == 2));
    }

    #[test]
    fn piston_no_extend_when_blocked() {
        let tb = make_tb();
        let lever_on = tb.lever[0][1];
        let mut blocks = vec![(0, 0, 0, tb.piston[2]), (0, 0, -1, lever_on)];
        for i in 1..=13 {
            blocks.push((0, 0, i, tb.stone));
        }
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |rid| {
            rid == tb.stone
        });
        assert!(update.changes.is_empty());
    }

    #[test]
    fn piston_already_extended_no_change() {
        let tb = make_tb();
        let lever_on = tb.lever[0][1];
        let blocks = vec![
            (0, 0, 0, tb.piston[2]),
            (0, 0, 1, tb.piston_arm[2]),
            (0, 0, -1, lever_on),
        ];
        let update = process_piston_tick(0, 0, 0, &tb, world_from(&blocks, tb.air), |_| false);
        assert!(update.changes.is_empty());
    }

    #[test]
    fn non_piston_returns_empty() {
        let tb = make_tb();
        let update = process_piston_tick(
            0,
            0,
            0,
            &tb,
            |_, _, _| Some(tb.stone),
            |rid| rid == tb.stone,
        );
        assert!(update.changes.is_empty());
    }
}
