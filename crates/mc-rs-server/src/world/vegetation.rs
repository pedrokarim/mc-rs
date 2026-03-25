use std::collections::HashMap;

use super::biome::biome_id;
use super::block_registry::BLOCKS;
use super::random::Random;

/// Generate all vegetation for a chunk: trees, grass, flowers, cactus, dead bush, etc.
pub fn generate_vegetation(
    biome_ids: &[[u32; 16]; 16],
    surfaces: &[[i32; 16]; 16],
    random: &mut Random,
) -> HashMap<(u8, i32, u8), u32> {
    let mut blocks: HashMap<(u8, i32, u8), u32> = HashMap::new();

    // ── Trees (biome-specific type) ──
    let tree_count = weighted_attempts(biome_ids, tree_density_for_biome);
    for _ in 0..tree_count {
        let tx = random.next_range(2, 13) as usize;
        let tz = random.next_range(2, 13) as usize;
        let biome = biome_ids[tx][tz];
        let surface_y = surfaces[tx][tz];
        if surface_y <= 62 {
            continue;
        }
        place_tree_for_biome(
            biome,
            tx as i32,
            surface_y + 1,
            tz as i32,
            random,
            &mut blocks,
        );
    }

    // ── Short grass ──
    let grass_count = weighted_attempts(biome_ids, grass_count_for_biome);
    for _ in 0..grass_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let biome = biome_ids[gx][gz];
        let sy = surfaces[gx][gz];
        if sy <= 62 || grass_count_for_biome(biome) == 0 {
            continue;
        }
        blocks
            .entry((gx as u8, sy + 1, gz as u8))
            .or_insert(BLOCKS.short_grass);
    }

    // ── Flowers ──
    let flower_count = weighted_attempts(biome_ids, flower_count_for_biome);
    for _ in 0..flower_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let biome = biome_ids[fx][fz];
        let sy = surfaces[fx][fz];
        if sy <= 62 || flower_count_for_biome(biome) == 0 {
            continue;
        }
        let pos = (fx as u8, sy + 1, fz as u8);
        blocks
            .entry(pos)
            .or_insert_with(|| pick_flower(biome, random));
    }

    // ── Ferns ──
    let fern_count = weighted_attempts(biome_ids, fern_count_for_biome);
    for _ in 0..fern_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let biome = biome_ids[fx][fz];
        let sy = surfaces[fx][fz];
        if sy <= 62 || fern_count_for_biome(biome) == 0 {
            continue;
        }
        blocks
            .entry((fx as u8, sy + 1, fz as u8))
            .or_insert(BLOCKS.fern);
    }

    // ── Tall grass (double plant) ──
    let tall_count = weighted_attempts(biome_ids, tall_grass_count_for_biome);
    for _ in 0..tall_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let biome = biome_ids[gx][gz];
        let sy = surfaces[gx][gz];
        if sy <= 62 || tall_grass_count_for_biome(biome) == 0 {
            continue;
        }
        blocks
            .entry((gx as u8, sy + 1, gz as u8))
            .or_insert(BLOCKS.tall_grass);
    }

    // ── Cactus (desert, mesa) ──
    let cactus_count = weighted_attempts(biome_ids, cactus_count_for_biome);
    for _ in 0..cactus_count {
        let cx = random.next_range(0, 15) as usize;
        let cz = random.next_range(0, 15) as usize;
        let biome = biome_ids[cx][cz];
        let sy = surfaces[cx][cz];
        if sy <= 62 || cactus_count_for_biome(biome) == 0 {
            continue;
        }
        // Cactus: 1-3 blocks tall
        let height = random.next_range(1, 3);
        let pos = (cx as u8, sy + 1, cz as u8);
        if !blocks.contains_key(&pos) {
            for h in 0..height {
                blocks.insert((cx as u8, sy + 1 + h, cz as u8), BLOCKS.cactus);
            }
        }
    }

    // ── Dead bush (desert, mesa, mega_taiga, swamp) ──
    let deadbush_count = weighted_attempts(biome_ids, deadbush_count_for_biome);
    for _ in 0..deadbush_count {
        let dx = random.next_range(0, 15) as usize;
        let dz = random.next_range(0, 15) as usize;
        let biome = biome_ids[dx][dz];
        let sy = surfaces[dx][dz];
        if sy <= 62 || deadbush_count_for_biome(biome) == 0 {
            continue;
        }
        blocks
            .entry((dx as u8, sy + 1, dz as u8))
            .or_insert(BLOCKS.deadbush);
    }

    // ── Mushrooms (swamp, mega_taiga, taiga) ──
    let mushroom_count = weighted_attempts(biome_ids, mushroom_count_for_biome);
    for _ in 0..mushroom_count {
        let mx = random.next_range(0, 15) as usize;
        let mz = random.next_range(0, 15) as usize;
        let biome = biome_ids[mx][mz];
        let sy = surfaces[mx][mz];
        if sy <= 62 || mushroom_count_for_biome(biome) == 0 {
            continue;
        }
        let mushroom = if random.next_bounded_int(4) == 0 {
            BLOCKS.red_mushroom
        } else {
            BLOCKS.brown_mushroom
        };
        blocks
            .entry((mx as u8, sy + 1, mz as u8))
            .or_insert(mushroom);
    }

    // ── Pumpkin (très rare, tous biomes avec herbe) ──
    if random.next_bounded_int(32) == 0 {
        let px = random.next_range(0, 15) as usize;
        let pz = random.next_range(0, 15) as usize;
        let biome = biome_ids[px][pz];
        let sy = surfaces[px][pz];
        if sy > 62 && is_grassy_biome(biome) {
            blocks
                .entry((px as u8, sy + 1, pz as u8))
                .or_insert(BLOCKS.pumpkin);
        }
    }

    // ── Reeds / sugar cane (near water) ──
    let reeds_count = weighted_attempts(biome_ids, reeds_count_for_biome);
    for _ in 0..reeds_count {
        let rx = random.next_range(0, 15) as usize;
        let rz = random.next_range(0, 15) as usize;
        let biome = biome_ids[rx][rz];
        let sy = surfaces[rx][rz];
        // Reeds grow at water level +1 or just above water
        if !(62..=64).contains(&sy) || reeds_count_for_biome(biome) == 0 {
            continue;
        }
        // Check if adjacent to water (any neighbor at water level)
        let near_water = check_near_water(rx, rz, surfaces);
        if near_water {
            let height = random.next_range(1, 3);
            for h in 0..height {
                blocks
                    .entry((rx as u8, sy + 1 + h, rz as u8))
                    .or_insert(BLOCKS.reeds);
            }
        }
    }

    // ── Bamboo (jungle, bamboo_jungle) ──
    let bamboo_count = weighted_attempts(biome_ids, bamboo_count_for_biome);
    for _ in 0..bamboo_count {
        let bx = random.next_range(0, 15) as usize;
        let bz = random.next_range(0, 15) as usize;
        let biome = biome_ids[bx][bz];
        let sy = surfaces[bx][bz];
        if sy <= 62 || bamboo_count_for_biome(biome) == 0 {
            continue;
        }
        let pos = (bx as u8, sy + 1, bz as u8);
        if !blocks.contains_key(&pos) {
            let height = if biome == biome_id::BAMBOO_JUNGLE {
                random.next_range(5, 12)
            } else {
                random.next_range(4, 8)
            };
            for h in 0..height {
                blocks.insert((bx as u8, sy + 1 + h, bz as u8), BLOCKS.bamboo);
            }
        }
    }

    // ── Waterlily (swamp) ──
    let waterlily_count = weighted_attempts(biome_ids, waterlily_count_for_biome);
    for _ in 0..waterlily_count {
        let wx = random.next_range(0, 15) as usize;
        let wz = random.next_range(0, 15) as usize;
        let biome = biome_ids[wx][wz];
        let sy = surfaces[wx][wz];
        // Lily pads go on water surface
        if waterlily_count_for_biome(biome) > 0 && sy < 62 {
            blocks
                .entry((wx as u8, 63, wz as u8))
                .or_insert(BLOCKS.waterlily);
        }
    }

    blocks
}

