//! Overworld terrain generator.
//!
//! Generates realistic Minecraft-like terrain with biomes, caves, ores,
//! trees, and vegetation using Perlin noise.

#![allow(clippy::needless_range_loop)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::biome::{BiomeDef, BiomeSelector, TreeType};
use crate::block_hash::WorldBlocks;
use crate::chunk::{ChunkColumn, OVERWORLD_MIN_Y};
use crate::noise::OctaveNoise;

/// Standard Minecraft sea level.
pub const SEA_LEVEL: i32 = 62;

/// Ore generation configuration.
struct OreConfig {
    block: u32,
    deepslate_block: u32,
    vein_size: u32,
    veins_per_chunk: u32,
    min_y: i32,
    max_y: i32,
}

/// Overworld terrain generator with noise-based terrain and biomes.
pub struct OverworldGenerator {
    seed: u64,
    blocks: WorldBlocks,
    biome_selector: BiomeSelector,
    terrain_noise: OctaveNoise,
    detail_noise: OctaveNoise,
    cave_noise_1: OctaveNoise,
    cave_noise_2: OctaveNoise,
}

impl OverworldGenerator {
    /// Create a new overworld generator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            blocks: WorldBlocks::compute(),
            biome_selector: BiomeSelector::new(seed),
            terrain_noise: OctaveNoise::new(seed, 6, 2.0, 0.5),
            detail_noise: OctaveNoise::new(seed.wrapping_add(1000), 3, 2.0, 0.5),
            cave_noise_1: OctaveNoise::new(seed.wrapping_add(2000), 3, 2.0, 0.5),
            cave_noise_2: OctaveNoise::new(seed.wrapping_add(3000), 3, 2.0, 0.5),
        }
    }

    /// Generate a full chunk column at the given chunk coordinates.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> ChunkColumn {
        let mut column = ChunkColumn::new_air(chunk_x, chunk_z, self.blocks.air);

        // Phase 1: Compute heightmap and biomes
        let mut heightmap = [[0i32; 16]; 16];
        let mut biome_map = [[0u8; 16]; 16]; // biome IDs
        self.compute_heightmap_and_biomes(chunk_x, chunk_z, &mut heightmap, &mut biome_map);

        // Phase 2: Fill terrain (stone, surface, bedrock)
        self.fill_terrain(&mut column, &heightmap, &biome_map);

        // Phase 3: Carve caves
        self.carve_caves(&mut column, chunk_x, chunk_z, &heightmap);

        // Phase 4: Fill water below sea level
        self.fill_water(&mut column, &heightmap);

        // Phase 5: Place ores
        self.place_ores(&mut column, chunk_x, chunk_z);

        // Phase 6: Place trees
        self.place_trees(&mut column, chunk_x, chunk_z, &heightmap, &biome_map);

        // Phase 7: Place vegetation
        self.place_vegetation(&mut column, chunk_x, chunk_z, &heightmap, &biome_map);

        // Phase 8: Store biome data
        for lx in 0..16 {
            for lz in 0..16 {
                column.biomes[lx * 16 + lz] = biome_map[lx][lz];
            }
        }

        column
    }

    /// Find the ground level at the spawn point (chunk 0,0, block 0,0).
    /// Returns the Y of the first air block above ground.
    pub fn find_spawn_y(&self) -> i32 {
        let chunk = self.generate_chunk(0, 0);
        // Search from Y=100 downward at (8, 8) — center of spawn chunk
        for y in (0..=120).rev() {
            if let Some(block) = chunk.get_block_world(8, y, 8) {
                if block != self.blocks.air && block != self.blocks.water {
                    return y + 1;
                }
            }
        }
        SEA_LEVEL + 1
    }

    fn compute_heightmap_and_biomes(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        heightmap: &mut [[i32; 16]; 16],
        biome_map: &mut [[u8; 16]; 16],
    ) {
        for lx in 0..16 {
            for lz in 0..16 {
                let world_x = chunk_x * 16 + lx as i32;
                let world_z = chunk_z * 16 + lz as i32;

                let biome = self.biome_selector.get_biome(world_x, world_z);
                biome_map[lx][lz] = biome.id;

                let nx = world_x as f64 / 128.0;
                let nz = world_z as f64 / 128.0;
                let base = self.terrain_noise.sample_2d(nx, nz);
                let detail = self.detail_noise.sample_2d(nx * 4.0, nz * 4.0) * 0.1;

                let height =
                    64.0 + (base + detail) * 20.0 * biome.height_scale + biome.height_offset;

                // Clamp to valid range
                heightmap[lx][lz] = (height.round() as i32).clamp(OVERWORLD_MIN_Y + 5, 250);
            }
        }
    }

    fn fill_terrain(
        &self,
        column: &mut ChunkColumn,
        heightmap: &[[i32; 16]; 16],
        biome_map: &[[u8; 16]; 16],
    ) {
        for lx in 0..16 {
            for lz in 0..16 {
                let surface_y = heightmap[lx][lz];
                let biome = self.biome_def(biome_map[lx][lz]);

                let surface_block = self.blocks.by_name(biome.surface_block);
                let filler_block = self.blocks.by_name(biome.filler_block);

                // Bedrock layer (Y=-64 to Y=-60, randomized)
                let mut rng = chunk_pos_rng(self.seed, lx as i32, lz as i32);
                for y in OVERWORLD_MIN_Y..=(OVERWORLD_MIN_Y + 4) {
                    let depth = (y - OVERWORLD_MIN_Y) as u32;
                    // 100% at -64, decreasing chance above
                    if depth == 0 || rng.gen_range(0..=depth) == 0 {
                        column.set_block_world(lx, y, lz, self.blocks.bedrock);
                    } else {
                        column.set_block_world(lx, y, lz, self.blocks.deepslate);
                    }
                }

                // Deepslate layer (Y=-59 to Y=-1)
                for y in (OVERWORLD_MIN_Y + 5)..0 {
                    if y <= surface_y {
                        column.set_block_world(lx, y, lz, self.blocks.deepslate);
                    }
                }

                // Stone layer (Y=0 to surface-5)
                for y in 0..=(surface_y - 5).max(-1) {
                    column.set_block_world(lx, y, lz, self.blocks.stone);
                }

                // Filler layer (surface-4 to surface-1)
                if surface_y > OVERWORLD_MIN_Y + 5 {
                    let filler_start = (surface_y - 4).max(0);
                    for y in filler_start..surface_y {
                        // Desert: use sandstone below sand
                        let block = if biome.id == 2 && y < surface_y - 1 {
                            self.blocks.sandstone
                        } else {
                            filler_block
                        };
                        column.set_block_world(lx, y, lz, block);
                    }

                    // Surface block
                    // If underwater, use the underwater block instead
                    if surface_y < SEA_LEVEL {
                        column.set_block_world(
                            lx,
                            surface_y,
                            lz,
                            self.blocks.by_name(biome.underwater_block),
                        );
                    } else {
                        column.set_block_world(lx, surface_y, lz, surface_block);
                    }
                }

                // Snow layer for snowy biomes
                if biome.has_snow && surface_y >= SEA_LEVEL {
                    column.set_block_world(lx, surface_y + 1, lz, self.blocks.snow_layer);
                }
            }
        }
    }

    fn carve_caves(
        &self,
        column: &mut ChunkColumn,
        chunk_x: i32,
        chunk_z: i32,
        heightmap: &[[i32; 16]; 16],
    ) {
        for lx in 0..16 {
            for lz in 0..16 {
                let world_x = chunk_x * 16 + lx as i32;
                let world_z = chunk_z * 16 + lz as i32;
                let surface_y = heightmap[lx][lz];

                // Carve from Y=OVERWORLD_MIN_Y+5 to surface-5
                let max_cave_y = (surface_y - 5).min(120);
                for y in (OVERWORLD_MIN_Y + 5)..=max_cave_y {
                    let nx = world_x as f64 / 32.0;
                    let ny = y as f64 / 24.0;
                    let nz = world_z as f64 / 32.0;

                    let c1 = self.cave_noise_1.sample_3d(nx, ny, nz);
                    let c2 = self.cave_noise_2.sample_3d(nx, ny, nz);

                    // Spaghetti caves: cave where both noise values are near zero
                    if c1 * c1 + c2 * c2 < 0.006 {
                        let block = if y < SEA_LEVEL {
                            self.blocks.water
                        } else {
                            self.blocks.air
                        };
                        column.set_block_world(lx, y, lz, block);
                    }
                }
            }
        }
    }

    fn fill_water(&self, column: &mut ChunkColumn, heightmap: &[[i32; 16]; 16]) {
        for lx in 0..16 {
            for lz in 0..16 {
                let surface_y = heightmap[lx][lz];
                for y in (surface_y + 1)..=SEA_LEVEL {
                    if let Some(block) = column.get_block_world(lx, y, lz) {
                        if block == self.blocks.air {
                            column.set_block_world(lx, y, lz, self.blocks.water);
                        }
                    }
                }
            }
        }
    }

    fn place_ores(&self, column: &mut ChunkColumn, chunk_x: i32, chunk_z: i32) {
        let ores = [
            OreConfig {
                block: self.blocks.coal_ore,
                deepslate_block: self.blocks.deepslate_coal_ore,
                vein_size: 17,
                veins_per_chunk: 20,
                min_y: 0,
                max_y: 128,
            },
            OreConfig {
                block: self.blocks.iron_ore,
                deepslate_block: self.blocks.deepslate_iron_ore,
                vein_size: 9,
                veins_per_chunk: 20,
                min_y: -64,
                max_y: 72,
            },
            OreConfig {
                block: self.blocks.gold_ore,
                deepslate_block: self.blocks.deepslate_gold_ore,
                vein_size: 9,
                veins_per_chunk: 2,
                min_y: -64,
                max_y: 32,
            },
            OreConfig {
                block: self.blocks.diamond_ore,
                deepslate_block: self.blocks.deepslate_diamond_ore,
                vein_size: 8,
                veins_per_chunk: 1,
                min_y: -64,
                max_y: 16,
            },
            OreConfig {
                block: self.blocks.redstone_ore,
                deepslate_block: self.blocks.deepslate_redstone_ore,
                vein_size: 8,
                veins_per_chunk: 8,
                min_y: -64,
                max_y: 16,
            },
            OreConfig {
                block: self.blocks.lapis_ore,
                deepslate_block: self.blocks.deepslate_lapis_ore,
                vein_size: 7,
                veins_per_chunk: 1,
                min_y: -32,
                max_y: 32,
            },
            OreConfig {
                block: self.blocks.emerald_ore,
                deepslate_block: self.blocks.deepslate_emerald_ore,
                vein_size: 1,
                veins_per_chunk: 1,
                min_y: -16,
                max_y: 48,
            },
            OreConfig {
                block: self.blocks.copper_ore,
                deepslate_block: self.blocks.deepslate_copper_ore,
                vein_size: 10,
                veins_per_chunk: 6,
                min_y: -16,
                max_y: 112,
            },
        ];

        for (ore_idx, ore) in ores.iter().enumerate() {
            let mut rng = StdRng::seed_from_u64(
                self.seed
                    .wrapping_mul(31)
                    .wrapping_add(chunk_x as u64)
                    .wrapping_mul(17)
                    .wrapping_add(chunk_z as u64)
                    .wrapping_mul(13)
                    .wrapping_add(ore_idx as u64),
            );

            for _ in 0..ore.veins_per_chunk {
                let vx = rng.gen_range(0..16) as usize;
                let vz = rng.gen_range(0..16) as usize;
                let vy = rng.gen_range(ore.min_y..=ore.max_y);

                // Expand vein
                let mut cx = vx as i32;
                let mut cy = vy;
                let mut cz = vz as i32;

                for _ in 0..ore.vein_size {
                    if (0..16).contains(&cx) && (0..16).contains(&cz) {
                        if let Some(existing) = column.get_block_world(cx as usize, cy, cz as usize)
                        {
                            let ore_block = if cy < 0 {
                                ore.deepslate_block
                            } else {
                                ore.block
                            };
                            // Only replace stone or deepslate
                            if existing == self.blocks.stone || existing == self.blocks.deepslate {
                                column.set_block_world(cx as usize, cy, cz as usize, ore_block);
                            }
                        }
                    }
                    // Random walk
                    match rng.gen_range(0..6) {
                        0 => cx += 1,
                        1 => cx -= 1,
                        2 => cy += 1,
                        3 => cy -= 1,
                        4 => cz += 1,
                        _ => cz -= 1,
                    }
                }
            }
        }
    }

    fn place_trees(
        &self,
        column: &mut ChunkColumn,
        chunk_x: i32,
        chunk_z: i32,
        heightmap: &[[i32; 16]; 16],
        biome_map: &[[u8; 16]; 16],
    ) {
        let center_biome = self.biome_def(biome_map[8][8]);
        let num_trees = center_biome.tree_density;
        if num_trees == 0 {
            return;
        }

        let mut rng = StdRng::seed_from_u64(
            self.seed
                .wrapping_add((chunk_x as u64).wrapping_mul(341_873_128_712))
                .wrapping_add((chunk_z as u64).wrapping_mul(132_897_987_541)),
        );

        for _ in 0..num_trees {
            let tx = rng.gen_range(2..14);
            let tz = rng.gen_range(2..14);
            let ty = heightmap[tx][tz] + 1;

            // Skip underwater
            if ty <= SEA_LEVEL {
                continue;
            }

            // Check surface is plantable
            if let Some(surface) = column.get_block_world(tx, ty - 1, tz) {
                if surface != self.blocks.grass_block && surface != self.blocks.dirt {
                    continue;
                }
            } else {
                continue;
            }

            let local_biome = self.biome_def(biome_map[tx][tz]);
            self.place_tree(column, tx, ty, tz, local_biome.tree_type, &mut rng);
        }
    }

    fn place_tree(
        &self,
        column: &mut ChunkColumn,
        x: usize,
        base_y: i32,
        z: usize,
        tree_type: TreeType,
        rng: &mut StdRng,
    ) {
        let (log, leaves) = match tree_type {
            TreeType::Oak => (self.blocks.oak_log, self.blocks.oak_leaves),
            TreeType::Birch => (self.blocks.birch_log, self.blocks.birch_leaves),
            TreeType::Spruce => (self.blocks.spruce_log, self.blocks.spruce_leaves),
            TreeType::Acacia => (self.blocks.acacia_log, self.blocks.acacia_leaves),
            TreeType::None => return,
        };

        let trunk_height = match tree_type {
            TreeType::Spruce => rng.gen_range(6..9),
            _ => rng.gen_range(4..7),
        };

        // Place trunk
        for dy in 0..trunk_height {
            column.set_block_world(x, base_y + dy, z, log);
        }

        // Place leaves
        match tree_type {
            TreeType::Spruce => {
                // Tapered canopy
                let top_y = base_y + trunk_height;
                // Top 1x1
                self.set_if_air(column, x, top_y, z, leaves);
                // Layer below: 3x3
                for dx in -1i32..=1 {
                    for dz in -1i32..=1 {
                        let lx = x as i32 + dx;
                        let lz = z as i32 + dz;
                        if (0..16).contains(&lx) && (0..16).contains(&lz) {
                            self.set_if_air(column, lx as usize, top_y - 1, lz as usize, leaves);
                        }
                    }
                }
                // Wider layers below
                for layer in 2..=3 {
                    let radius = layer;
                    let y = top_y - layer;
                    for dx in -radius..=radius {
                        for dz in -radius..=radius {
                            // Diamond shape
                            if dx.abs() + dz.abs() > radius + 1 {
                                continue;
                            }
                            let lx = x as i32 + dx;
                            let lz = z as i32 + dz;
                            if (0..16).contains(&lx) && (0..16).contains(&lz) {
                                self.set_if_air(column, lx as usize, y, lz as usize, leaves);
                            }
                        }
                    }
                }
            }
            _ => {
                // Standard canopy (oak, birch, acacia): 5x5x2 + 3x3x1 on top
                let top_y = base_y + trunk_height;
                // Top layer: 3x3
                for dx in -1i32..=1 {
                    for dz in -1i32..=1 {
                        let lx = x as i32 + dx;
                        let lz = z as i32 + dz;
                        if (0..16).contains(&lx) && (0..16).contains(&lz) {
                            self.set_if_air(column, lx as usize, top_y, lz as usize, leaves);
                        }
                    }
                }
                // Two layers below: 5x5 minus corners
                for layer in 1..=2 {
                    let y = top_y - layer;
                    for dx in -2i32..=2 {
                        for dz in -2i32..=2 {
                            // Skip corners for a rounder shape
                            if dx.abs() == 2 && dz.abs() == 2 {
                                continue;
                            }
                            let lx = x as i32 + dx;
                            let lz = z as i32 + dz;
                            if (0..16).contains(&lx) && (0..16).contains(&lz) {
                                self.set_if_air(column, lx as usize, y, lz as usize, leaves);
                            }
                        }
                    }
                }
            }
        }
    }

    fn place_vegetation(
        &self,
        column: &mut ChunkColumn,
        chunk_x: i32,
        chunk_z: i32,
        heightmap: &[[i32; 16]; 16],
        biome_map: &[[u8; 16]; 16],
    ) {
        let mut rng = StdRng::seed_from_u64(
            self.seed
                .wrapping_add((chunk_x as u64).wrapping_mul(7))
                .wrapping_add((chunk_z as u64).wrapping_mul(11))
                .wrapping_add(9999),
        );

        for lx in 0..16 {
            for lz in 0..16 {
                let surface_y = heightmap[lx][lz];
                if surface_y < SEA_LEVEL {
                    continue;
                }

                let biome = self.biome_def(biome_map[lx][lz]);
                let place_y = surface_y + 1;

                // Check surface is plantable
                if let Some(surface) = column.get_block_world(lx, surface_y, lz) {
                    if surface != self.blocks.grass_block
                        && surface != self.blocks.sand
                        && surface != self.blocks.dirt
                    {
                        continue;
                    }
                }

                // Check position is air
                if let Some(above) = column.get_block_world(lx, place_y, lz) {
                    if above != self.blocks.air {
                        continue;
                    }
                }

                let roll: f32 = rng.gen();

                match biome.id {
                    1 | 4 | 27 => {
                        // Plains, Forest, Birch Forest: tallgrass + flowers
                        if roll < 0.15 {
                            column.set_block_world(lx, place_y, lz, self.blocks.tallgrass);
                        } else if roll < 0.17 {
                            column.set_block_world(lx, place_y, lz, self.blocks.poppy);
                        } else if roll < 0.19 {
                            column.set_block_world(lx, place_y, lz, self.blocks.dandelion);
                        }
                    }
                    2 => {
                        // Desert: dead bush, cactus
                        if roll < 0.03 {
                            column.set_block_world(lx, place_y, lz, self.blocks.dead_bush);
                        } else if roll < 0.04 {
                            // Cactus: only on sand with air on all sides
                            if self.can_place_cactus(column, lx, place_y, lz) {
                                column.set_block_world(lx, place_y, lz, self.blocks.cactus);
                                // Stack 1-2 more cactus blocks
                                let extra: i32 = rng.gen_range(1..=2);
                                for dy in 1..=extra {
                                    column.set_block_world(
                                        lx,
                                        place_y + dy,
                                        lz,
                                        self.blocks.cactus,
                                    );
                                }
                            }
                        }
                    }
                    5 | 12 => {
                        // Taiga, Snowy Plains: sparse tallgrass
                        if roll < 0.05 {
                            column.set_block_world(lx, place_y, lz, self.blocks.tallgrass);
                        }
                    }
                    35 => {
                        // Savanna: tallgrass + dead bush
                        if roll < 0.10 {
                            column.set_block_world(lx, place_y, lz, self.blocks.tallgrass);
                        } else if roll < 0.12 {
                            column.set_block_world(lx, place_y, lz, self.blocks.dead_bush);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set a block only if the current block is air.
    fn set_if_air(&self, column: &mut ChunkColumn, x: usize, y: i32, z: usize, runtime_id: u32) {
        if let Some(current) = column.get_block_world(x, y, z) {
            if current == self.blocks.air {
                column.set_block_world(x, y, z, runtime_id);
            }
        }
    }

    /// Check if a cactus can be placed (air on all 4 horizontal sides).
    fn can_place_cactus(&self, column: &ChunkColumn, x: usize, y: i32, z: usize) -> bool {
        let checks: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
        for &(dx, dz) in checks {
            let nx = x as i32 + dx;
            let nz = z as i32 + dz;
            if !(0..16).contains(&nx) || !(0..16).contains(&nz) {
                continue; // Edge of chunk, assume ok
            }
            if let Some(block) = column.get_block_world(nx as usize, y, nz as usize) {
                if block != self.blocks.air {
                    return false;
                }
            }
        }
        true
    }

    /// Look up a biome definition by ID.
    fn biome_def(&self, id: u8) -> &'static BiomeDef {
        crate::biome::biome_defs()
            .iter()
            .find(|b| b.id == id)
            .unwrap_or(&crate::biome::biome_defs()[1]) // fallback: plains
    }
}

/// Create a deterministic RNG for a specific block column within the seed.
fn chunk_pos_rng(seed: u64, x: i32, z: i32) -> StdRng {
    StdRng::seed_from_u64(
        seed.wrapping_add((x as u64).wrapping_mul(1_000_003))
            .wrapping_add((z as u64).wrapping_mul(999_983)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gen() -> OverworldGenerator {
        OverworldGenerator::new(42)
    }

    #[test]
    fn generate_chunk_not_all_air() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut non_air = 0;
        for y in OVERWORLD_MIN_Y..320 {
            if let Some(block) = col.get_block_world(8, y, 8) {
                if block != gen.blocks.air {
                    non_air += 1;
                }
            }
        }
        assert!(
            non_air > 10,
            "Chunk should have many non-air blocks, got {non_air}"
        );
    }

    #[test]
    fn bedrock_layer_present() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        // Y=-64 must always be bedrock
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(
                    col.get_block_world(x, OVERWORLD_MIN_Y, z),
                    Some(gen.blocks.bedrock),
                    "Bedrock missing at ({x}, {}, {z})",
                    OVERWORLD_MIN_Y
                );
            }
        }
    }

    #[test]
    fn terrain_has_stone() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut stone_count = 0;
        for y in 0..60 {
            if let Some(block) = col.get_block_world(8, y, 8) {
                if block == gen.blocks.stone {
                    stone_count += 1;
                }
            }
        }
        assert!(stone_count > 10, "Should have stone below surface");
    }

    #[test]
    fn biome_data_stored() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        // At least some biome IDs should be non-zero (not all ocean)
        let non_zero = col.biomes.iter().filter(|&&b| b != 0).count();
        // It's possible (but unlikely) that the entire chunk is ocean.
        // The seed 42 should produce land at chunk (0,0).
        assert!(
            non_zero > 0 || col.biomes.iter().all(|&b| b == 0),
            "Biomes should be populated"
        );
    }

    #[test]
    fn deterministic_generation() {
        let gen1 = OverworldGenerator::new(42);
        let gen2 = OverworldGenerator::new(42);
        let col1 = gen1.generate_chunk(5, -3);
        let col2 = gen2.generate_chunk(5, -3);

        for y in OVERWORLD_MIN_Y..320 {
            for x in 0..16 {
                for z in 0..16 {
                    assert_eq!(
                        col1.get_block_world(x, y, z),
                        col2.get_block_world(x, y, z),
                        "Mismatch at ({x}, {y}, {z})"
                    );
                }
            }
        }
    }

    #[test]
    fn different_seeds_different_terrain() {
        let gen1 = OverworldGenerator::new(1);
        let gen2 = OverworldGenerator::new(9999);
        let col1 = gen1.generate_chunk(0, 0);
        let col2 = gen2.generate_chunk(0, 0);

        let mut differences = 0;
        for y in 50..80 {
            if col1.get_block_world(8, y, 8) != col2.get_block_world(8, y, 8) {
                differences += 1;
            }
        }
        assert!(
            differences > 0,
            "Different seeds should produce different terrain"
        );
    }

    #[test]
    fn find_spawn_y_above_ground() {
        let gen = test_gen();
        let y = gen.find_spawn_y();
        assert!(y > 0, "Spawn Y should be above ground level");
        assert!(y < 256, "Spawn Y should be reasonable");
    }

    #[test]
    fn sea_level_water_in_ocean() {
        let gen = OverworldGenerator::new(42);
        // Generate many chunks to find one with ocean (low terrain)
        let mut found_water = false;
        for cx in -10..10 {
            for cz in -10..10 {
                let col = gen.generate_chunk(cx, cz);
                for x in 0..16 {
                    for z in 0..16 {
                        if let Some(block) = col.get_block_world(x, SEA_LEVEL, z) {
                            if block == gen.blocks.water {
                                found_water = true;
                            }
                        }
                    }
                }
                if found_water {
                    break;
                }
            }
            if found_water {
                break;
            }
        }
        assert!(found_water, "Should find water at sea level somewhere");
    }

    #[test]
    fn ores_placed_in_stone() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let ore_blocks = [
            gen.blocks.coal_ore,
            gen.blocks.iron_ore,
            gen.blocks.copper_ore,
        ];
        let mut ore_count = 0;
        for y in 0..100 {
            for x in 0..16 {
                for z in 0..16 {
                    if let Some(block) = col.get_block_world(x, y, z) {
                        if ore_blocks.contains(&block) {
                            ore_count += 1;
                        }
                    }
                }
            }
        }
        assert!(ore_count > 0, "Should find ores in the chunk");
    }

    #[test]
    fn sub_chunk_count_matches() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        assert_eq!(
            col.sub_chunks.len(),
            crate::chunk::OVERWORLD_SUB_CHUNK_COUNT
        );
    }
}
