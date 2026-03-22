use std::collections::HashMap;

use super::biome::{self, BiomeSelector, Gaussian};
use super::chunk_serializer;
use super::flat_generator::block_ids;
use super::noise::Simplex;
use super::ore;
use super::random::Random;
use super::vegetation;

/// Additional block IDs for terrain generation.
/// Sequential indices from canonical_block_states.nbt (protocol 924).
pub mod extra_blocks {
    pub const STONE: u32 = 2532;
    pub const WATER: u32 = 9268;
    pub const SAND: u32 = 6234;
    pub const SANDSTONE: u32 = 5213;
    pub const GRAVEL: u32 = 15806;
    pub const OAK_LOG: u32 = 1366;
    pub const OAK_LEAVES: u32 = 2752;
    pub const SNOW_LAYER: u32 = 1019;
    pub const COAL_ORE: u32 = 6318;
    pub const IRON_ORE: u32 = 7336;
    pub const GOLD_ORE: u32 = 3203;
    pub const DIAMOND_ORE: u32 = 6501;
    pub const REDSTONE_ORE: u32 = 6356;
    pub const LAPIS_ORE: u32 = 14583;
    pub const SHORT_GRASS: u32 = 12421;
    pub const MYCELIUM: u32 = 5240;
    pub const RED_SAND: u32 = 2732;
    pub const HARDENED_CLAY: u32 = 2086;
    pub const SNOW_BLOCK: u32 = 6233;
    pub const PODZOL: u32 = 7292;
    pub const COARSE_DIRT: u32 = 6725;
    pub const RED_SANDSTONE: u32 = 12454;
    pub const DEEPSLATE: u32 = 1310;
    pub const TUFF: u32 = 1763;
    pub const GRANITE: u32 = 284;
    pub const DIORITE: u32 = 415;
    pub const ANDESITE: u32 = 2530;
    pub const LAVA: u32 = 5406;
}

/// Water surface level (same as PocketMine-MP).
const WATER_HEIGHT: i32 = 62;

/// Vertical noise sampling rate (same as PMMP).
const NOISE_SAMPLING_RATE_Y: usize = 8;

/// Gaussian smooth radius for biome elevation blending.
const SMOOTH_SIZE: usize = 2;

/// World-level generator state.
/// In PMMP, the Normal generator creates the noise + biome selector ONCE
/// in the constructor, then reuses them for all chunks.
/// This ensures noise continuity across chunk boundaries.
struct GeneratorState {
    noise_base: Simplex,
    selector: BiomeSelector,
    gaussian: Gaussian,
}

impl GeneratorState {
    fn new(seed: u64) -> Self {
        // Matches PMMP Normal::__construct:
        // 1. Create noiseBase with initial random state
        let mut random = Random::new(seed as i64);
        let noise_base = Simplex::new(&mut random, 4, 0.25, 1.0 / 32.0);

        // 2. Reset random to world seed, then create biome selector
        random.set_seed(seed as i64);
        let selector = BiomeSelector::new(&mut random);

        let gaussian = Gaussian::new(SMOOTH_SIZE);

        Self {
            noise_base,
            selector,
            gaussian,
        }
    }
}