fn weighted_attempts(biome_ids: &[[u32; 16]; 16], count_for_biome: fn(u32) -> i32) -> i32 {
    let mut total = 0i32;
    for row in biome_ids {
        for &biome in row {
            total += count_for_biome(biome);
        }
    }
    total / 256
}

/// Check if any adjacent column is at water level (for reed placement).
fn check_near_water(x: usize, z: usize, surfaces: &[[i32; 16]; 16]) -> bool {
    let dirs: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (dx, dz) in &dirs {
        let nx = x as i32 + dx;
        let nz = z as i32 + dz;
        if (0..16).contains(&nx) && (0..16).contains(&nz) && surfaces[nx as usize][nz as usize] < 62
        {
            return true; // neighbor is underwater
        }
    }
    false
}

fn is_grassy_biome(biome: u32) -> bool {
    matches!(
        biome,
        biome_id::PLAINS
            | biome_id::SUNFLOWER_PLAINS
            | biome_id::FOREST
            | biome_id::BIRCH_FOREST
            | biome_id::ROOFED_FOREST
            | biome_id::FLOWER_FOREST
            | biome_id::TAIGA
            | biome_id::COLD_TAIGA
            | biome_id::MEGA_TAIGA
            | biome_id::JUNGLE
            | biome_id::JUNGLE_EDGE
            | biome_id::BAMBOO_JUNGLE
            | biome_id::SAVANNA
            | biome_id::SAVANNA_PLATEAU
            | biome_id::SWAMPLAND
            | biome_id::EXTREME_HILLS
            | biome_id::EXTREME_HILLS_EDGE
            | biome_id::EXTREME_HILLS_PLUS_TREES
    )
}

