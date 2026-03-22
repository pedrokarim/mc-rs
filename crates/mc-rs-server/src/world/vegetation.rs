use std::collections::HashMap;

use super::biome::biome_id;
use super::flat_generator::block_ids;
use super::random::Random;
use super::terrain_generator::extra_blocks;

/// Generate vegetation (trees, flowers, grass, etc.) for a chunk.
/// Returns a map of (local_x, world_y, local_z) -> block ID.
pub fn generate_vegetation(
    biome_ids: &[[u32; 16]; 16],
    surfaces: &[[i32; 16]; 16],
    random: &mut Random,
) -> HashMap<(u8, i32, u8), u32> {
    let mut blocks: HashMap<(u8, i32, u8), u32> = HashMap::new();

    let center_biome = biome_ids[8][8];

    // ── Trees ──
    let tree_count = tree_count_for_biome(center_biome, random);
    for _ in 0..tree_count {
        let tx = random.next_range(2, 13) as usize;
        let tz = random.next_range(2, 13) as usize;
        let surface_y = surfaces[tx][tz];
        if surface_y <= 62 {
            continue;
        }
        let tree_height = random.next_bounded_int(3) + 4;
        place_oak_tree(
            tx as i32,
            surface_y + 1,
            tz as i32,
            tree_height,
            random,
            &mut blocks,
        );
    }

    // ── Short grass ──
    let grass_count = grass_count_for_biome(center_biome, random);
    for _ in 0..grass_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let surface_y = surfaces[gx][gz];
        if surface_y <= 62 {
            continue;
        }
        let pos = (gx as u8, surface_y + 1, gz as u8);
        blocks.entry(pos).or_insert(extra_blocks::SHORT_GRASS);
    }

    // ── Flowers ──
    let flower_count = flower_count_for_biome(center_biome, random);
    for _ in 0..flower_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let surface_y = surfaces[fx][fz];
        if surface_y <= 62 {
            continue;
        }
        let pos = (fx as u8, surface_y + 1, fz as u8);
        blocks
            .entry(pos)
            .or_insert_with(|| pick_flower(center_biome, random));
    }

    // ── Ferns (taiga, jungle, mega_taiga) ──
    let fern_count = fern_count_for_biome(center_biome, random);
    for _ in 0..fern_count {
        let fx = random.next_range(0, 15) as usize;
        let fz = random.next_range(0, 15) as usize;
        let surface_y = surfaces[fx][fz];
        if surface_y <= 62 {
            continue;
        }
        let pos = (fx as u8, surface_y + 1, fz as u8);
        blocks.entry(pos).or_insert(extra_blocks::FERN);
    }

    // ── Tall grass (double height) ──
    let tall_grass_count = tall_grass_count_for_biome(center_biome, random);
    for _ in 0..tall_grass_count {
        let gx = random.next_range(0, 15) as usize;
        let gz = random.next_range(0, 15) as usize;
        let surface_y = surfaces[gx][gz];
        if surface_y <= 62 {
            continue;
        }
        let pos = (gx as u8, surface_y + 1, gz as u8);
        blocks.entry(pos).or_insert(extra_blocks::TALL_GRASS);
    }

    blocks
}

/// Number of trees per chunk based on biome.
fn tree_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::FOREST | biome_id::BIRCH_FOREST | biome_id::ROOFED_FOREST => {
            random.next_range(6, 10)
        }
        biome_id::FOREST_HILLS | biome_id::BIRCH_FOREST_HILLS => random.next_range(5, 9),
        biome_id::FLOWER_FOREST => random.next_range(4, 8),
        biome_id::TAIGA | biome_id::COLD_TAIGA | biome_id::MEGA_TAIGA => random.next_range(5, 8),
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS | biome_id::MEGA_TAIGA_HILLS => {
            random.next_range(4, 7)
        }
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE | biome_id::JUNGLE_EDGE => {
            random.next_range(8, 14)
        }
        biome_id::JUNGLE_HILLS => random.next_range(6, 10),
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => {
            if random.next_bounded_int(5) == 0 {
                1
            } else {
                0
            }
        }
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => random.next_range(1, 3),
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_PLUS_TREES => random.next_range(0, 3),
        biome_id::SWAMPLAND => random.next_range(2, 4),
        _ => 0,
    }
}

/// Number of short grass per chunk.
fn grass_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => random.next_range(20, 40),
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => random.next_range(15, 30),
        biome_id::FOREST | biome_id::BIRCH_FOREST | biome_id::ROOFED_FOREST => {
            random.next_range(5, 15)
        }
        biome_id::FLOWER_FOREST => random.next_range(8, 20),
        biome_id::TAIGA | biome_id::COLD_TAIGA | biome_id::MEGA_TAIGA => random.next_range(3, 10),
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE | biome_id::JUNGLE_EDGE => {
            random.next_range(10, 25)
        }
        biome_id::SWAMPLAND => random.next_range(8, 16),
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => random.next_range(3, 8),
        _ => 0,
    }
}

/// Number of flowers per chunk.
fn flower_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::FLOWER_FOREST => random.next_range(15, 30),
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => random.next_range(2, 6),
        biome_id::FOREST | biome_id::BIRCH_FOREST => random.next_range(1, 4),
        biome_id::SWAMPLAND => random.next_range(0, 2),
        biome_id::SAVANNA => random.next_range(0, 2),
        biome_id::EXTREME_HILLS | biome_id::EXTREME_HILLS_EDGE => random.next_range(0, 2),
        _ => 0,
    }
}

