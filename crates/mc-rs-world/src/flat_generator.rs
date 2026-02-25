//! Flat world chunk generator.
//!
//! Generates a default Bedrock flat world:
//! - Y = 0: Bedrock
//! - Y = 1-2: Dirt
//! - Y = 3: Grass Block
//! - Y = 4+: Air

use crate::block_hash::FlatWorldBlocks;
use crate::chunk::{ChunkColumn, SubChunk, OVERWORLD_SUB_CHUNK_COUNT};

/// Generate a flat world chunk column at the given chunk coordinates.
pub fn generate_flat_chunk(x: i32, z: i32, blocks: &FlatWorldBlocks) -> ChunkColumn {
    // Build sub-chunks as a Vec first, then convert to array
    let sub_chunks: Vec<SubChunk> = (0..OVERWORLD_SUB_CHUNK_COUNT)
        .map(|index| {
            if index == 4 {
                // Sub-chunk 4 covers Y=0..15 — contains the flat world layers
                generate_mixed_sub_chunk(blocks)
            } else {
                // All other sub-chunks are air
                SubChunk::new_single(blocks.air)
            }
        })
        .collect();

    ChunkColumn {
        x,
        z,
        sub_chunks: sub_chunks
            .try_into()
            .unwrap_or_else(|_| panic!("expected {OVERWORLD_SUB_CHUNK_COUNT} sub-chunks")),
        biomes: [1u8; 256], // All plains
    }
}

/// Generate sub-chunk 4 (Y=0..15) with bedrock, dirt, grass, and air layers.
fn generate_mixed_sub_chunk(blocks: &FlatWorldBlocks) -> SubChunk {
    let mut sub = SubChunk::new_single(blocks.air);

    for x in 0..16 {
        for z in 0..16 {
            sub.set_block(x, 0, z, blocks.bedrock); // Y=0: bedrock
            sub.set_block(x, 1, z, blocks.dirt); // Y=1: dirt
            sub.set_block(x, 2, z, blocks.dirt); // Y=2: dirt
            sub.set_block(x, 3, z, blocks.grass_block); // Y=3: grass
                                                        // Y=4..15: air (already set)
        }
    }

    sub
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_blocks() -> FlatWorldBlocks {
        FlatWorldBlocks::compute()
    }

    #[test]
    fn flat_chunk_layer_layout() {
        let blocks = test_blocks();
        let col = generate_flat_chunk(0, 0, &blocks);
        let mixed = &col.sub_chunks[4];

        assert_eq!(mixed.get_block(0, 0, 0), blocks.bedrock);
        assert_eq!(mixed.get_block(0, 1, 0), blocks.dirt);
        assert_eq!(mixed.get_block(0, 2, 0), blocks.dirt);
        assert_eq!(mixed.get_block(0, 3, 0), blocks.grass_block);
        assert_eq!(mixed.get_block(0, 4, 0), blocks.air);
        assert_eq!(mixed.get_block(0, 15, 0), blocks.air);
    }

    #[test]
    fn air_subchunks() {
        let blocks = test_blocks();
        let col = generate_flat_chunk(0, 0, &blocks);

        // Sub-chunks 0-3 should be all air
        for i in 0..4 {
            assert_eq!(col.sub_chunks[i].palette.len(), 1);
            assert_eq!(col.sub_chunks[i].palette[0], blocks.air);
        }

        // Sub-chunks 5-23 should be all air
        for i in 5..24 {
            assert_eq!(col.sub_chunks[i].palette.len(), 1);
            assert_eq!(col.sub_chunks[i].palette[0], blocks.air);
        }
    }

    #[test]
    fn mixed_subchunk_palette() {
        let blocks = test_blocks();
        let col = generate_flat_chunk(0, 0, &blocks);
        // Sub-chunk 4 should have 4 unique block types
        assert_eq!(col.sub_chunks[4].palette.len(), 4);
    }

    #[test]
    fn uniform_across_xz() {
        let blocks = test_blocks();
        let col = generate_flat_chunk(5, -3, &blocks);
        let mixed = &col.sub_chunks[4];

        // Every (x, z) column should have the same pattern
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(mixed.get_block(x, 0, z), blocks.bedrock);
                assert_eq!(mixed.get_block(x, 3, z), blocks.grass_block);
            }
        }
    }
}