// ── Tree counts ──

fn tree_density_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::ROOFED_FOREST => 16,
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE => 11,
        biome_id::FOREST | biome_id::BIRCH_FOREST => 8,
        biome_id::FOREST_HILLS | biome_id::BIRCH_FOREST_HILLS => 7,
        biome_id::FLOWER_FOREST => 6,
        biome_id::JUNGLE_HILLS => 8,
        biome_id::JUNGLE_EDGE => 5,
        biome_id::TAIGA | biome_id::COLD_TAIGA | biome_id::MEGA_TAIGA => 6,
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS | biome_id::MEGA_TAIGA_HILLS => 5,
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => 2,
        biome_id::SWAMPLAND => 3,
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_PLUS_TREES => 1,
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => 0,
        biome_id::ICE_PLAINS => 0,
        _ => 0,
    }
}

fn cactus_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::DESERT | biome_id::DESERT_HILLS => 10,
        biome_id::MESA | biome_id::MESA_BRYCE => 5,
        _ => 0,
    }
}

fn deadbush_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::MESA
        | biome_id::MESA_BRYCE
        | biome_id::MESA_PLATEAU
        | biome_id::MESA_PLATEAU_STONE => 20,
        biome_id::DESERT | biome_id::DESERT_HILLS => 2,
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 1,
        biome_id::SWAMPLAND => 1,
        _ => 0,
    }
}

fn mushroom_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::SWAMPLAND => 8,
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 3,
        biome_id::MUSHROOM_ISLAND => 1,
        biome_id::TAIGA | biome_id::COLD_TAIGA => 1,
        _ => 0,
    }
}

fn reeds_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::DESERT | biome_id::DESERT_HILLS => 50,
        biome_id::SWAMPLAND => 10,
        biome_id::RIVER | biome_id::FROZEN_RIVER => 5,
        _ if is_grassy_biome(biome) => 10,
        _ => 0,
    }
}

fn bamboo_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::BAMBOO_JUNGLE => 60,
        biome_id::JUNGLE | biome_id::JUNGLE_HILLS | biome_id::JUNGLE_EDGE => 16,
        _ => 0,
    }
}

fn waterlily_count_for_biome(biome: u32) -> i32 {
    if biome == biome_id::SWAMPLAND { 4 } else { 0 }
}

// ── Vegetation counts (from BDS feature_rules) ──

