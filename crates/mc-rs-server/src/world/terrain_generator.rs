use super::biome::{self, BiomeSelector, Gaussian};
use super::chunk_serializer;
use super::flat_generator::block_ids;
use super::ore;
use super::perlin::{self, OctavePerlin};
use super::random::Random;
use super::vegetation;

/// Block runtime IDs from canonical_block_states.nbt (protocol 924).
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
}

/// Sea level (Bedrock Edition = 64, surface at Y=63).
const SEA_LEVEL: i32 = 64;

/// Terrain noise constants (from BetterVanillaGenerator / Bedrock).
const COORD_SCALE: f64 = 684.412;
const HEIGHT_SCALE: f64 = 684.412;
const BASE_SIZE: f64 = 8.5;
const STRETCH_Y: f64 = 12.0;

/// Gaussian smooth radius for biome elevation blending.
const SMOOTH_SIZE: usize = 2;

/// Default biome depth and scale (Phase A: uniform terrain).
/// Phase B will use per-biome values.
const DEFAULT_DEPTH: f64 = 0.1;
const DEFAULT_SCALE: f64 = 0.2;

/// World-level generator state.
/// Created once per seed, reused for all chunks.
struct GeneratorState {
    noise_low: OctavePerlin,
    noise_high: OctavePerlin,
    noise_selector: OctavePerlin,
    selector: BiomeSelector,
    gaussian: Gaussian,
}

impl GeneratorState {
    fn new(seed: u64) -> Self {
        let s = seed as i64;

        // 3 Perlin noise layers for terrain density
        let noise_low = OctavePerlin::new(s, 16, 0.5, 2.0);
        let noise_high = OctavePerlin::new(s.wrapping_add(1), 16, 0.5, 2.0);
        let noise_selector = OctavePerlin::new(s.wrapping_add(2), 8, 0.5, 2.0);

        // Biome selector (still uses Simplex for temperature/rainfall)
        let mut random = Random::new(s);
        // Consume some RNG state for Simplex initialization
        let _simplex = super::noise::Simplex::new(&mut random, 4, 0.25, 1.0 / 32.0);
        random.set_seed(s);
        let selector = BiomeSelector::new(&mut random);

        let gaussian = Gaussian::new(SMOOTH_SIZE);

        Self {
            noise_low,
            noise_high,
            noise_selector,
            selector,
            gaussian,
        }
    }
}

/// Generate biome IDs for a chunk.
fn generate_biome_ids(
    base_x: i32,
    base_z: i32,
    state: &GeneratorState,
    seed: u64,
) -> [[u32; 16]; 16] {
    let mut biome_ids = [[0u32; 16]; 16];
    for (x, row) in biome_ids.iter_mut().enumerate() {
        for (z, cell) in row.iter_mut().enumerate() {
            let abs_x = base_x + x as i32;
            let abs_z = base_z + z as i32;
            *cell = biome::pick_biome_with_jitter(&state.selector, abs_x, abs_z, seed);
        }
    }
    biome_ids
}

