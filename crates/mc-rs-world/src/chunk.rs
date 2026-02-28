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
    /// 2D biome map: one biome ID per XZ column, indexed `[x * 16 + z]`.
    pub biomes: [u8; 256],
    /// Whether this chunk has unsaved modifications.
    pub dirty: bool,
    /// Cached serialized payload: `(sub_chunk_count, payload_bytes)`.
    /// Set to `None` when the chunk is modified.
    pub cached_payload: Option<(u32, Vec<u8>)>,
}

impl ChunkColumn {
    /// Create a new chunk column filled entirely with air (or any single block).
    pub fn new_air(x: i32, z: i32, air_id: u32) -> Self {
        Self {
            x,
            z,
            sub_chunks: std::array::from_fn(|_| SubChunk::new_single(air_id)),
            biomes: [0; 256],
            dirty: false,
            cached_payload: None,
        }
    }

    /// Set a block using local x (0..16), world y, local z (0..16).
    /// Returns false if y is out of range.
    pub fn set_block_world(
        &mut self,
        local_x: usize,
        world_y: i32,
        local_z: usize,
        runtime_id: u32,
    ) -> bool {
        let shifted = world_y - OVERWORLD_MIN_Y;
        if shifted < 0 || shifted >= (OVERWORLD_SUB_CHUNK_COUNT as i32 * 16) {
            return false;
        }
        let sub_index = shifted as usize / 16;
        let local_y = shifted as usize % 16;
        self.sub_chunks[sub_index].set_block(local_x, local_y, local_z, runtime_id);
        self.cached_payload = None;
        true
    }

    /// Get the runtime ID of a block using local x (0..16), world y, local z (0..16).
    /// Returns `None` if y is out of range.
    pub fn get_block_world(&self, local_x: usize, world_y: i32, local_z: usize) -> Option<u32> {
        let shifted = world_y - OVERWORLD_MIN_Y;
        if shifted < 0 || shifted >= (OVERWORLD_SUB_CHUNK_COUNT as i32 * 16) {
            return None;
        }
        let sub_index = shifted as usize / 16;
        let local_y = shifted as usize % 16;
        Some(self.sub_chunks[sub_index].get_block(local_x, local_y, local_z))
    }
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

    #[test]
    fn chunk_column_new_air() {
        let col = ChunkColumn::new_air(3, -5, 999);
        assert_eq!(col.x, 3);
        assert_eq!(col.z, -5);
        assert_eq!(col.biomes, [0u8; 256]);
        // All blocks should be the air ID
        assert_eq!(col.get_block_world(0, 0, 0), Some(999));
        assert_eq!(col.get_block_world(8, 100, 8), Some(999));
    }

    #[test]
    fn set_get_block_world() {
        let mut col = ChunkColumn::new_air(0, 0, 1);
        // Set block at Y=0 (sub_chunk index 4, local_y=0)
        assert!(col.set_block_world(5, 0, 5, 42));
        assert_eq!(col.get_block_world(5, 0, 5), Some(42));
        // Set block at Y=-64 (sub_chunk 0, local_y=0)
        assert!(col.set_block_world(0, -64, 0, 77));
        assert_eq!(col.get_block_world(0, -64, 0), Some(77));
        // Set block at Y=319 (sub_chunk 23, local_y=15)
        assert!(col.set_block_world(0, 319, 0, 88));
        assert_eq!(col.get_block_world(0, 319, 0), Some(88));
        // Out of range
        assert!(!col.set_block_world(0, -65, 0, 99));
        assert!(!col.set_block_world(0, 320, 0, 99));
        assert_eq!(col.get_block_world(0, -65, 0), None);
    }

    #[test]
    fn biome_storage() {
        let mut col = ChunkColumn::new_air(0, 0, 1);
        col.biomes[3 * 16 + 5] = 42;
        assert_eq!(col.biomes[3 * 16 + 5], 42);
        assert_eq!(col.biomes[0], 0); // others still default
    }
}