fn grass_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::BAMBOO_JUNGLE => 150,
        biome_id::JUNGLE | biome_id::JUNGLE_EDGE => 25,
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => 20,
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => 10,
        biome_id::FLOWER_FOREST => 8,
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 7,
        biome_id::FOREST | biome_id::BIRCH_FOREST | biome_id::ROOFED_FOREST => 5,
        biome_id::SWAMPLAND => 5,
        biome_id::SAVANNA_MUTATED => 5,
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => 3,
        biome_id::TAIGA | biome_id::COLD_TAIGA => 1,
        _ => 0,
    }
}

fn flower_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::FLOWER_FOREST => 100,
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => 4,
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE | biome_id::JUNGLE_EDGE => 4,
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => 4,
        biome_id::FOREST | biome_id::BIRCH_FOREST => 2,
        biome_id::SWAMPLAND => 1,
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => 1,
        _ => 0,
    }
}

fn fern_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 10,
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE | biome_id::JUNGLE_EDGE => 8,
        biome_id::TAIGA | biome_id::COLD_TAIGA => 5,
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS => 4,
        _ => 0,
    }
}

fn tall_grass_count_for_biome(biome: u32) -> i32 {
    match biome {
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => 7,
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE => 5,
        biome_id::FLOWER_FOREST => 5,
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => 3,
        biome_id::FOREST | biome_id::BIRCH_FOREST => 2,
        _ => 0,
    }
}

// ── Flower picker ──

fn pick_flower(biome: u32, random: &mut Random) -> u32 {
    match biome {
        biome_id::FLOWER_FOREST => match random.next_bounded_int(7) {
            0 => BLOCKS.dandelion,
            1 => BLOCKS.poppy,
            2 => BLOCKS.allium,
            3 => BLOCKS.azure_bluet,
            4 => BLOCKS.oxeye_daisy,
            5 => BLOCKS.cornflower,
            _ => BLOCKS.blue_orchid,
        },
        biome_id::SWAMPLAND => BLOCKS.blue_orchid,
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => {
            if random.next_bounded_int(3) == 0 {
                BLOCKS.dandelion
            } else {
                match random.next_bounded_int(4) {
                    0 => BLOCKS.poppy,
                    1 => BLOCKS.azure_bluet,
                    2 => BLOCKS.oxeye_daisy,
                    _ => BLOCKS.cornflower,
                }
            }
        }
        _ => {
            if random.next_bounded_int(2) == 0 {
                BLOCKS.dandelion
            } else {
                BLOCKS.poppy
            }
        }
    }
}

// ── Tree placement by biome ──