/// Number of ferns per chunk.
fn fern_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::TAIGA | biome_id::COLD_TAIGA => random.next_range(5, 12),
        biome_id::MEGA_TAIGA | biome_id::MEGA_TAIGA_HILLS => random.next_range(8, 16),
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE | biome_id::JUNGLE_EDGE => {
            random.next_range(6, 14)
        }
        biome_id::TAIGA_HILLS | biome_id::COLD_TAIGA_HILLS => random.next_range(3, 8),
        _ => 0,
    }
}

/// Number of tall grass (double plant) per chunk.
fn tall_grass_count_for_biome(biome: u32, random: &mut Random) -> i32 {
    match biome {
        biome_id::PLAINS | biome_id::SUNFLOWER_PLAINS => random.next_range(3, 8),
        biome_id::SAVANNA | biome_id::SAVANNA_PLATEAU => random.next_range(2, 5),
        biome_id::FLOWER_FOREST => random.next_range(2, 6),
        biome_id::FOREST | biome_id::BIRCH_FOREST => random.next_range(1, 3),
        biome_id::JUNGLE | biome_id::BAMBOO_JUNGLE => random.next_range(3, 8),
        _ => 0,
    }
}

/// Pick a random flower type based on biome.
fn pick_flower(biome: u32, random: &mut Random) -> u32 {
    match biome {
        biome_id::FLOWER_FOREST => {
            // Flower forest: all types equally
            match random.next_bounded_int(7) {
                0 => extra_blocks::DANDELION,
                1 => extra_blocks::POPPY,
                2 => extra_blocks::ALLIUM,
                3 => extra_blocks::AZURE_BLUET,
                4 => extra_blocks::OXEYE_DAISY,
                5 => extra_blocks::CORNFLOWER,
                _ => extra_blocks::BLUE_ORCHID,
            }
        }
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
            // Default: dandelion or poppy
            if random.next_bounded_int(2) == 0 {
                extra_blocks::DANDELION
            } else {
                extra_blocks::POPPY
            }
        }
    }
}

/// Place an oak tree at the given position.
fn place_oak_tree(
    x: i32,
    y: i32,
    z: i32,
    height: i32,
    random: &mut Random,
    blocks: &mut HashMap<(u8, i32, u8), u32>,
) {
    if !(2..=13).contains(&x) || !(2..=13).contains(&z) {
        return;
    }
    if y + height + 1 > 256 {
        return;
    }

    // Place trunk
    for yy in 0..height {
        blocks.insert((x as u8, y + yy, z as u8), extra_blocks::OAK_LOG);
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

                if x_off == mid && z_off == mid && (y_off == 0 || random.next_bounded_int(2) == 0) {
                    continue;
                }

                if (0..16).contains(&xx) && (0..16).contains(&zz) {
                    let pos = (xx as u8, yy, zz as u8);
                    if blocks.get(&pos) != Some(&extra_blocks::OAK_LOG) {
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
    fn test_forest_has_trees_and_flowers() {
        let biome_ids = [[biome_id::FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        assert!(!veg.is_empty());

        let has_logs = veg.values().any(|&id| id == extra_blocks::OAK_LOG);
        let has_leaves = veg.values().any(|&id| id == extra_blocks::OAK_LEAVES);
        let has_grass = veg.values().any(|&id| id == extra_blocks::SHORT_GRASS);
        assert!(has_logs, "Expected oak logs in forest");
        assert!(has_leaves, "Expected oak leaves in forest");
        assert!(has_grass, "Expected grass in forest");
    }

    #[test]
    fn test_plains_has_lots_of_grass() {
        let biome_ids = [[biome_id::PLAINS; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let grass_count = veg
            .values()
            .filter(|&&id| id == extra_blocks::SHORT_GRASS)
            .count();
        assert!(
            grass_count >= 10,
            "Plains should have lots of grass, got {grass_count}"
        );
    }

    #[test]
    fn test_flower_forest_has_flowers() {
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
        let flower_count = veg.values().filter(|&&id| flower_ids.contains(&id)).count();
        assert!(
            flower_count >= 5,
            "Flower forest should have many flowers, got {flower_count}"
        );
    }

    #[test]
    fn test_no_vegetation_in_desert() {
        let biome_ids = [[biome_id::DESERT; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let has_logs = veg.values().any(|&id| id == extra_blocks::OAK_LOG);
        let has_grass = veg.values().any(|&id| id == extra_blocks::SHORT_GRASS);
        assert!(!has_logs, "Desert should not have trees");
        assert!(!has_grass, "Desert should not have grass");
    }

    #[test]
    fn test_taiga_has_ferns() {
        let biome_ids = [[biome_id::TAIGA; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        let has_ferns = veg.values().any(|&id| id == extra_blocks::FERN);
        assert!(has_ferns, "Taiga should have ferns");
    }

    #[test]
    fn test_vegetation_bounds() {
        let biome_ids = [[biome_id::FOREST; 16]; 16];
        let surfaces = [[65i32; 16]; 16];
        let mut rng = Random::new(42);

        let veg = generate_vegetation(&biome_ids, &surfaces, &mut rng);
        for &(x, _y, z) in veg.keys() {
            assert!(x < 16, "x={x} out of bounds");
            assert!(z < 16, "z={z} out of bounds");
        }
    }
}
