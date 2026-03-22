use std::collections::HashMap;

use super::biome::biome_id;
use super::flat_generator::block_ids;
use super::random::Random;
use super::terrain_generator::extra_blocks;

/// Generate vegetation (trees, tall grass) for a chunk.
/// Returns a map of (local_x, world_y, local_z) -> block ID.
/// Only generates blocks within the chunk (0..16, 0..16).
pub fn generate_vegetation(
    biome_ids: &[[u32; 16]; 16],
    surfaces: &[[i32; 16]; 16],
    random: &mut Random,
) -> HashMap<(u8, i32, u8), u32> {
    let mut blocks: HashMap<(u8, i32, u8), u32> = HashMap::new();

    // Determine tree count based on center biome
    let center_biome = biome_ids[8][8];
    let tree_count = match center_biome {
        biome_id::FOREST | biome_id::BIRCH_FOREST => random.next_range(4, 8),
        biome_id::TAIGA => random.next_range(3, 6),
        biome_id::PLAINS => {
            if random.next_bounded_int(5) == 0 {
                1
            } else {
                0
            }
        }
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => random.next_range(0, 3),
        biome_id::SWAMPLAND => random.next_range(1, 3),
        _ => 0, // No trees in ocean, desert, river, ice_plains
    };

    // Place trees
    for _ in 0..tree_count {
        let tx = random.next_range(2, 13) as usize; // Keep away from edges
        let tz = random.next_range(2, 13) as usize;
        let surface_y = surfaces[tx][tz];

        // Only place on grass blocks (surface must be above water)
        if surface_y <= 62 {
            continue;
        }

        let tree_height = random.next_bounded_int(3) + 4; // 4-6
        place_oak_tree(
            tx as i32,
            surface_y + 1,
            tz as i32,
            tree_height,
            random,
            &mut blocks,
        );
    }

    // Tall grass for grassy biomes
    let grass_count = match center_biome {
        biome_id::PLAINS => random.next_range(8, 16),
        biome_id::FOREST | biome_id::BIRCH_FOREST => random.next_range(2, 6),
        biome_id::TAIGA => random.next_range(1, 4),
        biome_id::SWAMPLAND => random.next_range(4, 8),
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => random.next_range(1, 4),
        _ => 0,
    };

    let tall_grass_id = extra_blocks::SHORT_GRASS;

    for _ in 0..grass_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let surface_y = surfaces[gx][gz];

        // Only on grass blocks above water
        if surface_y <= 62 {
            continue;
        }

        // Don't place on top of trees
        let pos = (gx as u8, surface_y + 1, gz as u8);
        blocks.entry(pos).or_insert(tall_grass_id);
    }

    blocks
}

/// Place an oak tree at the given position.
/// Port of PMMP's Tree + OakTree classes.
fn place_oak_tree(
    x: i32,
    y: i32,
    z: i32,
    height: i32,
    random: &mut Random,
    blocks: &mut HashMap<(u8, i32, u8), u32>,
) {
    // Check bounds: tree + canopy must fit within chunk
    if !(2..=13).contains(&x) || !(2..=13).contains(&z) {
        return;
    }
    // Ensure tree doesn't go above world height limit
    if y + height + 1 > 256 {
        return;
    }

    // Place trunk
    for yy in 0..height {
        let pos = (x as u8, y + yy, z as u8);
        blocks.insert(pos, extra_blocks::OAK_LOG);
    }

    // Place dirt under trunk
    blocks.insert((x as u8, y - 1, z as u8), block_ids::DIRT);

    // Place canopy (leaves)
    for yy in (y + height - 3)..=(y + height) {
        let y_off = yy - (y + height);
        let mid = 1 - y_off / 2;

        for xx in (x - mid)..=(x + mid) {
            let x_off = (xx - x).abs();
            for zz in (z - mid)..=(z + mid) {
                let z_off = (zz - z).abs();

                // Skip some corners for natural look
                if x_off == mid && z_off == mid && (y_off == 0 || random.next_bounded_int(2) == 0) {
                    continue;
                }

                // Only place in valid chunk range and don't override trunk
                if (0..16).contains(&xx) && (0..16).contains(&zz) {
                    let pos = (xx as u8, yy, zz as u8);
                    if !blocks.contains_key(&pos) || blocks[&pos] != extra_blocks::OAK_LOG {
                        blocks.insert(pos, extra_blocks::OAK_LEAVES);
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
    fn test_tree_generation() {
        let biome_ids = [[biome_id::FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(!veg.is_empty(), "Expected vegetation in forest biome");

        // Should have logs and leaves
        let has_logs = veg.values().any(|&id| id == extra_blocks::OAK_LOG);
        let has_leaves = veg.values().any(|&id| id == extra_blocks::OAK_LEAVES);
        assert!(has_logs, "Expected oak logs");
        assert!(has_leaves, "Expected oak leaves");
    }

    #[test]
    fn test_no_trees_in_desert() {
        let biome_ids = [[biome_id::DESERT; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let has_logs = veg.values().any(|&id| id == extra_blocks::OAK_LOG);
        assert!(!has_logs, "Should not have trees in desert");
    }

    #[test]
    fn test_vegetation_bounds() {
        let biome_ids = [[biome_id::FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        for &(x, _y, z) in veg.keys() {
            assert!(x < 16, "Vegetation x={x} out of bounds");
            assert!(z < 16, "Vegetation z={z} out of bounds");
        }
    }
}
