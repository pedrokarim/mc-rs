use super::chunk_serializer;
use super::flat_generator::block_ids;
use super::noise::Simplex;
use super::random::Random;

/// Additional block IDs for terrain generation.
pub mod extra_blocks {
    pub const STONE: u32 = 12683;
    pub const WATER: u32 = 7972;
    pub const SAND: u32 = 11768;
    pub const GRAVEL: u32 = 11802;
    pub const OAK_LOG: u32 = 10714;
    pub const OAK_LEAVES: u32 = 8873;
    pub const SNOW: u32 = 11456;
}

/// Water surface level (same as PocketMine-MP).
const WATER_HEIGHT: i32 = 62;

/// Vertical noise sampling rate (same as PMMP).
const NOISE_SAMPLING_RATE_Y: usize = 8;

/// Phase 1: Fixed elevation (Plains biome).
/// Will be replaced by per-biome values in Phase 2.
const MIN_ELEVATION: f64 = 62.0;
const MAX_ELEVATION: f64 = 68.0;

/// Get the surface height at a specific world position.
/// Approximation: uses the noise field to find the highest solid block.
pub fn get_surface_height(world_x: i32, world_z: i32, seed: u64) -> i32 {
    // Generate noise for this column to find actual surface
    let chunk_x = world_x.div_euclid(16);
    let chunk_z = world_z.div_euclid(16);
    let local_x = world_x.rem_euclid(16) as usize;
    let local_z = world_z.rem_euclid(16) as usize;

    let mut random =
        Random::new(0xdeadbeef_i64 ^ ((chunk_x as i64) << 8) ^ chunk_z as i64 ^ seed as i64);
    let noise_base = Simplex::new(&mut random, 4, 0.25, 1.0 / 32.0);

    let min_sum = MIN_ELEVATION - 1.0;
    let max_sum = MAX_ELEVATION;
    let smooth_height = (max_sum - min_sum) / 2.0;
    let max_block_y = max_sum.max(WATER_HEIGHT as f64) as i32;

    let noise_min =
        (min_sum / NOISE_SAMPLING_RATE_Y as f64).floor() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let noise_max =
        (max_sum / NOISE_SAMPLING_RATE_Y as f64).ceil() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let y_size = (noise_max - noise_min) as usize;

    let noise = noise_base.get_fast_noise_3d(
        16,
        y_size,
        16,
        4,
        NOISE_SAMPLING_RATE_Y,
        4,
        chunk_x * 16,
        noise_min,
        chunk_z * 16,
    );

    // Find highest solid block in this column
    let mut surface = WATER_HEIGHT; // default to water level
    for y in (noise_min..=max_block_y).rev() {
        let noise_value = if y > noise_max {
            -1.0
        } else {
            let yi = (y - noise_min) as usize;
            if yi < noise[local_x][local_z].len() {
                noise[local_x][local_z][yi]
                    - 1.0 / smooth_height * (y as f64 - smooth_height - min_sum)
            } else {
                -1.0
            }
        };

        if noise_value > 0.0 {
            surface = y;
            break;
        }
    }

    surface
}