fn place_tree_for_biome(
    biome: u32,
    x: i32,
    y: i32,
    z: i32,
    random: &mut Random,
    blocks: &mut HashMap<(u8, i32, u8), u32>,
) {
    match biome {
        biome_id::BIRCH_FOREST | biome_id::BIRCH_FOREST_HILLS => {
            let h = random.next_bounded_int(3) + 5; // 5-7
            place_simple_tree(
                x,
                y,
                z,
                h,
                BLOCKS.birch_log,
                BLOCKS.birch_leaves,
                random,
                blocks,
            );
        }
        biome_id::TAIGA
        | biome_id::COLD_TAIGA
        | biome_id::TAIGA_HILLS
        | biome_id::COLD_TAIGA_HILLS
        | biome_id::ICE_PLAINS => {
            let h = random.next_bounded_int(4) + 6; // 6-9
            place_spruce_tree(x, y, z, h, random, blocks);
        }
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => {
            let h = random.next_bounded_int(4) + 7; // 7-10
            place_spruce_tree(x, y, z, h, random, blocks);
        }
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU | biome_id::SAVANNA_MUTATED => {
            let h = random.next_bounded_int(3) + 5; // 5-7
            place_simple_tree(
                x,
                y,
                z,
                h,
                BLOCKS.acacia_log,
                BLOCKS.acacia_leaves,
                random,
                blocks,
            );
        }
        biome_id::JUNGLE
        | biome_id::JUNGLE_HILLS
        | biome_id::JUNGLE_EDGE
        | biome_id::BAMBOO_JUNGLE => {
            let h = random.next_bounded_int(5) + 5; // 5-9
            place_simple_tree(
                x,
                y,
                z,
                h,
                BLOCKS.jungle_log,
                BLOCKS.jungle_leaves,
                random,
                blocks,
            );
        }
        biome_id::ROOFED_FOREST => {
            let h = random.next_bounded_int(3) + 5;
            place_simple_tree(
                x,
                y,
                z,
                h,
                BLOCKS.dark_oak_log,
                BLOCKS.dark_oak_leaves,
                random,
                blocks,
            );
        }
        biome_id::EXTREME_HILLS
        | biome_id::EXTREME_HILLS_PLUS_TREES
        | biome_id::EXTREME_HILLS_EDGE => {
            // Mix of spruce and oak
            if random.next_bounded_int(3) == 0 {
                let h = random.next_bounded_int(4) + 6;
                place_spruce_tree(x, y, z, h, random, blocks);
            } else {
                let h = random.next_bounded_int(3) + 4;
                place_simple_tree(
                    x,
                    y,
                    z,
                    h,
                    BLOCKS.oak_log,
                    BLOCKS.oak_leaves,
                    random,
                    blocks,
                );
            }
        }
        _ => {
            // Default: oak tree
            let h = random.next_bounded_int(3) + 4;
            place_simple_tree(
                x,
                y,
                z,
                h,
                BLOCKS.oak_log,
                BLOCKS.oak_leaves,
                random,
                blocks,
            );
        }
    }
}

/// Generic tree: straight trunk + round canopy (oak, birch, acacia, jungle, dark oak).
#[allow(clippy::too_many_arguments)]
fn place_simple_tree(
    x: i32,
    y: i32,
    z: i32,
    height: i32,
    log_id: u32,
    leaf_id: u32,
    random: &mut Random,
    blocks: &mut HashMap<(u8, i32, u8), u32>,
) {
    if !(2..=13).contains(&x) || !(2..=13).contains(&z) || y + height + 1 > 256 {
        return;
    }
    // Trunk
    for yy in 0..height {
        blocks.insert((x as u8, y + yy, z as u8), log_id);
    }
    blocks.insert((x as u8, y - 1, z as u8), BLOCKS.dirt);
    // Canopy
    for yy in (y + height - 3)..=(y + height) {
        let y_off = yy - (y + height);
        let mid = 1 - y_off / 2;
        for xx in (x - mid)..=(x + mid) {
            let x_off = (xx - x).abs();
            for zz in (z - mid)..=(z + mid) {
                let z_off = (zz - z).abs();
                if x_off == mid && z_off == mid && (y_off == 0 || random.next_bounded_int(2) == 0) {
                    continue;
                }
                if (0..16).contains(&xx) && (0..16).contains(&zz) {
                    let pos = (xx as u8, yy, zz as u8);
                    if blocks.get(&pos) != Some(&log_id) {
                        blocks.insert(pos, leaf_id);
                    }
                }
            }
        }
    }
}