/// Generate biome data for a chunk with Gaussian-smoothed elevations.
/// Returns (biome_ids[16][16], min_heights[16][16], max_heights[16][16]).
#[allow(clippy::type_complexity)]
fn generate_biomes(
    base_x: i32,
    base_z: i32,
    state: &GeneratorState,
    seed: u64,
) -> ([[u32; 16]; 16], [[f64; 16]; 16], [[f64; 16]; 16]) {
    let padding = SMOOTH_SIZE as i32;
    let start = -padding;
    let end = 16 + padding;

    let mut biome_cache: HashMap<(i32, i32), u32> = HashMap::new();
    let mut all_same = true;
    let mut first_biome = None;

    for x in start..end {
        let abs_x = base_x + x;
        for z in start..end {
            let abs_z = base_z + z;
            let biome_id = biome::pick_biome_with_jitter(&state.selector, abs_x, abs_z, seed);
            biome_cache.insert((x, z), biome_id);

            match first_biome {
                None => first_biome = Some(biome_id),
                Some(fb) if fb != biome_id => all_same = false,
                _ => {}
            }
        }
    }

    let mut biome_ids = [[0u32; 16]; 16];
    for (x, row) in biome_ids.iter_mut().enumerate() {
        for (z, cell) in row.iter_mut().enumerate() {
            *cell = *biome_cache.get(&(x as i32, z as i32)).unwrap();
        }
    }

    let mut min_heights = [[0.0f64; 16]; 16];
    let mut max_heights = [[0.0f64; 16]; 16];

    if all_same {
        let biome_def = biome::get_biome(first_biome.unwrap());
        let min_el = biome_def.min_elevation - 1.0;
        let max_el = biome_def.max_elevation;
        for x in 0..16 {
            for z in 0..16 {
                min_heights[x][z] = min_el;
                max_heights[x][z] = max_el;
            }
        }
    } else {
        let smooth = &state.gaussian.kernel_1d;
        let weight_sum = state.gaussian.weight_sum_1d;
        let ss = state.gaussian.smooth_size as i32;

        let mut min_x: HashMap<(i32, i32), f64> = HashMap::new();
        let mut max_x: HashMap<(i32, i32), f64> = HashMap::new();

        for x in 0..16i32 {
            for z in start..end {
                let mut min_sum = 0.0;
                let mut max_sum = 0.0;
                for sx in -ss..=ss {
                    let weight = smooth[(sx + ss) as usize];
                    let adj_biome = biome_cache[&(x + sx, z)];
                    let adj_def = biome::get_biome(adj_biome);
                    min_sum += (adj_def.min_elevation - 1.0) * weight;
                    max_sum += adj_def.max_elevation * weight;
                }
                min_x.insert((x, z), min_sum / weight_sum);
                max_x.insert((x, z), max_sum / weight_sum);
            }
        }

        for x in 0..16 {
            for z in 0..16 {
                let mut min_sum = 0.0;
                let mut max_sum = 0.0;
                for sx in -ss..=ss {
                    let weight = smooth[(sx + ss) as usize];
                    min_sum += min_x[&(x, z as i32 + sx)] * weight;
                    max_sum += max_x[&(x, z as i32 + sx)] * weight;
                }
                min_heights[x as usize][z] = min_sum / weight_sum;
                max_heights[x as usize][z] = max_sum / weight_sum;
            }
        }
    }

    (biome_ids, min_heights, max_heights)
}

/// Compute the surface height (highest solid Y) for each column using the noise field.
fn compute_surface_heights(
    noise: &[Vec<Vec<f64>>],
    min_heights: &[[f64; 16]; 16],
    max_heights: &[[f64; 16]; 16],
    noise_min: i32,
    noise_max: i32,
) -> [[i32; 16]; 16] {
    let mut surfaces = [[WATER_HEIGHT; 16]; 16];

    for x in 0..16 {
        for z in 0..16 {
            let col_min = min_heights[x][z];
            let col_max = max_heights[x][z];
            let smooth_height = (col_max - col_min) / 2.0;
            let col_max_block = col_max.max(WATER_HEIGHT as f64) as i32;

            for y in (noise_min..=col_max_block).rev() {
                let noise_value = if y > noise_max || smooth_height == 0.0 {
                    -1.0
                } else {
                    let yi = (y - noise_min) as usize;
                    if yi < noise[x][z].len() {
                        noise[x][z][yi] - 1.0 / smooth_height * (y as f64 - smooth_height - col_min)
                    } else {
                        -1.0
                    }
                };

                if noise_value > 0.0 {
                    surfaces[x][z] = y;
                    break;
                }
            }
        }
    }

    surfaces
}