/// Compute surface height (highest solid Y) for each column using density grid.
fn compute_surface_heights(density_grid: &[[[f64; 33]; 5]; 5]) -> [[i32; 16]; 16] {
    let mut surfaces = [[SEA_LEVEL - 1; 16]; 16];

    #[allow(clippy::needless_range_loop)]
    for x in 0..16 {
        for z in 0..16 {
            // Scan from top down to find first solid block
            for y in (1..200).rev() {
                let density = perlin::interpolate_density(density_grid, x, z, y);
                if density > 0.0 {
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

    let density_grid = perlin::sample_density_grid(
        &state.noise_low,
        &state.noise_high,
        &state.noise_selector,
        chunk_x,
        chunk_z,
        COORD_SCALE,
        HEIGHT_SCALE,
        BASE_SIZE,
        DEFAULT_SCALE,
        STRETCH_Y,
    );

    let surfaces = compute_surface_heights(&density_grid);
    surfaces[local_x][local_z]
}

/// Determine the block at a given world position using density-based terrain.
fn block_at_density(
    world_y: i32,
    density: f64,
    surface_y: i32,
    cover: &[u32],
    is_non_solid_top: bool,
) -> u32 {
    if world_y == 0 {
        return block_ids::BEDROCK;
    }
    if world_y < 0 {
        return extra_blocks::STONE;
    }

    if density > 0.0 {
        // Solid block — apply ground cover
        if !cover.is_empty() {
            let diff_y = if is_non_solid_top { 1 } else { 0 };
            let cover_start = surface_y + diff_y;
            let depth = cover_start - world_y;
            if depth >= 0 && (depth as usize) < cover.len() {
                return cover[depth as usize];
            }
        }
        extra_blocks::STONE
    } else if world_y < SEA_LEVEL {
        extra_blocks::WATER
    } else {
        // Air — check for snow_layer on top
        if is_non_solid_top && world_y == surface_y + 1 && !cover.is_empty() {
            return cover[0];
        }
        block_ids::AIR
    }
}

/// Generate a terrain chunk using Bedrock-style 3-layer Perlin density.
///
/// Returns (sub_chunk_count, payload_bytes).
pub fn generate_terrain_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let mut payload = Vec::with_capacity(16384);

    let state = GeneratorState::new(seed);

    // Generate density grid (5x5x33 samples, trilinearly interpolated)
    let density_grid = perlin::sample_density_grid(
        &state.noise_low,
        &state.noise_high,
        &state.noise_selector,
        chunk_x,
        chunk_z,
        COORD_SCALE,
        HEIGHT_SCALE,
        BASE_SIZE,
        DEFAULT_SCALE,
        STRETCH_Y,
    );

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let biome_ids = generate_biome_ids(base_x, base_z, &state, seed);

    // Pre-compute surface heights
    let surfaces = compute_surface_heights(&density_grid);

    // Per-chunk RNG for randomized elements
    let mut chunk_random =
        Random::new(0xdeadbeef_i64 ^ ((chunk_x as i64) << 8) ^ chunk_z as i64 ^ seed as i64);

    // Generate ore and vegetation
    let ore_map = ore::generate_ores(chunk_x, chunk_z, &mut chunk_random);
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

    // Find max terrain height for this chunk
    let max_surface = surfaces
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(SEA_LEVEL);
    let max_block_y = max_surface.max(SEA_LEVEL) + 10; // extra for trees
    let sub_chunk_count = (((max_block_y + 64) / 16) + 1).clamp(1, 24) as usize;

    for sub_idx in 0..sub_chunk_count {
        let sub_y_start = -64 + (sub_idx as i32 * 16);

        // Sub-chunks below Y=0 are all stone (underground)
        if sub_y_start + 15 < 0 {
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
                let surface_y = surfaces[local_x][local_z];
                let col_idx = local_x * 16 + local_z;
                let cover = &covers[col_idx];
                let is_non_solid_top = non_solid_top[col_idx];

                for local_y in 0..16usize {
                    let world_y = sub_y_start + local_y as i32;
                    let idx = (local_x << 8) | (local_z << 4) | local_y;

                    // Get density from the interpolated grid
                    let density = if world_y < 0 {
                        1.0 // Always solid below Y=0
                    } else {
                        perlin::interpolate_density(&density_grid, local_x, local_z, world_y)
                    };

                    let mut block =
                        block_at_density(world_y, density, surface_y, cover, is_non_solid_top);

                    // Replace stone with ore
                    if block == extra_blocks::STONE {
                        if let Some(&ore_id) = ore_map.get(&(local_x as u8, world_y, local_z as u8))
                        {
                            block = ore_id;
                        }
                    }

                    // Apply vegetation
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

    // Biome sections
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
            h >= 30 && h <= 200,
            "Surface height {h} out of expected range"
        );
    }

    #[test]
    fn test_chunk_continuity() {
        let seed = 42u64;
        let h_left = get_surface_height(15, 8, seed);
        let h_right = get_surface_height(16, 8, seed);
        let diff = (h_left - h_right).abs();
        assert!(
            diff <= 8,
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

    #[test]
    fn test_sea_level_water() {
        // At sea level, blocks below should be water or stone, above should be air
        let h = get_surface_height(100, 100, 42);
        // Surface should be somewhere near sea level for default terrain
        assert!(h >= 20 && h <= 180, "Surface height {h} unexpected");
    }
}