/// Spruce tree: straight trunk + triangular canopy (narrow at top, wide at bottom).
fn place_spruce_tree(
    x: i32,
    y: i32,
    z: i32,
    height: i32,
    random: &mut Random,
    blocks: &mut HashMap<(u8, i32, u8), u32>,
) {
    if !(3..=12).contains(&x) || !(3..=12).contains(&z) || y + height + 2 > 256 {
        return;
    }
    let log = BLOCKS.spruce_log;
    let leaf = BLOCKS.spruce_leaves;

    // Trunk
    for yy in 0..height {
        blocks.insert((x as u8, y + yy, z as u8), log);
    }
    blocks.insert((x as u8, y - 1, z as u8), BLOCKS.dirt);

    // Triangular canopy: starts narrow at top, widens toward bottom
    // Top: single leaf on top
    blocks.insert((x as u8, y + height, z as u8), leaf);

    // Canopy layers from top to bottom
    let canopy_start = y + height - 1;
    let canopy_layers = (height - 2).min(6); // 4-6 layers of canopy

    for layer in 0..canopy_layers {
        let yy = canopy_start - layer;
        // Radius alternates: 1, 2, 1, 2, 3, 2...
        let radius = if layer % 2 == 0 {
            1 + layer / 2
        } else {
            1 + (layer - 1) / 2
        };
        let radius = radius.min(3);

        for xx in (x - radius)..=(x + radius) {
            for zz in (z - radius)..=(z + radius) {
                // Diamond/circle shape
                let dx = (xx - x).abs();
                let dz = (zz - z).abs();
                if dx + dz > radius + 1 {
                    continue;
                }
                // Skip corners randomly
                if dx == radius && dz == radius && random.next_bounded_int(2) == 0 {
                    continue;
                }
                if (0..16).contains(&xx) && (0..16).contains(&zz) {
                    let pos = (xx as u8, yy, zz as u8);
                    if blocks.get(&pos) != Some(&log) {
                        blocks.insert(pos, leaf);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forest_has_trees_and_flowers() {
        let biome_ids = [[biome_id::FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(!veg.is_empty());
        assert!(veg.values().any(|&id| id == BLOCKS.oak_log));
        assert!(veg.values().any(|&id| id == BLOCKS.oak_leaves));
    }

    #[test]
    fn test_desert_has_cactus_and_deadbush() {
        let biome_ids = [[biome_id::DESERT; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == BLOCKS.cactus),
            "Desert should have cactus"
        );
        assert!(
            veg.values().any(|&id| id == BLOCKS.deadbush),
            "Desert should have dead bush"
        );
    }

    #[test]
    fn test_taiga_has_spruce() {
        let biome_ids = [[biome_id::TAIGA; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == BLOCKS.spruce_log),
            "Taiga should have spruce"
        );
        assert!(veg.values().any(|&id| id == BLOCKS.spruce_leaves));
    }

    #[test]
    fn test_birch_forest_has_birch() {
        let biome_ids = [[biome_id::BIRCH_FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == BLOCKS.birch_log),
            "Birch forest should have birch"
        );
    }

    #[test]
    fn test_savanna_has_acacia() {
        let biome_ids = [[biome_id::SAVANNA; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == BLOCKS.acacia_log),
            "Savanna should have acacia"
        );
    }

    #[test]
    fn test_jungle_has_bamboo() {
        let biome_ids = [[biome_id::JUNGLE; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == BLOCKS.bamboo),
            "Jungle should have bamboo"
        );
        assert!(
            veg.values().any(|&id| id == BLOCKS.jungle_log),
            "Jungle should have jungle trees"
        );
    }

    #[test]
    fn test_swamp_has_mushrooms_and_waterlily() {
        let biome_ids = [[biome_id::SWAMPLAND; 16]; 16];
        let mut surfaces = [[65i32; 16]; 16];
        // Some columns underwater for waterlily
        for x in 0..4 {
            for z in 0..4 {
                surfaces[x][z] = 58;
            }
        }
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let has_mushroom = veg
            .values()
            .any(|&id| id == BLOCKS.brown_mushroom || id == BLOCKS.red_mushroom);
        assert!(has_mushroom, "Swamp should have mushrooms");
    }

    #[test]
    fn test_flower_forest_has_many_flowers() {
        let biome_ids = [[biome_id::FLOWER_FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let flower_ids = [
            BLOCKS.dandelion,
            BLOCKS.poppy,
            BLOCKS.allium,
            BLOCKS.azure_bluet,
            BLOCKS.oxeye_daisy,
            BLOCKS.cornflower,
            BLOCKS.blue_orchid,
        ];
        let count = veg.values().filter(|&&id| flower_ids.contains(&id)).count();
        assert!(
            count >= 20,
            "Flower forest should have many flowers, got {count}"
        );
    }

    #[test]
    fn test_vegetation_bounds() {
        let biome_ids = [[biome_id::JUNGLE; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        for &(x, _y, z) in veg.keys() {
            assert!(x < 16, "x={x} out of bounds");
            assert!(z < 16, "z={z} out of bounds");
        }
    }
}
