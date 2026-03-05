//! End dimension terrain generator.
//!
//! Generates The End: main island with end_stone, obsidian pillars,
//! void gap, and sparse outer islands. Uses the same 24-sub-chunk
//! ChunkColumn but only populates relevant Y ranges.

#![allow(clippy::needless_range_loop)]

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::block_hash::EndBlocks;
use crate::chunk::ChunkColumn;
use crate::noise::OctaveNoise;

/// Center Y for the main island platform.
const END_ISLAND_Y: i32 = 48;

/// Radius of the main island in blocks.
const MAIN_ISLAND_RADIUS: f64 = 80.0;

/// Distance threshold for outer islands (in blocks from origin).
const OUTER_ISLAND_MIN_DIST: f64 = 1000.0;

/// End terrain generator.
pub struct EndGenerator {
    seed: u64,
    blocks: EndBlocks,
    island_noise: OctaveNoise,
}

impl EndGenerator {
    /// Create a new End generator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            blocks: EndBlocks::compute(),
            island_noise: OctaveNoise::new(seed.wrapping_add(8000), 4, 2.0, 0.5),
        }
    }

    /// Generate a full chunk column at the given chunk coordinates.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> ChunkColumn {
        let mut column = ChunkColumn::new_air(chunk_x, chunk_z, self.blocks.air);

        // Compute distance from chunk center to world origin (in blocks)
        let center_wx = (chunk_x * 16 + 8) as f64;
        let center_wz = (chunk_z * 16 + 8) as f64;
        let dist = (center_wx * center_wx + center_wz * center_wz).sqrt();

        if dist < MAIN_ISLAND_RADIUS + 16.0 {
            // Main island generation
            self.generate_main_island(&mut column, chunk_x, chunk_z);

            // Obsidian pillars (only near center)
            if dist < 50.0 {
                self.place_obsidian_pillars(&mut column, chunk_x, chunk_z);
            }
        } else if dist > OUTER_ISLAND_MIN_DIST {
            // Outer islands (sparse)
            self.generate_outer_island(&mut column, chunk_x, chunk_z);
        }
        // Between main and outer islands: void (all air)

        // Biome data: the_end = biome ID 9
        for i in 0..256 {
            column.biomes[i] = 9;
        }

        column
    }

    /// Find a safe spawn Y on the main island.
    pub fn find_spawn_y(&self) -> i32 {
        let chunk = self.generate_chunk(0, 0);
        // Search downward from above the island
        for y in (END_ISLAND_Y..(END_ISLAND_Y + 60)).rev() {
            if let Some(block) = chunk.get_block_world(8, y, 8) {
                if block != self.blocks.air {
                    return y + 1;
                }
            }
        }
        END_ISLAND_Y + 16
    }

    fn generate_main_island(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        for lx in 0..16 {
            for lz in 0..16 {
                let wx = (cx * 16 + lx as i32) as f64;
                let wz = (cz * 16 + lz as i32) as f64;
                let dist = (wx * wx + wz * wz).sqrt();

                if dist > MAIN_ISLAND_RADIUS {
                    continue;
                }

                // Edge falloff: thinner at edges
                let edge_factor = 1.0 - (dist / MAIN_ISLAND_RADIUS).powi(2);
                let noise_val = self.island_noise.sample_2d(wx / 64.0, wz / 64.0);

                let thickness = (12.0 * edge_factor + noise_val * 4.0).max(1.0) as i32;
                let top_y = END_ISLAND_Y + (noise_val * 3.0) as i32;
                let bottom_y = (top_y - thickness).max(0);

                for y in bottom_y..=top_y {
                    column.set_block_world(lx, y, lz, self.blocks.end_stone);
                }
            }
        }
    }

    fn place_obsidian_pillars(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        // Fixed pillar positions determined by seed (same for all chunks)
        let mut rng = StdRng::seed_from_u64(self.seed.wrapping_add(0xE1D));
        let num_pillars = rng.gen_range(5u32..=10);

        for _ in 0..num_pillars {
            let px = rng.gen_range(-30..=30);
            let pz = rng.gen_range(-30..=30);
            let height = rng.gen_range(20..=50);

            // Check if this pillar's 3×3 footprint intersects this chunk
            for dx in -1..=1i32 {
                for dz in -1..=1i32 {
                    let wx = px + dx;
                    let wz = pz + dz;
                    let lx = wx - cx * 16;
                    let lz = wz - cz * 16;

                    if !(0..16).contains(&lx) || !(0..16).contains(&lz) {
                        continue;
                    }

                    // Find island surface at this position
                    let mut base_y = END_ISLAND_Y;
                    for y in (END_ISLAND_Y..(END_ISLAND_Y + 20)).rev() {
                        if let Some(block) = column.get_block_world(lx as usize, y, lz as usize) {
                            if block == self.blocks.end_stone {
                                base_y = y + 1;
                                break;
                            }
                        }
                    }

                    for y in base_y..(base_y + height) {
                        column.set_block_world(lx as usize, y, lz as usize, self.blocks.obsidian);
                    }
                    // Bedrock cap
                    column.set_block_world(
                        lx as usize,
                        base_y + height,
                        lz as usize,
                        self.blocks.bedrock,
                    );
                }
            }
        }
    }

    fn generate_outer_island(&self, column: &mut ChunkColumn, cx: i32, cz: i32) {
        // Deterministic chance per chunk (~2%)
        let mut rng = StdRng::seed_from_u64(
            self.seed
                .wrapping_mul(9001)
                .wrapping_add(cx as u64)
                .wrapping_mul(7333)
                .wrapping_add(cz as u64)
                .wrapping_add(0xE0),
        );

        if rng.gen_range(0u32..50) != 0 {
            return;
        }

        // Small end_stone platform
        let radius = rng.gen_range(4..=8);
        let thickness = rng.gen_range(3..=6);
        let base_y = END_ISLAND_Y + rng.gen_range(-10..=10);

        for lx in 0..16 {
            for lz in 0..16 {
                let dx = lx as i32 - 8;
                let dz = lz as i32 - 8;
                let dist_sq = dx * dx + dz * dz;

                if dist_sq <= radius * radius {
                    let edge_factor = 1.0 - (dist_sq as f64 / (radius * radius) as f64);
                    let local_thickness = (thickness as f64 * edge_factor).max(1.0) as i32;

                    for dy in 0..local_thickness {
                        column.set_block_world(lx, base_y - dy, lz, self.blocks.end_stone);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_gen() -> EndGenerator {
        EndGenerator::new(42)
    }

    #[test]
    fn end_deterministic() {
        let gen1 = EndGenerator::new(42);
        let gen2 = EndGenerator::new(42);
        let col1 = gen1.generate_chunk(0, 0);
        let col2 = gen2.generate_chunk(0, 0);

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
    fn end_main_island_has_endstone() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        let mut count = 0;
        for y in 30..70 {
            for x in 0..16 {
                for z in 0..16 {
                    if col.get_block_world(x, y, z) == Some(gen.blocks.end_stone) {
                        count += 1;
                    }
                }
            }
        }
        assert!(
            count > 100,
            "Main island chunk should have end_stone, got {count}"
        );
    }

    #[test]
    fn end_void_between_islands() {
        let gen = test_gen();
        // Chunk at (10, 10) = 160 blocks from origin, should be void
        let col = gen.generate_chunk(10, 10);
        let mut non_air = 0;
        for y in 0..256 {
            for x in 0..16 {
                for z in 0..16 {
                    if let Some(block) = col.get_block_world(x, y, z) {
                        if block != gen.blocks.air {
                            non_air += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            non_air, 0,
            "Void region should be all air, got {non_air} blocks"
        );
    }

    #[test]
    fn end_obsidian_pillars() {
        let gen = test_gen();
        // Check chunks near origin for obsidian
        let mut found = false;
        for cx in -2..=2 {
            for cz in -2..=2 {
                let col = gen.generate_chunk(cx, cz);
                for y in END_ISLAND_Y..(END_ISLAND_Y + 60) {
                    for x in 0..16 {
                        for z in 0..16 {
                            if col.get_block_world(x, y, z) == Some(gen.blocks.obsidian) {
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
        assert!(found, "Should find obsidian pillars near origin");
    }

    #[test]
    fn end_biome_is_the_end() {
        let gen = test_gen();
        let col = gen.generate_chunk(0, 0);
        for &biome in col.biomes.iter() {
            assert_eq!(biome, 9, "All End biomes should be the_end (9)");
        }
    }

    #[test]
    fn end_find_spawn_valid() {
        let gen = test_gen();
        let y = gen.find_spawn_y();
        assert!(y > END_ISLAND_Y, "Spawn should be above island base");
        assert!(y < END_ISLAND_Y + 80, "Spawn should be reasonable height");
    }

    #[test]
    fn end_bedrock_on_pillars() {
        let gen = test_gen();
        // Check for bedrock anywhere near origin (pillar caps)
        let mut found = false;
        for cx in -2..=2 {
            for cz in -2..=2 {
                let col = gen.generate_chunk(cx, cz);
                for y in (END_ISLAND_Y + 20)..(END_ISLAND_Y + 60) {
                    for x in 0..16 {
                        for z in 0..16 {
                            if col.get_block_world(x, y, z) == Some(gen.blocks.bedrock) {
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
        assert!(found, "Should find bedrock caps on obsidian pillars");
    }

    #[test]
    fn end_outer_islands_sparse() {
        let gen = test_gen();
        // At dist > 1000 blocks (chunk 63+), some chunks should have end_stone
        let mut found = false;
        for cx in 63..80 {
            for cz in 63..80 {
                let col = gen.generate_chunk(cx, cz);
                for y in 30..70 {
                    for x in 0..16 {
                        for z in 0..16 {
                            if col.get_block_world(x, y, z) == Some(gen.blocks.end_stone) {
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
        assert!(found, "Should find outer island end_stone in far chunks");
    }
}
