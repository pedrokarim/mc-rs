//! Nether terrain generator.
//!
//! Generates Nether-like terrain: netherrack caves, lava sea, soul sand,
//! glowstone clusters, and ores. Y range: 0-127, bedrock floor and ceiling.
//! Uses the same 24-sub-chunk ChunkColumn but only populates Y=0..127.

#![allow(clippy::needless_range_loop)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::block_hash::NetherBlocks;
use crate::chunk::ChunkColumn;
use crate::noise::OctaveNoise;

/// Nether lava sea level.
pub const NETHER_LAVA_SEA: i32 = 31;

/// Nether ore configuration.
struct NetherOreConfig {
    block: u32,
    vein_size: u32,
    veins_per_chunk: u32,
    min_y: i32,
    max_y: i32,
}

/// Nether terrain generator with noise-based caves and features.
pub struct NetherGenerator {
    seed: u64,
    blocks: NetherBlocks,
    density_noise: OctaveNoise,
    detail_noise: OctaveNoise,
    soul_sand_noise: OctaveNoise,
}

impl NetherGenerator {
    /// Create a new Nether generator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            blocks: NetherBlocks::compute(),
            density_noise: OctaveNoise::new(seed.wrapping_add(5000), 4, 2.0, 0.5),
            detail_noise: OctaveNoise::new(seed.wrapping_add(6000), 3, 2.0, 0.5),
            soul_sand_noise: OctaveNoise::new(seed.wrapping_add(7000), 3, 2.0, 0.5),
        }
    }

    /// Generate a full chunk column at the given chunk coordinates.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> ChunkColumn {
        let mut column = ChunkColumn::new_air(chunk_x, chunk_z, self.blocks.air);

        // Phase 1: Fill Y=0..127 with netherrack
        self.fill_netherrack(&mut column);

        // Phase 2: Carve caves (large open caverns)
        self.carve_caves(&mut column, chunk_x, chunk_z);

        // Phase 3: Bedrock floor and ceiling
        self.place_bedrock(&mut column, chunk_x, chunk_z);

        // Phase 4: Lava sea (fill air below Y=31 with lava)
        self.fill_lava_sea(&mut column);

        // Phase 5: Soul sand patches
        self.place_soul_sand(&mut column, chunk_x, chunk_z);

        // Phase 6: Glowstone clusters on ceiling
        self.place_glowstone(&mut column, chunk_x, chunk_z);

        // Phase 7: Ores (quartz, gold)
        self.place_ores(&mut column, chunk_x, chunk_z);

        // Phase 8: Biome data (all nether_wastes = biome ID 8)
        for i in 0..256 {
            column.biomes[i] = 8;
        }

        column
    }

    /// Find a safe spawn Y in the Nether (first air above lava sea at chunk center).
    pub fn find_spawn_y(&self) -> i32 {
        let chunk = self.generate_chunk(0, 0);
        for y in (NETHER_LAVA_SEA + 1)..120 {
            if let Some(block) = chunk.get_block_world(8, y, 8) {
                if block == self.blocks.air {
                    if let Some(below) = chunk.get_block_world(8, y - 1, 8) {
                        if below != self.blocks.air && below != self.blocks.lava {
                            return y;
                        }
                    }
                }
            }
        }
        NETHER_LAVA_SEA + 2
    }

    fn fill_netherrack(&self, column: &mut ChunkColumn) {
        for lx in 0..16 {
            for lz in 0..16 {
                for y in 0..128 {
                    column.set_block_world(lx, y, lz, self.blocks.netherrack);
                }
            }
        }
    }

    fn carve_caves(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        for lx in 0..16 {
            for lz in 0..16 {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                for y in 1..127 {
                    let nx = wx as f64 / 48.0;
                    let ny = y as f64 / 32.0;
                    let nz = wz as f64 / 48.0;

                    let density = self.density_noise.sample_3d(nx, ny, nz);
                    let detail = self.detail_noise.sample_3d(nx * 2.0, ny * 2.0, nz * 2.0) * 0.15;

                    // Vertical gradient: more solid near floor and ceiling
                    let center_dist = ((y as f64 - 64.0) / 64.0).abs();
                    let vert_factor = (1.0 - center_dist * center_dist).max(0.0);

                    let combined = density + detail;

                    // Carve if noise is low enough (threshold modulated by vertical factor)
                    if combined < 0.3 * vert_factor - 0.1 {
                        column.set_block_world(lx, y, lz, self.blocks.air);
                    }
                }
            }
        }
    }

    fn place_bedrock(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        let mut rng = StdRng::seed_from_u64(
            self.seed
                .wrapping_add((cx as u64).wrapping_mul(1_000_003))
                .wrapping_add((cz as u64).wrapping_mul(999_983))
                .wrapping_add(0xBED),
        );

        for lx in 0..16 {
            for lz in 0..16 {
                // Floor: Y=0 always bedrock, Y=1..4 random
                column.set_block_world(lx, 0, lz, self.blocks.bedrock);
                for y in 1..5 {
                    if rng.gen_range(0u32..5) < (5 - y as u32) {
                        column.set_block_world(lx, y, lz, self.blocks.bedrock);
                    }
                }
                // Ceiling: Y=127 always bedrock, Y=123..126 random
                column.set_block_world(lx, 127, lz, self.blocks.bedrock);
                for y in 123..127 {
                    if rng.gen_range(0u32..5) < (y as u32 - 122) {
                        column.set_block_world(lx, y, lz, self.blocks.bedrock);
                    }
                }
            }
        }
    }

    fn fill_lava_sea(&self, column: &mut ChunkColumn) {
        for lx in 0..16 {
            for lz in 0..16 {
                for y in 1..=NETHER_LAVA_SEA {
                    if let Some(block) = column.get_block_world(lx, y, lz) {
                        if block == self.blocks.air {
                            column.set_block_world(lx, y, lz, self.blocks.lava);
                        }
                    }
                }
            }
        }
    }

    fn place_soul_sand(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        for lx in 0..16 {
            for lz in 0..16 {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;

                let noise_val = self
                    .soul_sand_noise
                    .sample_2d(wx as f64 / 32.0, wz as f64 / 32.0);

                // Soul sand patches where noise > 0.3
                if noise_val > 0.3 {
                    // Find exposed floor below Y=40
                    for y in (1..40).rev() {
                        if let Some(block) = column.get_block_world(lx, y, lz) {
                            if block == self.blocks.netherrack {
                                // Check block above is air or lava
                                if let Some(above) = column.get_block_world(lx, y + 1, lz) {
                                    if above == self.blocks.air || above == self.blocks.lava {
                                        column.set_block_world(lx, y, lz, self.blocks.soul_sand);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn place_glowstone(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        let mut rng = StdRng::seed_from_u64(
            self.seed
                .wrapping_mul(2347)
                .wrapping_add(cx as u64)
                .wrapping_mul(8761)
                .wrapping_add(cz as u64)
                .wrapping_add(0x610),
        );

        for lx in 0..16 {
            for lz in 0..16 {
                if rng.gen_range(0u32..20) != 0 {
                    continue; // ~5% chance per column
                }

                // Find ceiling: lowest netherrack from Y=126 downward
                for y in (32..126).rev() {
                    if let Some(block) = column.get_block_world(lx, y, lz) {
                        if block == self.blocks.netherrack {
                            if let Some(below) = column.get_block_world(lx, y - 1, lz) {
                                if below == self.blocks.air {
                                    // Place glowstone cluster hanging down
                                    let cluster_size = rng.gen_range(1..=4);
                                    for dy in 0..cluster_size {
                                        let gy = y - 1 - dy;
                                        if gy > 0 {
                                            column.set_block_world(
                                                lx,
                                                gy,
                                                lz,
                                                self.blocks.glowstone,
                                            );
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn place_ores(&self, column: &mut ChunkColumn, chunk_x: i32, chunk_z: i32) {
        let ores = [
            NetherOreConfig {
                block: self.blocks.nether_quartz_ore,
                vein_size: 14,
                veins_per_chunk: 16,
                min_y: 10,
                max_y: 117,
            },
            NetherOreConfig {
                block: self.blocks.nether_gold_ore,
                vein_size: 10,
                veins_per_chunk: 10,
                min_y: 10,
                max_y: 117,
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
                    .wrapping_add(ore_idx as u64)
                    .wrapping_add(0xAE7),
            );

            for _ in 0..ore.veins_per_chunk {
                let vx = rng.gen_range(0..16);
                let vz = rng.gen_range(0..16);
                let vy = rng.gen_range(ore.min_y..=ore.max_y);

                let mut cx = vx;
                let mut cy = vy;
                let mut cz = vz;

                for _ in 0..ore.vein_size {
                    if (0..16).contains(&cx) && (0..16).contains(&cz) {
                        if let Some(existing) = column.get_block_world(cx as usize, cy, cz as usize)
                        {
                            if existing == self.blocks.netherrack {
                                column.set_block_world(cx as usize, cy, cz as usize, ore.block);
                            }
                        }
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gen() -> NetherGenerator {
        NetherGenerator::new(42)
    }

    #[test]
    fn nether_deterministic() {
        let gen1 = NetherGenerator::new(42);
        let gen2 = NetherGenerator::new(42);
        let col1 = gen1.generate_chunk(3, -2);
        let col2 = gen2.generate_chunk(3, -2);

        for y in 0..128 {
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
    fn nether_bedrock_floor_and_ceiling() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(
                    col.get_block_world(x, 0, z),
                    Some(gen.blocks.bedrock),
                    "Floor bedrock missing at ({x}, 0, {z})"
                );
                assert_eq!(
                    col.get_block_world(x, 127, z),
                    Some(gen.blocks.bedrock),
                    "Ceiling bedrock missing at ({x}, 127, {z})"
                );
            }
        }
    }

    #[test]
    fn nether_has_netherrack() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut count = 0;
        for y in 0..128 {
            for x in 0..16 {
                for z in 0..16 {
                    if col.get_block_world(x, y, z) == Some(gen.blocks.netherrack) {
                        count += 1;
                    }
                }
            }
        }
        assert!(
            count > 1000,
            "Netherrack should be the dominant block, got {count}"
        );
    }

    #[test]
    fn nether_lava_sea_present() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut lava_count = 0;
        for y in 1..=NETHER_LAVA_SEA {
            for x in 0..16 {
                for z in 0..16 {
                    if col.get_block_world(x, y, z) == Some(gen.blocks.lava) {
                        lava_count += 1;
                    }
                }
            }
        }
        assert!(lava_count > 0, "Should have lava below sea level");
    }

    #[test]
    fn nether_has_caves() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut air_count = 0;
        for y in 32..120 {
            for x in 0..16 {
                for z in 0..16 {
                    if col.get_block_world(x, y, z) == Some(gen.blocks.air) {
                        air_count += 1;
                    }
                }
            }
        }
        assert!(
            air_count > 100,
            "Nether should have carved caves, got {air_count} air blocks"
        );
    }

    #[test]
    fn nether_has_quartz_ore() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut count = 0;
        for y in 10..117 {
            for x in 0..16 {
                for z in 0..16 {
                    if col.get_block_world(x, y, z) == Some(gen.blocks.nether_quartz_ore) {
                        count += 1;
                    }
                }
            }
        }
        assert!(count > 0, "Should find quartz ore in Nether");
    }

    #[test]
    fn nether_has_glowstone() {
        let gen = test_gen();
        // Check several chunks since glowstone is sparse
        let mut found = false;
        for cx in -2..=2 {
            for cz in -2..=2 {
                let col = gen.generate_chunk(cx, cz);
                for y in 32..126 {
                    for x in 0..16 {
                        for z in 0..16 {
                            if col.get_block_world(x, y, z) == Some(gen.blocks.glowstone) {
                                found = true;
                            }
                        }
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "Should find glowstone in Nether chunks");
    }

    #[test]
    fn nether_biome_is_nether_wastes() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        for &biome in col.biomes.iter() {
            assert_eq!(biome, 8, "All Nether biomes should be nether_wastes (8)");
        }
    }

    #[test]
    fn nether_find_spawn_valid() {
        let gen = test_gen();
        let y = gen.find_spawn_y();
        assert!(y > NETHER_LAVA_SEA, "Spawn should be above lava sea");
        assert!(y < 127, "Spawn should be below ceiling");
    }

    #[test]
    fn nether_has_soul_sand() {
        let gen = test_gen();
        // Check several chunks since soul sand is patchy
        let mut found = false;
        for cx in -3..=3 {
            for cz in -3..=3 {
                let col = gen.generate_chunk(cx, cz);
                for y in 1..40 {
                    for x in 0..16 {
                        for z in 0..16 {
                            if col.get_block_world(x, y, z) == Some(gen.blocks.soul_sand) {
                                found = true;
                            }
                        }
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "Should find soul sand in Nether chunks");
    }
}
