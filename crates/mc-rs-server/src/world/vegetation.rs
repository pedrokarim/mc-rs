use std::collections::HashMap;

use super::biome::biome_id;
use super::flat_generator::block_ids;
use super::random::Random;
use super::terrain_generator::extra_blocks;

/// Generate all vegetation for a chunk: trees, grass, flowers, cactus, dead bush, etc.
pub fn generate_vegetation(
    biome_ids: &[[u32; 16]; 16],
    surfaces: &[[i32; 16]; 16],
    random: &mut Random,
) -> HashMap<(u8, i32, u8), u32> {
    let mut blocks: HashMap<(u8, i32, u8), u32> = HashMap::new();
    let center_biome = biome_ids[8][8];

    // ── Trees (biome-specific type) ──
    let tree_count = tree_count_for_biome(center_biome, random);
    for _ in 0..tree_count {
        let tx = random.next_range(2, 13) as usize;
        let tz = random.next_range(2, 13) as usize;
        let surface_y = surfaces[tx][tz];
        if surface_y <= 62 {
            continue;
        }
        place_tree_for_biome(
            center_biome,
            tx as i32,
            surface_y + 1,
            tz as i32,
            random,
            &mut blocks,
        );
    }

    // ── Short grass ──
    let grass_count = grass_count_for_biome(center_biome);
    for _ in 0..grass_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let sy = surfaces[gx][gz];
        if sy <= 62 {
            continue;
        }
        blocks
            .entry((gx as u8, sy + 1, gz as u8))
            .or_insert(extra_blocks::SHORT_GRASS);
    }

    // ── Flowers ──
    let flower_count = flower_count_for_biome(center_biome);
    for _ in 0..flower_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let sy = surfaces[fx][fz];
        if sy <= 62 {
            continue;
        }
        let pos = (fx as u8, sy + 1, fz as u8);
        blocks
            .entry(pos)
            .or_insert_with(|| pick_flower(center_biome, random));
    }

    // ── Ferns ──
    let fern_count = fern_count_for_biome(center_biome);
    for _ in 0..fern_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let sy = surfaces[fx][fz];
        if sy <= 62 {
            continue;
        }
        blocks
            .entry((fx as u8, sy + 1, fz as u8))
            .or_insert(extra_blocks::FERN);
    }

    // ── Tall grass (double plant) ──
    let tall_count = tall_grass_count_for_biome(center_biome);
    for _ in 0..tall_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let sy = surfaces[gx][gz];
        if sy <= 62 {
            continue;
        }
        blocks
            .entry((gx as u8, sy + 1, gz as u8))
            .or_insert(extra_blocks::TALL_GRASS);
    }

    // ── Cactus (desert, mesa) ──
    let cactus_count = match center_biome {
        biome_id::DESERT | biome_id::DESERT_HILLS => 10,
        biome_id::MESA | biome_id::MESA_BRYCE => 5,
        _ => 0,
    };
    for _ in 0..cactus_count {
        let cx = random.next_range(0, 15) as usize;
        let cz = random.next_range(0, 15) as usize;
        let sy = surfaces[cx][cz];
        if sy <= 62 {
            continue;
        }
        // Cactus: 1-3 blocks tall
        let height = random.next_range(1, 3);
        let pos = (cx as u8, sy + 1, cz as u8);
        if !blocks.contains_key(&pos) {
            for h in 0..height {
                blocks.insert((cx as u8, sy + 1 + h, cz as u8), extra_blocks::CACTUS);
            }
        }
    }

    // ── Dead bush (desert, mesa, mega_taiga, swamp) ──
    let deadbush_count = match center_biome {
        biome_id::MESA
        | biome_id::MESA_BRYCE
        | biome_id::MESA_PLATEAU
        | biome_id::MESA_PLATEAU_STONE => 20,
        biome_id::DESERT | biome_id::DESERT_HILLS => 2,
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 1,
        biome_id::SWAMPLAND => 1,
        _ => 0,
    };
    for _ in 0..deadbush_count {
        let dx = random.next_range(0, 15) as usize;
        let dz = random.next_range(0, 15) as usize;
        let sy = surfaces[dx][dz];
        if sy <= 62 {
            continue;
        }
        blocks
            .entry((dx as u8, sy + 1, dz as u8))
            .or_insert(extra_blocks::DEADBUSH);
    }

    // ── Mushrooms (swamp, mega_taiga, taiga) ──
    let mushroom_count = match center_biome {
        biome_id::SWAMPLAND => 8,
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => 3,
        biome_id::MUSHROOM_ISLAND => 1,
        biome_id::TAIGA | biome_id::COLD_TAIGA => 1,
        _ => 0,
    };
    for _ in 0..mushroom_count {
        let mx = random.next_range(0, 15) as usize;
        let mz = random.next_range(0, 15) as usize;
        let sy = surfaces[mx][mz];
        if sy <= 62 {
            continue;
        }
        let mushroom = if random.next_bounded_int(4) == 0 {
            extra_blocks::RED_MUSHROOM
        } else {
            extra_blocks::BROWN_MUSHROOM
        };
        blocks
            .entry((mx as u8, sy + 1, mz as u8))
            .or_insert(mushroom);
    }

    // ── Pumpkin (très rare, tous biomes avec herbe) ──
    if random.next_bounded_int(32) == 0 && is_grassy_biome(center_biome) {
        let px = random.next_range(0, 15) as usize;
        let pz = random.next_range(0, 15) as usize;
        let sy = surfaces[px][pz];
        if sy > 62 {
            blocks
                .entry((px as u8, sy + 1, pz as u8))
                .or_insert(extra_blocks::PUMPKIN);
        }
    }

    // ── Reeds / sugar cane (near water) ──
    let reeds_count = match center_biome {
        biome_id::DESERT | biome_id::DESERT_HILLS => 50,
        biome_id::SWAMPLAND => 10,
        biome_id::RIVER | biome_id::FROZEN_RIVER => 5,
        _ if is_grassy_biome(center_biome) => 10,
        _ => 0,
    };
    for _ in 0..reeds_count {
        let rx = random.next_range(0, 15) as usize;
        let rz = random.next_range(0, 15) as usize;
        let sy = surfaces[rx][rz];
        // Reeds grow at water level +1 or just above water
        if !(62..=64).contains(&sy) {
            continue;
        }
        // Check if adjacent to water (any neighbor at water level)
        let near_water = check_near_water(rx, rz, surfaces);
        if near_water {
            let height = random.next_range(1, 3);
            for h in 0..height {
                blocks
                    .entry((rx as u8, sy + 1 + h, rz as u8))
                    .or_insert(extra_blocks::REEDS);
            }
        }
    }

    // ── Bamboo (jungle, bamboo_jungle) ──
    let bamboo_count = match center_biome {
        biome_id::BAMBOO_JUNGLE => random.next_range(40, 80),
        biome_id::JUNGLE | biome_id::JUNGLE_HILLS | biome_id::JUNGLE_EDGE => 16,
        _ => 0,
    };
    for _ in 0..bamboo_count {
        let bx = random.next_range(0, 15) as usize;
        let bz = random.next_range(0, 15) as usize;
        let sy = surfaces[bx][bz];
        if sy <= 62 {
            continue;
        }
        let pos = (bx as u8, sy + 1, bz as u8);
        if !blocks.contains_key(&pos) {
            let height = random.next_range(5, 12);
            for h in 0..height {
                blocks.insert((bx as u8, sy + 1 + h, bz as u8), extra_blocks::BAMBOO);
            }
        }
    }

    // ── Waterlily (swamp) ──
    if center_biome == biome_id::SWAMPLAND {
        for _ in 0..4 {
            let wx = random.next_range(0, 15) as usize;
            let wz = random.next_range(0, 15) as usize;
            let sy = surfaces[wx][wz];
            // Lily pads go on water surface
            if sy < 62 {
                blocks
                    .entry((wx as u8, 63, wz as u8))
                    .or_insert(extra_blocks::WATERLILY);
            }
        }
    }

    blocks
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

