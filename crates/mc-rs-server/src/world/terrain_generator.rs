use super::chunk_serializer;
use super::flat_generator::block_ids;

/// Additional block IDs for terrain generation.
/// Runtime IDs from canonical_block_states.nbt (sequential index).
pub mod extra_blocks {
    pub const STONE: u32 = 12683;
    pub const WATER: u32 = 7972;
    pub const SAND: u32 = 11768;
    pub const GRAVEL: u32 = 11802;
    pub const OAK_LOG: u32 = 10714;
    pub const OAK_LEAVES: u32 = 8873;
    pub const SNOW: u32 = 11456;
}

/// Simple Perlin-like noise for terrain height.
/// Uses a basic hash function for deterministic pseudo-random values.
fn noise2d(x: i32, z: i32, seed: u64) -> f64 {
    let n = x as i64 * 374761393 + z as i64 * 668265263 + seed as i64;
    let n = (n ^ (n >> 13)) * 1274126177;
    let n = n ^ (n >> 16);
    (n as f64 / i64::MAX as f64).abs()
}

/// Smoothed noise — averages with neighbors for smoother terrain.
fn smooth_noise(x: i32, z: i32, seed: u64) -> f64 {
    let corners = (noise2d(x - 1, z - 1, seed) + noise2d(x + 1, z - 1, seed)
        + noise2d(x - 1, z + 1, seed) + noise2d(x + 1, z + 1, seed)) / 16.0;
    let sides = (noise2d(x - 1, z, seed) + noise2d(x + 1, z, seed)
        + noise2d(x, z - 1, seed) + noise2d(x, z + 1, seed)) / 8.0;
    let center = noise2d(x, z, seed) / 4.0;
    corners + sides + center
}

/// Interpolated noise at fractional coordinates.
fn interpolated_noise(x: f64, z: f64, seed: u64) -> f64 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - x.floor();
    let fz = z - z.floor();

    let v1 = smooth_noise(ix, iz, seed);
    let v2 = smooth_noise(ix + 1, iz, seed);
    let v3 = smooth_noise(ix, iz + 1, seed);
    let v4 = smooth_noise(ix + 1, iz + 1, seed);

    let i1 = v1 * (1.0 - fx) + v2 * fx;
    let i2 = v3 * (1.0 - fx) + v4 * fx;

    i1 * (1.0 - fz) + i2 * fz
}

/// Multi-octave noise for realistic terrain.
fn terrain_noise(world_x: i32, world_z: i32, seed: u64) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 0.01; // Low frequency = large features
    let octaves = 4;

    for i in 0..octaves {
        let octave_seed = seed.wrapping_add(i * 1000);
        total += interpolated_noise(
            world_x as f64 * frequency,
            world_z as f64 * frequency,
            octave_seed,
        ) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    total
}

/// Generate a terrain chunk at the given chunk coordinates.
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_terrain_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(8192);

    // Calculate heightmap for this chunk (16x16 columns)
    let mut heightmap = [[0i32; 16]; 16];
    for local_x in 0..16 {
        for local_z in 0..16 {
            let world_x = chunk_x * 16 + local_x as i32;
            let world_z = chunk_z * 16 + local_z as i32;

            // Base height around sea level (Y=-4 in world coords = sub-chunk relative)
            // World Y range: -64 to 320
            // Sea level: Y=62 (vanilla) but for our purposes, we use lower terrain
            // Height in range -60 to -40 (relative to bedrock at -64)
            let noise_val = terrain_noise(world_x, world_z, seed);
            let height = -60 + (noise_val * 12.0) as i32; // hills between -60 and -48
            heightmap[local_x][local_z] = height;
        }
    }

    // Determine how many sub-chunks we need
    let max_height = heightmap.iter().flatten().copied().max().unwrap_or(-60);
    // Sub-chunk index = (world_y + 64) / 16
    // Sub-chunk 0 = Y[-64, -49], sub-chunk 1 = Y[-48, -33], etc.
    let max_sub_chunk = ((max_height + 64) / 16 + 1).max(1) as usize;
    let sub_chunk_count = max_sub_chunk.min(24);

    // Generate each sub-chunk
    for sub_idx in 0..sub_chunk_count {
        let sub_y_start = -64 + (sub_idx as i32 * 16); // world Y of bottom of this sub-chunk

        // Build palette and block array for this sub-chunk
        let mut blocks = [0u32; 4096]; // palette index 0 = air
        let mut palette_map: Vec<u32> = vec![block_ids::AIR]; // index 0 = air

        let mut get_palette_idx = |block_id: u32, map: &mut Vec<u32>| -> u32 {
            if let Some(idx) = map.iter().position(|&b| b == block_id) {
                idx as u32
            } else {
                map.push(block_id);
                (map.len() - 1) as u32
            }
        };

        let mut has_blocks = false;

        for local_x in 0..16usize {
            for local_z in 0..16usize {
                let surface_y = heightmap[local_x][local_z];

                for local_y in 0..16usize {
                    let world_y = sub_y_start + local_y as i32;
                    let idx = (local_x << 8) | (local_z << 4) | local_y;

                    let block = if world_y == -64 {
                        // Bedrock layer
                        block_ids::BEDROCK
                    } else if world_y < surface_y - 3 {
                        // Deep underground = stone
                        extra_blocks::STONE
                    } else if world_y < surface_y {
                        // Near surface = dirt
                        block_ids::DIRT
                    } else if world_y == surface_y {
                        // Surface = grass
                        block_ids::GRASS_BLOCK
                    } else {
                        // Air
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

    // All 24 biome sections
    let biome_section = chunk_serializer::serialize_biome_section_single(1); // Plains
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
        // Different chunks should produce different data
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_terrain_deterministic() {
        let (c1, p1) = generate_terrain_chunk(3, 7, 12345);
        let (c2, p2) = generate_terrain_chunk(3, 7, 12345);
        assert_eq!(c1, c2);
        assert_eq!(p1, p2);
    }
}