/// Get the surface height at a specific world position.
pub fn get_surface_height(world_x: i32, world_z: i32, seed: u64) -> i32 {
    let chunk_x = world_x.div_euclid(16);
    let chunk_z = world_z.div_euclid(16);
    let local_x = world_x.rem_euclid(16) as usize;
    let local_z = world_z.rem_euclid(16) as usize;

    let state = GeneratorState::new(seed);

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let (_biome_ids, min_heights, max_heights) = generate_biomes(base_x, base_z, &state, seed);

    let mut global_min = f64::MAX;
    let mut global_max = f64::MIN;
    for x in 0..16 {
        for z in 0..16 {
            global_min = global_min.min(min_heights[x][z]);
            global_max = global_max.max(max_heights[x][z]);
        }
    }

    let noise_min =
        (global_min / NOISE_SAMPLING_RATE_Y as f64).floor() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let noise_max =
        (global_max / NOISE_SAMPLING_RATE_Y as f64).ceil() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let y_size = ((noise_max - noise_min) as usize).max(NOISE_SAMPLING_RATE_Y);

    let noise = state.noise_base.get_fast_noise_3d(
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

    let surfaces =
        compute_surface_heights(&noise, &min_heights, &max_heights, noise_min, noise_max);
    surfaces[local_x][local_z]
}

/// Deterministic hash for block variety (bedrock dégradé, stone variants).
/// Returns a value 0..99 for percentage-based decisions.
#[inline]
fn block_variety_hash(x: i32, y: i32, z: i32, seed: u64) -> u32 {
    let h = (x as u64).wrapping_mul(73856093)
        ^ (y as u64).wrapping_mul(19349663)
        ^ (z as u64).wrapping_mul(83492791)
        ^ seed;
    (h.wrapping_mul(h.wrapping_add(223)) >> 16) as u32 % 100
}

/// Pick the underground solid block based on Y level.
/// Implements realistic layer composition from Bedrock Edition analysis:
/// - Y=-64 to -60: bedrock (gradient)
/// - Y=-59 to 0: deepslate + tuff
/// - Y=1 to 8: deepslate/stone transition
/// - Y=9+: stone + granite/diorite/andesite
fn underground_block(world_x: i32, world_y: i32, world_z: i32, seed: u64) -> u32 {
    let h = block_variety_hash(world_x, world_y, world_z, seed);

    if world_y <= -63 {
        // Y=-64, -63: 100% bedrock
        block_ids::BEDROCK
    } else if world_y == -62 {
        // 75% bedrock, 25% deepslate
        if h < 75 {
            block_ids::BEDROCK
        } else {
            extra_blocks::DEEPSLATE
        }
    } else if world_y == -61 {
        // 50% bedrock, 50% deepslate
        if h < 50 {
            block_ids::BEDROCK
        } else {
            extra_blocks::DEEPSLATE
        }
    } else if world_y == -60 {
        // 25% bedrock, 75% deepslate
        if h < 25 {
            block_ids::BEDROCK
        } else {
            extra_blocks::DEEPSLATE
        }
    } else if world_y <= 0 {
        // Deepslate zone: 88% deepslate, 7% tuff, 5% other
        if h < 7 {
            extra_blocks::TUFF
        } else {
            extra_blocks::DEEPSLATE
        }
    } else if world_y <= 8 {
        // Transition zone: linear blend deepslate → stone over 8 levels
        // Y=1: 70% deepslate, Y=4: 40%, Y=8: 0%
        let deepslate_pct = (80 - world_y * 10).max(0) as u32;
        if h < deepslate_pct {
            extra_blocks::DEEPSLATE
        } else {
            // Stone with variants
            stone_with_variants(world_x, world_y, world_z, seed)
        }
    } else {
        // Stone zone with variants
        stone_with_variants(world_x, world_y, world_z, seed)
    }
}

/// Stone with granite/diorite/andesite variants (~7% each).
#[inline]
fn stone_with_variants(x: i32, y: i32, z: i32, seed: u64) -> u32 {
    let h = block_variety_hash(x, y, z, seed.wrapping_add(1));
    if h < 7 {
        extra_blocks::GRANITE
    } else if h < 14 {
        extra_blocks::DIORITE
    } else if h < 21 {
        extra_blocks::ANDESITE
    } else {
        extra_blocks::STONE
    }
}

/// Determine the block at a given world position, including ground cover.
#[allow(clippy::too_many_arguments)]
fn block_at(
    world_x: i32,
    world_y: i32,
    world_z: i32,
    surface_y: i32,
    cover: &[u32],
    noise_value_positive: bool,
    is_non_solid_top: bool,
    seed: u64,
) -> u32 {
    if noise_value_positive || world_y <= 0 {
        // Solid block
        if !cover.is_empty() && noise_value_positive {
            let diff_y = if is_non_solid_top { 1 } else { 0 };
            let cover_start = surface_y + diff_y;
            let depth = cover_start - world_y;
            if depth >= 0 && (depth as usize) < cover.len() {
                return cover[depth as usize];
            }
        }
        // Underground block with realistic layers
        underground_block(world_x, world_y, world_z, seed)
    } else if world_y <= WATER_HEIGHT {
        extra_blocks::WATER
    } else {
        // Air — check for snow_layer on top
        if is_non_solid_top && world_y == surface_y + 1 && !cover.is_empty() {
            return cover[0];
        }
        block_ids::AIR
    }
}

/// Generate a terrain chunk at the given chunk coordinates.
/// Uses PocketMine-MP's Normal generator algorithm with biome system and ground cover.
///
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_terrain_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(16384);

    // Create generator state ONCE per world seed (same noise for all chunks)
    // This is critical: PMMP creates noiseBase in the constructor, not per chunk
    let state = GeneratorState::new(seed);

    // Per-chunk RNG for randomized elements (ore, vegetation)
    let mut chunk_random =
        Random::new(0xdeadbeef_i64 ^ ((chunk_x as i64) << 8) ^ chunk_z as i64 ^ seed as i64);

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let (biome_ids, min_heights, max_heights) = generate_biomes(base_x, base_z, &state, seed);

    let mut global_min = f64::MAX;
    let mut global_max = f64::MIN;
    for x in 0..16 {
        for z in 0..16 {
            global_min = global_min.min(min_heights[x][z]);
            global_max = global_max.max(max_heights[x][z]);
        }
    }

    let noise_min =
        (global_min / NOISE_SAMPLING_RATE_Y as f64).floor() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let noise_max =
        (global_max / NOISE_SAMPLING_RATE_Y as f64).ceil() as i32 * NOISE_SAMPLING_RATE_Y as i32;
    let y_size = ((noise_max - noise_min) as usize).max(NOISE_SAMPLING_RATE_Y);

    // Generate 3D noise field — uses the WORLD-SEEDED noise (continuous across chunks)
    let noise = state.noise_base.get_fast_noise_3d(
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

    // Pre-compute surface heights for ground cover
    let surfaces =
        compute_surface_heights(&noise, &min_heights, &max_heights, noise_min, noise_max);

    // Generate ore positions (uses chunk-seeded RNG)
    let ore_map = ore::generate_ores(chunk_x, chunk_z, &mut chunk_random);

    // Generate vegetation (trees, tall grass)
    let veg_map = vegetation::generate_vegetation(&biome_ids, &surfaces, &mut chunk_random);

    // Pre-compute ground cover per column
    let mut covers: Vec<Vec<u32>> = Vec::with_capacity(256);
    let mut non_solid_top = [false; 256];
    for x in 0..16 {
        for z in 0..16 {
            let biome_def = biome::get_biome(biome_ids[x][z]);
            let is_non_solid = !biome_def.ground_cover.is_empty()
                && biome_def.ground_cover[0] == extra_blocks::SNOW_LAYER;
            non_solid_top[x * 16 + z] = is_non_solid;
            covers.push(biome_def.ground_cover);
        }
    }

    let max_block_y = global_max.max(WATER_HEIGHT as f64) as i32 + 1;
    let sub_chunk_count = (((max_block_y + 64) / 16) + 1).clamp(1, 24) as usize;
    let min_noise_sub_chunk = ((noise_min + 64) as f64 / 16.0).floor() as i32;

    for sub_idx in 0..sub_chunk_count {
        let sub_y_start = -64 + (sub_idx as i32 * 16);

        // Flood-fill with stone for sub-chunks above Y=0 but below noise range
        if sub_y_start >= 0 && (sub_idx as i32) < min_noise_sub_chunk {
            let blocks = [0u32; 4096];
            let palette = vec![extra_blocks::STONE];
            let sub_chunk = chunk_serializer::serialize_sub_chunk(&blocks, &palette);
            payload.extend_from_slice(&sub_chunk);
            continue;
        }

        let mut blocks = [0u32; 4096];
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
                let col_min = min_heights[local_x][local_z];
                let col_max = max_heights[local_x][local_z];
                let smooth_height = (col_max - col_min) / 2.0;
                let col_max_block = col_max.max(WATER_HEIGHT as f64) as i32 + 1;
                let surface_y = surfaces[local_x][local_z];
                let col_idx = local_x * 16 + local_z;
                let cover = &covers[col_idx];
                let is_non_solid_top = non_solid_top[col_idx];

                for local_y in 0..16usize {
                    let world_y = sub_y_start + local_y as i32;
                    let idx = (local_x << 8) | (local_z << 4) | local_y;

                    let world_x = base_x + local_x as i32;
                    let world_z = base_z + local_z as i32;

                    let mut block = if world_y < 0 || world_y < noise_min {
                        // Underground: use realistic layers
                        underground_block(world_x, world_y, world_z, seed)
                    } else if world_y <= col_max_block {
                        // Noise-sculpted zone
                        let noise_positive = if world_y > noise_max || smooth_height == 0.0 {
                            false
                        } else {
                            let yi = (world_y - noise_min) as usize;
                            if yi < noise[local_x][local_z].len() {
                                let nv = noise[local_x][local_z][yi]
                                    - 1.0 / smooth_height
                                        * (world_y as f64 - smooth_height - col_min);
                                nv > 0.0
                            } else {
                                false
                            }
                        };

                        block_at(
                            world_x,
                            world_y,
                            world_z,
                            surface_y,
                            cover,
                            noise_positive,
                            is_non_solid_top,
                            seed,
                        )
                    } else {
                        block_ids::AIR
                    };

                    // Replace stone/deepslate with ore if applicable
                    if block == extra_blocks::STONE || block == extra_blocks::DEEPSLATE {
                        if let Some(&ore_id) = ore_map.get(&(local_x as u8, world_y, local_z as u8))
                        {
                            block = ore_id;
                        }
                    }

                    // Apply vegetation (trees) — overrides air/grass blocks
                    if let Some(&veg_id) = veg_map.get(&(local_x as u8, world_y, local_z as u8)) {
                        if veg_id != 0 {
                            block = veg_id;
                        }
                    }

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
            payload.push(8); // version
            payload.push(0); // 0 storage layers
        }
    }

    // Biome sections — use center biome
    let center_biome = biome_ids[8][8];
    let biome_section = chunk_serializer::serialize_biome_section_single(center_biome);
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
        assert!(
            h >= 40 && h <= 140,
            "Surface height {h} out of expected range"
        );
    }

    #[test]
    fn test_chunk_continuity() {
        // Test that adjacent chunks have continuous terrain at boundaries
        let seed = 42u64;

        // Get surface heights at the boundary between chunk (0,0) and (1,0)
        // Last column of chunk (0,0) = world_x=15
        // First column of chunk (1,0) = world_x=16
        let h_left = get_surface_height(15, 8, seed);
        let h_right = get_surface_height(16, 8, seed);

        // Adjacent columns should be within a few blocks of each other
        let diff = (h_left - h_right).abs();
        assert!(
            diff <= 5,
            "Chunk boundary discontinuity: left={h_left}, right={h_right}, diff={diff}"
        );
    }

    #[test]
    fn test_different_biomes_exist() {
        let state = GeneratorState::new(42);

        let mut biomes = std::collections::HashSet::new();
        for x in -10..10 {
            for z in -10..10 {
                biomes.insert(biome::pick_biome_with_jitter(
                    &state.selector,
                    x * 64,
                    z * 64,
                    42,
                ));
            }
        }
        assert!(
            biomes.len() >= 3,
            "Expected biome variety, got {} biomes: {:?}",
            biomes.len(),
            biomes
        );
    }
}