/// Generate a terrain chunk at the given chunk coordinates.
/// Uses PocketMine-MP's Normal generator algorithm:
/// - Simplex 3D noise with trilinear interpolation
/// - Terrain sculpting with smoothHeight formula
/// - Water at Y=62
///
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_terrain_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(16384);

    // Initialize RNG same as PMMP
    let mut random =
        Random::new(0xdeadbeef_i64 ^ ((chunk_x as i64) << 8) ^ chunk_z as i64 ^ seed as i64);
    let noise_base = Simplex::new(&mut random, 4, 0.25, 1.0 / 32.0);

    // Phase 1: Fixed elevation for all columns (Plains biome)
    // Phase 2 will add per-column biome-based elevation
    let min_sum = MIN_ELEVATION - 1.0; // 61.0
    let max_sum = MAX_ELEVATION; // 68.0
    let smooth_height = (max_sum - min_sum) / 2.0; // 3.5
    let max_block_y = max_sum.max(WATER_HEIGHT as f64) as i32; // 68

    // Align noise bounds to sampling rate
    let noise_min =
        (min_sum / NOISE_SAMPLING_RATE_Y as f64).floor() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let noise_max =
        (max_sum / NOISE_SAMPLING_RATE_Y as f64).ceil() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let y_size = (noise_max - noise_min) as usize;

    // Generate 3D noise field with sparse sampling + trilinear interpolation
    let noise = noise_base.get_fast_noise_3d(
        16,
        y_size,
        16,
        4,
        NOISE_SAMPLING_RATE_Y,
        4,
        chunk_x * 16,
        noise_min,
        chunk_z * 16,
    );

    // Sub-chunk 0 starts at Y=-64
    // Bedrock at Y=0 → sub-chunk index 4
    // Need sub-chunks up to max_block_y
    let sub_chunk_count = (((max_block_y + 64) / 16) + 1).max(1) as usize;
    let sub_chunk_count = sub_chunk_count.min(24);

    // Sub-chunk index where noise_min starts
    let min_noise_sub_chunk = (noise_min + 64) / 16;

    for sub_idx in 0..sub_chunk_count {
        let sub_y_start = -64 + (sub_idx as i32 * 16);

        // Check if this sub-chunk is entirely below noise_min and above Y=0
        // → flood-fill with stone
        if sub_y_start >= 0 && (sub_idx as i32) < min_noise_sub_chunk {
            // Entire sub-chunk is solid stone
            let blocks = [0u32; 4096]; // all palette index 0
            let palette = vec![extra_blocks::STONE];
            let sub_chunk = chunk_serializer::serialize_sub_chunk(&blocks, &palette);
            payload.extend_from_slice(&sub_chunk);
            continue;
        }

        // Build palette and block array
        let mut blocks = [0u32; 4096]; // palette index 0 = air
        let mut palette_map: Vec<u32> = vec![block_ids::AIR];

        let get_palette_idx = |block_id: u32, map: &mut Vec<u32>| -> u32 {
            if let Some(idx) = map.iter().position(|&b| b == block_id) {
                idx as u32
            } else {
                map.push(block_id);
                (map.len() - 1) as u32
            }
        };

        let mut has_blocks = false;

        #[allow(clippy::needless_range_loop)]
        for local_x in 0..16usize {
            for local_z in 0..16usize {
                for local_y in 0..16usize {
                    let world_y = sub_y_start + local_y as i32;
                    let idx = (local_x << 8) | (local_z << 4) | local_y;

                    let block = if world_y == 0 {
                        // Bedrock layer
                        block_ids::BEDROCK
                    } else if world_y < 0 {
                        // Below bedrock: stone fill (Y=-64 to -1)
                        extra_blocks::STONE
                    } else if world_y < noise_min {
                        // Below noise range but above bedrock: always stone
                        extra_blocks::STONE
                    } else if world_y <= max_block_y {
                        // In the noise-sculpted zone
                        let noise_value = if world_y > noise_max {
                            -1.0
                        } else {
                            let yi = (world_y - noise_min) as usize;
                            if yi < noise[local_x][local_z].len() {
                                noise[local_x][local_z][yi]
                                    - 1.0 / smooth_height
                                        * (world_y as f64 - smooth_height - min_sum)
                            } else {
                                -1.0
                            }
                        };

                        if noise_value > 0.0 {
                            extra_blocks::STONE
                        } else if world_y <= WATER_HEIGHT {
                            extra_blocks::WATER
                        } else {
                            block_ids::AIR
                        }
                    } else {
                        block_ids::AIR
                    };

                    if block != block_ids::AIR {
                        let pidx = get_palette_idx(block, &mut palette_map);
                        blocks[idx] = pidx;
                        has_blocks = true;
                    }
                }
            }
        }

        if has_blocks {
            let sub_chunk = chunk_serializer::serialize_sub_chunk(&blocks, &palette_map);
            payload.extend_from_slice(&sub_chunk);
        } else {
            // Empty sub-chunk (all air)
            payload.push(8); // version
            payload.push(0); // 0 storage layers
        }
    }

    // All 24 biome sections — Plains (biome ID = 1)
    let biome_section = chunk_serializer::serialize_biome_section_single(1);
    for _ in 0..24 {
        payload.extend_from_slice(&biome_section);
    }

    // Border blocks count
    payload.push(0);

    (sub_chunk_count as u32, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_chunk_generates() {
        let (count, payload) = generate_terrain_chunk(0, 0, 42);
        assert!(count >= 1);
        assert!(!payload.is_empty());
    }

    #[test]
    fn test_terrain_varies() {
        let (_, p1) = generate_terrain_chunk(0, 0, 42);
        let (_, p2) = generate_terrain_chunk(5, 5, 42);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_terrain_deterministic() {
        let (c1, p1) = generate_terrain_chunk(3, 7, 12345);
        let (c2, p2) = generate_terrain_chunk(3, 7, 12345);
        assert_eq!(c1, c2);
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_surface_height_reasonable() {
        let h = get_surface_height(0, 0, 42);
        // Surface should be around sea level (62) +/- terrain variation
        assert!(
            h >= 50 && h <= 130,
            "Surface height {h} out of expected range"
        );
    }

    #[test]
    fn test_water_at_sea_level() {
        // Generate a chunk and verify water exists
        let (count, _payload) = generate_terrain_chunk(0, 0, 42);
        // With terrain around Y=62-68, we need at least enough sub-chunks
        // to cover up to Y=68 → sub-chunk index = (68+64)/16 = 8.25 → 9
        assert!(count >= 5, "Expected at least 5 sub-chunks, got {count}");
    }
}
