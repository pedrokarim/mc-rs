use std::collections::HashMap;
use std::f64::consts::PI;

use super::random::Random;
use super::terrain_generator::extra_blocks;

/// Ore type definition matching PMMP's OreType.
pub struct OreType {
    pub block_id: u32,
    pub cluster_count: i32,
    pub cluster_size: i32,
    pub min_height: i32,
    pub max_height: i32,
}

/// Default ore types matching PMMP's Normal generator.
pub fn default_ore_types() -> Vec<OreType> {
    vec![
        OreType {
            block_id: extra_blocks::COAL_ORE,
            cluster_count: 20,
            cluster_size: 16,
            min_height: 0,
            max_height: 128,
        },
        OreType {
            block_id: extra_blocks::IRON_ORE,
            cluster_count: 20,
            cluster_size: 8,
            min_height: 0,
            max_height: 64,
        },
        OreType {
            block_id: extra_blocks::REDSTONE_ORE,
            cluster_count: 8,
            cluster_size: 7,
            min_height: 0,
            max_height: 16,
        },
        OreType {
            block_id: extra_blocks::LAPIS_ORE,
            cluster_count: 1,
            cluster_size: 6,
            min_height: 0,
            max_height: 32,
        },
        OreType {
            block_id: extra_blocks::GOLD_ORE,
            cluster_count: 2,
            cluster_size: 8,
            min_height: 0,
            max_height: 32,
        },
        OreType {
            block_id: extra_blocks::DIAMOND_ORE,
            cluster_count: 1,
            cluster_size: 7,
            min_height: 0,
            max_height: 16,
        },
        // Dirt and gravel pockets in stone
        OreType {
            block_id: block_ids::DIRT,
            cluster_count: 20,
            cluster_size: 32,
            min_height: 0,
            max_height: 128,
        },
        OreType {
            block_id: extra_blocks::GRAVEL,
            cluster_count: 10,
            cluster_size: 16,
            min_height: 0,
            max_height: 128,
        },
    ]
}

use super::flat_generator::block_ids;

/// Generate all ore positions for a chunk.
/// Returns a map of (local_x, world_y, local_z) -> ore block ID.
/// Only generates positions within the chunk (0..16, 0..16).
pub fn generate_ores(
    chunk_x: i32,
    chunk_z: i32,
    random: &mut Random,
) -> HashMap<(u8, i32, u8), u32> {
    let mut ores: HashMap<(u8, i32, u8), u32> = HashMap::new();
    let ore_types = default_ore_types();

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;

    for ore_type in &ore_types {
        for _ in 0..ore_type.cluster_count {
            let ore_x = random.next_range(0, 15);
            let ore_y = random.next_range(ore_type.min_height, ore_type.max_height);
            let ore_z = random.next_range(0, 15);

            place_ore_cluster(
                random,
                ore_type,
                base_x + ore_x,
                ore_y,
                base_z + ore_z,
                base_x,
                base_z,
                &mut ores,
            );
        }
    }

    ores
}

/// Place a single ore cluster (curved vein with sphere placement).
/// Port of PMMP's Ore::placeObject.
#[allow(clippy::too_many_arguments)]
fn place_ore_cluster(
    random: &mut Random,
    ore_type: &OreType,
    world_x: i32,
    world_y: i32,
    world_z: i32,
    base_x: i32,
    base_z: i32,
    ores: &mut HashMap<(u8, i32, u8), u32>,
) {
    let cluster_size = ore_type.cluster_size;
    let angle = random.next_float() * PI;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let offset_x = cos_a * cluster_size as f64 / 8.0;
    let offset_z = sin_a * cluster_size as f64 / 8.0;

    let x1 = (world_x as f64 + 8.0) + offset_x;
    let x2 = (world_x as f64 + 8.0) - offset_x;
    let z1 = (world_z as f64 + 8.0) + offset_z;
    let z2 = (world_z as f64 + 8.0) - offset_z;
    let y1 = world_y + random.next_bounded_int(3) + 2;
    let y2 = world_y + random.next_bounded_int(3) + 2;

    for count in 0..=cluster_size {
        let t = count as f64 / cluster_size as f64;
        let center_x = x1 + (x2 - x1) * t;
        let center_y = y1 as f64 + (y2 - y1) as f64 * t;
        let center_z = z1 + (z2 - z1) * t;

        let radius = ((count as f64 * PI / cluster_size as f64).sin() + 1.0)
            * random.next_float()
            * cluster_size as f64
            / 16.0
            + 1.0;
        let radius = radius / 2.0;

        // Place sphere at this point
        let start_x = (center_x - radius) as i32;
        let start_y = (center_y - radius) as i32;
        let start_z = (center_z - radius) as i32;
        let end_x = (center_x + radius) as i32;
        let end_y = (center_y + radius) as i32;
        let end_z = (center_z + radius) as i32;

        for xx in start_x..=end_x {
            let size_x = (xx as f64 + 0.5 - center_x) / radius;
            let size_x_sq = size_x * size_x;
            if size_x_sq >= 1.0 {
                continue;
            }

            for yy in start_y..=end_y {
                if yy <= 0 {
                    continue;
                }
                let size_y = (yy as f64 + 0.5 - center_y) / radius;
                let size_y_sq = size_y * size_y;
                if size_x_sq + size_y_sq >= 1.0 {
                    continue;
                }

                for zz in start_z..=end_z {
                    let size_z = (zz as f64 + 0.5 - center_z) / radius;
                    let size_z_sq = size_z * size_z;
                    if size_x_sq + size_y_sq + size_z_sq >= 1.0 {
                        continue;
                    }

                    // Convert to chunk-local coordinates
                    let local_x = xx - base_x;
                    let local_z = zz - base_z;

                    // Only place within this chunk
                    if (0..16).contains(&local_x) && (0..16).contains(&local_z) {
                        ores.insert((local_x as u8, yy, local_z as u8), ore_type.block_id);
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
    fn test_ore_generation() {
        let mut rng = Random::new(42);
        let ores = generate_ores(0, 0, &mut rng);
        // Should generate at least some ores
        assert!(!ores.is_empty(), "Expected ore positions, got none");
        // All positions should be within chunk bounds
        for &(x, y, z) in ores.keys() {
            assert!(x < 16, "Ore x={x} out of bounds");
            assert!(z < 16, "Ore z={z} out of bounds");
            assert!(y > 0, "Ore y={y} at bedrock level");
        }
    }

    #[test]
    fn test_ore_deterministic() {
        let mut rng1 = Random::new(42);
        let ores1 = generate_ores(0, 0, &mut rng1);

        let mut rng2 = Random::new(42);
        let ores2 = generate_ores(0, 0, &mut rng2);

        assert_eq!(ores1.len(), ores2.len());
        for (key, val) in &ores1 {
            assert_eq!(ores2.get(key), Some(val));
        }
    }

    #[test]
    fn test_ore_variety() {
        let mut rng = Random::new(12345);
        let ores = generate_ores(0, 0, &mut rng);
        let unique_types: std::collections::HashSet<u32> = ores.values().copied().collect();
        // Should have multiple ore types
        assert!(
            unique_types.len() >= 3,
            "Expected variety of ores, got {} types",
            unique_types.len()
        );
    }
}