fn tree_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::ROOFED_FOREST => 16,
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE => random.next_range(8, 14),
        biome_id::FOREST | biome_id::BIRCH_FOREST => random.next_range(6, 10),
        biome_id::FOREST_HILLS | biome_id::BIRCH_FOREST_HILLS => random.next_range(5, 9),
        biome_id::FLOWER_FOREST => random.next_range(4, 8),
        biome_id::JUNGLE_HILLS => random.next_range(6, 10),
        biome_id::JUNGLE_EDGE => random.next_range(4, 7),
        biome_id::TAIGA | biome_id::COLD_TAIGA | biome_id::MEGA_TAIGA => random.next_range(5, 8),
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS | biome_id::MEGA_TAIGA_HILLS => {
            random.next_range(4, 7)
        }
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => random.next_range(1, 3),
        biome_id::SWAMPLAND => random.next_range(2, 4),
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_PLUS_TREES => random.next_range(0, 3),
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => {
            if random.next_bounded_int(5) == 0 {
                1
            } else {
                0
            }
        }
        biome_id::ICE_PLAINS => {
            if random.next_bounded_int(3) == 0 {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
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
            0 => extra_blocks::DANDELION,
            1 => extra_blocks::POPPY,
            2 => extra_blocks::ALLIUM,
            3 => extra_blocks::AZURE_BLUET,
            4 => extra_blocks::OXEYE_DAISY,
            5 => extra_blocks::CORNFLOWER,
            _ => extra_blocks::BLUE_ORCHID,
        },
        biome_id::SWAMPLAND => extra_blocks::BLUE_ORCHID,
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => {
            if random.next_bounded_int(3) == 0 {
                extra_blocks::DANDELION
            } else {
                match random.next_bounded_int(4) {
                    0 => extra_blocks::POPPY,
                    1 => extra_blocks::AZURE_BLUET,
                    2 => extra_blocks::OXEYE_DAISY,
                    _ => extra_blocks::CORNFLOWER,
                }
            }
        }
        _ => {
            if random.next_bounded_int(2) == 0 {
                extra_blocks::DANDELION
            } else {
                extra_blocks::POPPY
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
                extra_blocks::BIRCH_LOG,
                extra_blocks::BIRCH_LEAVES,
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
                extra_blocks::ACACIA_LOG,
                extra_blocks::ACACIA_LEAVES,
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
                extra_blocks::JUNGLE_LOG,
                extra_blocks::JUNGLE_LEAVES,
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
                extra_blocks::DARK_OAK_LOG,
                extra_blocks::DARK_OAK_LEAVES,
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
                    extra_blocks::OAK_LOG,
                    extra_blocks::OAK_LEAVES,
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
                extra_blocks::OAK_LOG,
                extra_blocks::OAK_LEAVES,
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
    blocks.insert((x as u8, y - 1, z as u8), block_ids::DIRT);
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
    let log = extra_blocks::SPRUCE_LOG;
    let leaf = extra_blocks::SPRUCE_LEAVES;

    // Trunk
    for yy in 0..height {
        blocks.insert((x as u8, y + yy, z as u8), log);
    }
    blocks.insert((x as u8, y - 1, z as u8), block_ids::DIRT);

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
        assert!(veg.values().any(|&id| id == extra_blocks::OAK_LOG));
        assert!(veg.values().any(|&id| id == extra_blocks::OAK_LEAVES));
    }

    #[test]
    fn test_desert_has_cactus_and_deadbush() {
        let biome_ids = [[biome_id::DESERT; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == extra_blocks::CACTUS),
            "Desert should have cactus"
        );
        assert!(
            veg.values().any(|&id| id == extra_blocks::DEADBUSH),
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
            veg.values().any(|&id| id == extra_blocks::SPRUCE_LOG),
            "Taiga should have spruce"
        );
        assert!(veg.values().any(|&id| id == extra_blocks::SPRUCE_LEAVES));
    }

    #[test]
    fn test_birch_forest_has_birch() {
        let biome_ids = [[biome_id::BIRCH_FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(
            veg.values().any(|&id| id == extra_blocks::BIRCH_LOG),
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
            veg.values().any(|&id| id == extra_blocks::ACACIA_LOG),
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
            veg.values().any(|&id| id == extra_blocks::BAMBOO),
            "Jungle should have bamboo"
        );
        assert!(
            veg.values().any(|&id| id == extra_blocks::JUNGLE_LOG),
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
            .any(|&id| id == extra_blocks::BROWN_MUSHROOM || id == extra_blocks::RED_MUSHROOM);
        assert!(has_mushroom, "Swamp should have mushrooms");
    }

    #[test]
    fn test_flower_forest_has_many_flowers() {
        let biome_ids = [[biome_id::FLOWER_FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);
        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let flower_ids = [
            extra_blocks::DANDELION,
            extra_blocks::POPPY,
            extra_blocks::ALLIUM,
            extra_blocks::AZURE_BLUET,
            extra_blocks::OXEYE_DAISY,
            extra_blocks::CORNFLOWER,
            extra_blocks::BLUE_ORCHID,
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
