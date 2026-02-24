//! Chunk and sub-chunk data structures.

/// Total sub-chunks for Overworld: Y range [-64, 319] = 384 blocks / 16 = 24.
pub const OVERWORLD_SUB_CHUNK_COUNT: usize = 24;

/// Minimum Y coordinate in the Overworld.
pub const OVERWORLD_MIN_Y: i32 = -64;

/// A 16x16x16 sub-chunk with a single block storage layer.
pub struct SubChunk {
    /// Palette indices for each block, stored in XZY order: `(x*16 + z)*16 + y`.
    pub blocks: [u16; 4096],
    /// Palette of block runtime IDs (FNV-1a hashes).
    pub palette: Vec<u32>,
}

/// A full chunk column (16x384x16 for Overworld).
pub struct ChunkColumn {
    pub x: i32,
    pub z: i32,
    pub sub_chunks: [SubChunk; OVERWORLD_SUB_CHUNK_COUNT],
}

impl SubChunk {
    /// Create a sub-chunk filled entirely with a single block.
    pub fn new_single(runtime_id: u32) -> Self {
        Self {
            blocks: [0; 4096],
            palette: vec![runtime_id],
        }
    }

    /// Set a block at local coordinates within this sub-chunk.
    /// `x`, `y`, `z` must each be in `[0, 15]`.
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, runtime_id: u32) {
        debug_assert!(x < 16 && y < 16 && z < 16);
        let palette_index = match self.palette.iter().position(|&id| id == runtime_id) {
            Some(idx) => idx,
            None => {
                self.palette.push(runtime_id);
                self.palette.len() - 1
            }
        };
        let block_index = (x * 16 + z) * 16 + y;
        self.blocks[block_index] = palette_index as u16;
    }

    /// Get the runtime ID of the block at local coordinates.
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u32 {
        let block_index = (x * 16 + z) * 16 + y;
        let palette_index = self.blocks[block_index] as usize;
        self.palette[palette_index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_block_subchunk() {
        let sub = SubChunk::new_single(42);
        assert_eq!(sub.palette.len(), 1);
        assert_eq!(sub.get_block(0, 0, 0), 42);
        assert_eq!(sub.get_block(15, 15, 15), 42);
    }

    #[test]
    fn set_get_roundtrip() {
        let mut sub = SubChunk::new_single(100);
        sub.set_block(5, 10, 3, 200);
        assert_eq!(sub.get_block(5, 10, 3), 200);
        assert_eq!(sub.get_block(0, 0, 0), 100);
    }

    #[test]
    fn palette_growth() {
        let mut sub = SubChunk::new_single(1);
        assert_eq!(sub.palette.len(), 1);
        sub.set_block(0, 0, 0, 2);
        assert_eq!(sub.palette.len(), 2);
        sub.set_block(0, 1, 0, 3);
        assert_eq!(sub.palette.len(), 3);
        // Setting a block with an existing runtime ID should not grow the palette
        sub.set_block(0, 2, 0, 2);
        assert_eq!(sub.palette.len(), 3);
    }

    #[test]
    fn xzy_ordering() {
        let mut sub = SubChunk::new_single(0);
        sub.set_block(1, 2, 3, 99);
        // Verify using raw index: (x*16 + z)*16 + y = (1*16 + 3)*16 + 2 = 306
        let idx = sub.blocks[306] as usize;
        assert_eq!(sub.palette[idx], 99);
    }
}
