//! LevelDB-based chunk storage for world persistence.
//!
//! Uses standard Bedrock LevelDB key format with a custom sub-chunk palette
//! format (FNV-1a hashes instead of NBT compounds). Full BDS compatibility
//! is deferred to Phase 5.5.

use std::path::Path;

use rusty_leveldb::DB;

use crate::chunk::{ChunkColumn, SubChunk, OVERWORLD_SUB_CHUNK_COUNT};

// ─── LevelDB key tags ───────────────────────────────────────────────────────

const TAG_CHUNK_VERSION: u8 = 0x2C;
const TAG_DATA_2D: u8 = 0x2D;
const TAG_SUB_CHUNK_PREFIX: u8 = 0x2F;
const TAG_FINALIZED_STATE: u8 = 0x36;

/// Current chunk format version.
const CHUNK_VERSION: u8 = 40;

// ─── Key builders ───────────────────────────────────────────────────────────

/// Build a LevelDB key: `[X:i32_le][Z:i32_le][tag]` (overworld, no dimension prefix).
fn chunk_key(cx: i32, cz: i32, tag: u8) -> Vec<u8> {
    chunk_key_dim(cx, cz, 0, tag)
}

/// Build a dimension-aware LevelDB key.
///
/// Overworld (dim=0): `[X:i32_le][Z:i32_le][tag]`
/// Nether (dim=1): `[X:i32_le][Z:i32_le][01 00 00 00][tag]`
/// End (dim=2): `[X:i32_le][Z:i32_le][02 00 00 00][tag]`
fn chunk_key_dim(cx: i32, cz: i32, dim: i32, tag: u8) -> Vec<u8> {
    let cap = if dim == 0 { 9 } else { 13 };
    let mut key = Vec::with_capacity(cap);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    if dim != 0 {
        key.extend_from_slice(&dim.to_le_bytes());
    }
    key.push(tag);
    key
}

/// Build a sub-chunk LevelDB key: `[X:i32_le][Z:i32_le][0x2F][y_index]` (overworld).
#[cfg(test)]
fn sub_chunk_key(cx: i32, cz: i32, y_index: i8) -> Vec<u8> {
    sub_chunk_key_dim(cx, cz, 0, y_index)
}

/// Build a dimension-aware sub-chunk LevelDB key.
fn sub_chunk_key_dim(cx: i32, cz: i32, dim: i32, y_index: i8) -> Vec<u8> {
    let cap = if dim == 0 { 10 } else { 14 };
    let mut key = Vec::with_capacity(cap);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    if dim != 0 {
        key.extend_from_slice(&dim.to_le_bytes());
    }
    key.push(TAG_SUB_CHUNK_PREFIX);
    key.push(y_index as u8);
    key
}

// ─── Disk serialization (custom palette format) ─────────────────────────────

/// Determine minimum bits-per-block for a given palette size.
/// Valid values: 0, 1, 2, 3, 4, 5, 6, 8, 16.
fn bits_per_block_for_palette(palette_size: usize) -> u8 {
    match palette_size {
        0..=1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        65..=256 => 8,
        _ => 16,
    }
}

/// Serialize a sub-chunk to disk format.
///
/// Format: `[version=9][num_layers=1][palette_header][block_data...][palette_size:i32_le][palette:u32_le[]]`
/// Palette header bit 0 = 0 (persistence mode).
fn serialize_sub_chunk_disk(sub: &SubChunk) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(9); // version
    buf.push(1); // num_layers

    let palette_size = sub.palette.len();

    if palette_size <= 1 {
        // Single-block: bpb=0, header = (0 << 1) | 0 = 0
        buf.push(0x00);
        // palette_size as i32_le
        buf.extend_from_slice(&(palette_size as i32).to_le_bytes());
        if palette_size == 1 {
            buf.extend_from_slice(&sub.palette[0].to_le_bytes());
        }
    } else {
        let bpb = bits_per_block_for_palette(palette_size);
        let header = bpb << 1; // bit 0 = 0 for persistence
        buf.push(header);

        // Pack block indices into u32 words (LSB-first)
        let blocks_per_word = 32 / bpb as usize;
        let word_count = 4096_usize.div_ceil(blocks_per_word);

        for word_idx in 0..word_count {
            let mut word: u32 = 0;
            for slot in 0..blocks_per_word {
                let block_idx = word_idx * blocks_per_word + slot;
                if block_idx < 4096 {
                    let palette_index = sub.blocks[block_idx] as u32;
                    word |= palette_index << (bpb as u32 * slot as u32);
                }
            }
            buf.extend_from_slice(&word.to_le_bytes());
        }

        // Palette as i32_le size + u32_le entries
        buf.extend_from_slice(&(palette_size as i32).to_le_bytes());
        for &runtime_id in &sub.palette {
            buf.extend_from_slice(&runtime_id.to_le_bytes());
        }
    }

    buf
}

/// Deserialize a sub-chunk from disk format.
fn deserialize_sub_chunk_disk(data: &[u8]) -> Option<SubChunk> {
    if data.len() < 3 {
        return None;
    }

    let version = data[0];
    if version != 9 {
        return None;
    }

    let _num_layers = data[1];
    let palette_header = data[2];
    let bpb = palette_header >> 1;

    let mut pos = 3;

    if bpb == 0 {
        // Single-block sub-chunk
        if pos + 4 > data.len() {
            return None;
        }
        let palette_size =
            i32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if palette_size == 0 {
            return Some(SubChunk::new_single(0));
        }

        if pos + 4 > data.len() {
            return None;
        }
        let runtime_id =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        Some(SubChunk::new_single(runtime_id))
    } else {
        // Multi-block sub-chunk
        let blocks_per_word = 32 / bpb as usize;
        let word_count = 4096_usize.div_ceil(blocks_per_word);
        let block_data_bytes = word_count * 4;

        if pos + block_data_bytes + 4 > data.len() {
            return None;
        }

        // Read block data
        let mut blocks = [0u16; 4096];
        let mask = (1u32 << bpb) - 1;

        for word_idx in 0..word_count {
            let offset = pos + word_idx * 4;
            let word = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            for slot in 0..blocks_per_word {
                let block_idx = word_idx * blocks_per_word + slot;
                if block_idx < 4096 {
                    let palette_index = (word >> (bpb as u32 * slot as u32)) & mask;
                    blocks[block_idx] = palette_index as u16;
                }
            }
        }

        pos += block_data_bytes;

        // Read palette
        let palette_size =
            i32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + palette_size * 4 > data.len() {
            return None;
        }

        let mut palette = Vec::with_capacity(palette_size);
        for i in 0..palette_size {
            let offset = pos + i * 4;
            let runtime_id = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            palette.push(runtime_id);
        }

        Some(SubChunk { blocks, palette })
    }
}

/// Serialize Data2D: heightmap (i16_le[256]) + biomes (u8[256]).
fn serialize_data_2d(biomes: &[u8; 256]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(768); // 512 + 256
                                           // Heightmap: all zeros (we don't track heightmap separately)
    for _ in 0..256 {
        buf.extend_from_slice(&0i16.to_le_bytes());
    }
    // Biomes
    buf.extend_from_slice(biomes);
    buf
}

/// Deserialize Data2D: skip heightmap, read biomes.
fn deserialize_data_2d(data: &[u8]) -> Option<[u8; 256]> {
    // 256 * 2 (heightmap) + 256 (biomes) = 768 bytes
    if data.len() < 768 {
        return None;
    }
    let mut biomes = [0u8; 256];
    biomes.copy_from_slice(&data[512..768]);
    Some(biomes)
}

// ─── LevelDB provider ──────────────────────────────────────────────────────

/// Wraps a `rusty_leveldb::DB` for chunk persistence.
pub struct LevelDbProvider {
    db: DB,
}

impl LevelDbProvider {
    /// Open or create a LevelDB database at the given path.
    pub fn open(path: &Path) -> Result<Self, String> {
        let opts = rusty_leveldb::Options {
            create_if_missing: true,
            ..rusty_leveldb::Options::default()
        };

        let db = DB::open(path, opts).map_err(|e| format!("Failed to open LevelDB: {e}"))?;
        Ok(Self { db })
    }

    /// Load a chunk from LevelDB (overworld). Returns `None` if the chunk doesn't exist.
    pub fn load_chunk(&mut self, cx: i32, cz: i32) -> Option<ChunkColumn> {
        self.load_chunk_dim(cx, cz, 0)
    }

    /// Load a chunk from LevelDB for a specific dimension.
    pub fn load_chunk_dim(&mut self, cx: i32, cz: i32, dim: i32) -> Option<ChunkColumn> {
        // Check if chunk version key exists
        let version_key = chunk_key_dim(cx, cz, dim, TAG_CHUNK_VERSION);
        self.db.get(&version_key)?;

        // Load biomes from Data2D
        let data_2d_key = chunk_key_dim(cx, cz, dim, TAG_DATA_2D);
        let biomes = if let Some(data) = self.db.get(&data_2d_key) {
            deserialize_data_2d(&data).unwrap_or([0u8; 256])
        } else {
            [0u8; 256]
        };

        // Load 24 sub-chunks
        let sub_chunks: Vec<SubChunk> = (0..OVERWORLD_SUB_CHUNK_COUNT)
            .map(|i| {
                let y_index = i as i8 - 4; // 0 -> -4, 23 -> 19
                let key = sub_chunk_key_dim(cx, cz, dim, y_index);
                if let Some(data) = self.db.get(&key) {
                    deserialize_sub_chunk_disk(&data).unwrap_or_else(|| SubChunk::new_single(0))
                } else {
                    SubChunk::new_single(0) // air
                }
            })
            .collect();

        let sub_chunks: [SubChunk; OVERWORLD_SUB_CHUNK_COUNT] = sub_chunks
            .try_into()
            .unwrap_or_else(|_| panic!("expected {OVERWORLD_SUB_CHUNK_COUNT} sub-chunks"));

        Some(ChunkColumn {
            x: cx,
            z: cz,
            sub_chunks,
            biomes,
            dirty: false,
            cached_payload: None,
        })
    }

    /// Save a chunk to LevelDB (overworld).
    pub fn save_chunk(&mut self, column: &ChunkColumn) -> Result<(), String> {
        self.save_chunk_dim(column, 0)
    }

    /// Save a chunk to LevelDB for a specific dimension.
    pub fn save_chunk_dim(&mut self, column: &ChunkColumn, dim: i32) -> Result<(), String> {
        let cx = column.x;
        let cz = column.z;

        // Write chunk version
        let version_key = chunk_key_dim(cx, cz, dim, TAG_CHUNK_VERSION);
        self.db
            .put(&version_key, &[CHUNK_VERSION])
            .map_err(|e| format!("put version: {e}"))?;

        // Write Data2D (heightmap + biomes)
        let data_2d_key = chunk_key_dim(cx, cz, dim, TAG_DATA_2D);
        let data_2d = serialize_data_2d(&column.biomes);
        self.db
            .put(&data_2d_key, &data_2d)
            .map_err(|e| format!("put data2d: {e}"))?;

        // Write 24 sub-chunks
        for (i, sub_chunk) in column.sub_chunks.iter().enumerate() {
            let y_index = i as i8 - 4;
            let key = sub_chunk_key_dim(cx, cz, dim, y_index);
            let data = serialize_sub_chunk_disk(sub_chunk);
            self.db
                .put(&key, &data)
                .map_err(|e| format!("put sub-chunk {y_index}: {e}"))?;
        }

        // Write finalized state = 2 (done)
        let finalized_key = chunk_key_dim(cx, cz, dim, TAG_FINALIZED_STATE);
        self.db
            .put(&finalized_key, &2i32.to_le_bytes())
            .map_err(|e| format!("put finalized: {e}"))?;

        Ok(())
    }

    /// Flush pending writes to disk.
    pub fn flush(&mut self) -> Result<(), String> {
        self.db.flush().map_err(|e| format!("flush: {e}"))
    }

    /// Raw get from LevelDB (for block entity data, etc.).
    pub fn get_raw(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.get(key)
    }

    /// Raw put to LevelDB (for block entity data, etc.).
    pub fn put_raw(&mut self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.db.put(key, value).map_err(|e| format!("put_raw: {e}"))
    }
}

/// Block entity LevelDB tag.
const TAG_BLOCK_ENTITY: u8 = 0x31;

/// Build a LevelDB key for block entity data in a chunk (overworld).
pub fn block_entity_key(cx: i32, cz: i32) -> Vec<u8> {
    chunk_key(cx, cz, TAG_BLOCK_ENTITY)
}

/// Build a LevelDB key for block entity data in a chunk for a specific dimension.
pub fn block_entity_key_dim(cx: i32, cz: i32, dim: i32) -> Vec<u8> {
    chunk_key_dim(cx, cz, dim, TAG_BLOCK_ENTITY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ─── Key tests ──────────────────────────────────────────────────────

    #[test]
    fn chunk_key_dim_overworld_no_prefix() {
        let key = chunk_key_dim(10, -5, 0, TAG_CHUNK_VERSION);
        assert_eq!(key.len(), 9); // no dimension prefix for overworld
        assert_eq!(&key[0..4], &10i32.to_le_bytes());
        assert_eq!(&key[4..8], &(-5i32).to_le_bytes());
        assert_eq!(key[8], TAG_CHUNK_VERSION);
    }

    #[test]
    fn chunk_key_dim_nether_has_prefix() {
        let key = chunk_key_dim(10, -5, 1, TAG_CHUNK_VERSION);
        assert_eq!(key.len(), 13); // +4 bytes for dimension
        assert_eq!(&key[0..4], &10i32.to_le_bytes());
        assert_eq!(&key[4..8], &(-5i32).to_le_bytes());
        assert_eq!(&key[8..12], &1i32.to_le_bytes());
        assert_eq!(key[12], TAG_CHUNK_VERSION);
    }

    #[test]
    fn chunk_key_dim_end_has_prefix() {
        let key = chunk_key_dim(0, 0, 2, TAG_DATA_2D);
        assert_eq!(key.len(), 13);
        assert_eq!(&key[8..12], &2i32.to_le_bytes());
        assert_eq!(key[12], TAG_DATA_2D);
    }

    #[test]
    fn sub_chunk_key_dim_nether() {
        let key = sub_chunk_key_dim(5, 3, 1, 2);
        assert_eq!(key.len(), 14); // +4 for dimension
        assert_eq!(&key[0..4], &5i32.to_le_bytes());
        assert_eq!(&key[4..8], &3i32.to_le_bytes());
        assert_eq!(&key[8..12], &1i32.to_le_bytes());
        assert_eq!(key[12], TAG_SUB_CHUNK_PREFIX);
        assert_eq!(key[13], 2);
    }

    #[test]
    fn chunk_key_bytes() {
        let key = chunk_key(10, -5, TAG_CHUNK_VERSION);
        assert_eq!(key.len(), 9);
        // X = 10 as i32_le
        assert_eq!(&key[0..4], &10i32.to_le_bytes());
        // Z = -5 as i32_le
        assert_eq!(&key[4..8], &(-5i32).to_le_bytes());
        // tag
        assert_eq!(key[8], TAG_CHUNK_VERSION);
    }

    #[test]
    fn sub_chunk_key_bytes() {
        // Example from docs: chunk (10, -5) sub-chunk y=3
        let key = sub_chunk_key(10, -5, 3);
        assert_eq!(key.len(), 10);
        assert_eq!(&key[0..4], &[0x0A, 0x00, 0x00, 0x00]); // X=10
        assert_eq!(&key[4..8], &[0xFB, 0xFF, 0xFF, 0xFF]); // Z=-5
        assert_eq!(key[8], 0x2F); // SubChunkPrefix
        assert_eq!(key[9], 3); // y_index
    }

    #[test]
    fn sub_chunk_key_negative_y() {
        let key = sub_chunk_key(0, 0, -4);
        assert_eq!(key[9], 0xFC); // -4 as u8 (two's complement)
    }

    #[test]
    fn chunk_key_zero_coords() {
        let key = chunk_key(0, 0, TAG_DATA_2D);
        assert_eq!(&key[0..8], &[0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(key[8], TAG_DATA_2D);
    }

    // ─── Disk serialization tests ───────────────────────────────────────

    #[test]
    fn roundtrip_single_block_subchunk() {
        let sub = SubChunk::new_single(42);
        let data = serialize_sub_chunk_disk(&sub);
        let restored = deserialize_sub_chunk_disk(&data).unwrap();
        assert_eq!(restored.palette.len(), 1);
        assert_eq!(restored.palette[0], 42);
        assert_eq!(restored.get_block(0, 0, 0), 42);
        assert_eq!(restored.get_block(15, 15, 15), 42);
    }

    #[test]
    fn roundtrip_mixed_subchunk() {
        let mut sub = SubChunk::new_single(100);
        sub.set_block(0, 0, 0, 200);
        sub.set_block(5, 10, 3, 300);
        sub.set_block(15, 15, 15, 400);

        let data = serialize_sub_chunk_disk(&sub);
        let restored = deserialize_sub_chunk_disk(&data).unwrap();

        assert_eq!(restored.palette.len(), 4);
        assert_eq!(restored.get_block(0, 0, 0), 200);
        assert_eq!(restored.get_block(5, 10, 3), 300);
        assert_eq!(restored.get_block(15, 15, 15), 400);
        assert_eq!(restored.get_block(1, 0, 0), 100); // default fill
    }

    #[test]
    fn persistence_type_bit_is_zero() {
        let sub = SubChunk::new_single(42);
        let data = serialize_sub_chunk_disk(&sub);
        // data[2] = palette_header; bit 0 should be 0 (persistence)
        assert_eq!(data[2] & 1, 0, "persistence type bit should be 0");
    }

    #[test]
    fn mixed_persistence_type_bit() {
        let mut sub = SubChunk::new_single(1);
        sub.set_block(0, 0, 0, 2);
        let data = serialize_sub_chunk_disk(&sub);
        assert_eq!(data[2] & 1, 0, "persistence type bit should be 0");
    }

    #[test]
    fn roundtrip_data_2d() {
        let mut biomes = [0u8; 256];
        biomes[0] = 1; // plains
        biomes[100] = 2; // desert
        biomes[255] = 12; // ice plains

        let data = serialize_data_2d(&biomes);
        assert_eq!(data.len(), 768);

        let restored = deserialize_data_2d(&data).unwrap();
        assert_eq!(restored, biomes);
    }

    #[test]
    fn data_2d_too_short() {
        let data = vec![0u8; 100];
        assert!(deserialize_data_2d(&data).is_none());
    }

    #[test]
    fn deserialize_invalid_version() {
        let data = vec![8, 1, 0x00, 0, 0, 0, 1]; // version 8, not 9
        assert!(deserialize_sub_chunk_disk(&data).is_none());
    }

    #[test]
    fn deserialize_too_short() {
        let data = vec![9, 1]; // only 2 bytes
        assert!(deserialize_sub_chunk_disk(&data).is_none());
    }

    // ─── LevelDB integration tests ─────────────────────────────────────

    fn temp_db_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mc_rs_test_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn open_creates_new_db() {
        let path = temp_db_path();
        let result = LevelDbProvider::open(&path);
        assert!(result.is_ok());
        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn load_missing_returns_none() {
        let path = temp_db_path();
        let mut provider = LevelDbProvider::open(&path).unwrap();
        assert!(provider.load_chunk(0, 0).is_none());
        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn save_load_roundtrip() {
        let path = temp_db_path();
        let mut provider = LevelDbProvider::open(&path).unwrap();

        let mut column = ChunkColumn::new_air(5, -3, 10);
        column.biomes[0] = 1;
        column.biomes[100] = 2;
        column.sub_chunks[4].set_block(0, 0, 0, 42);
        column.dirty = true;

        provider.save_chunk(&column).unwrap();
        provider.flush().unwrap();

        let loaded = provider.load_chunk(5, -3).unwrap();
        assert_eq!(loaded.x, 5);
        assert_eq!(loaded.z, -3);
        assert_eq!(loaded.biomes[0], 1);
        assert_eq!(loaded.biomes[100], 2);
        assert_eq!(loaded.sub_chunks[4].get_block(0, 0, 0), 42);
        assert!(!loaded.dirty, "loaded chunks should not be dirty");

        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn multiple_chunks() {
        let path = temp_db_path();
        let mut provider = LevelDbProvider::open(&path).unwrap();

        for i in 0..3 {
            let mut col = ChunkColumn::new_air(i, i * 2, 10);
            col.biomes[0] = i as u8;
            provider.save_chunk(&col).unwrap();
        }
        provider.flush().unwrap();

        for i in 0..3 {
            let loaded = provider.load_chunk(i, i * 2).unwrap();
            assert_eq!(loaded.x, i);
            assert_eq!(loaded.z, i * 2);
            assert_eq!(loaded.biomes[0], i as u8);
        }

        // Non-existent chunk
        assert!(provider.load_chunk(99, 99).is_none());

        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn save_load_roundtrip_nether() {
        let path = temp_db_path();
        let mut provider = LevelDbProvider::open(&path).unwrap();

        let mut column = ChunkColumn::new_air(3, 7, 10);
        column.biomes[0] = 8; // nether wastes
        column.sub_chunks[5].set_block(1, 1, 1, 99);
        column.dirty = true;

        provider.save_chunk_dim(&column, 1).unwrap();
        provider.flush().unwrap();

        // Loading as overworld should return None
        assert!(provider.load_chunk_dim(3, 7, 0).is_none());

        // Loading as nether should work
        let loaded = provider.load_chunk_dim(3, 7, 1).unwrap();
        assert_eq!(loaded.x, 3);
        assert_eq!(loaded.z, 7);
        assert_eq!(loaded.biomes[0], 8);
        assert_eq!(loaded.sub_chunks[5].get_block(1, 1, 1), 99);

        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn save_load_roundtrip_end() {
        let path = temp_db_path();
        let mut provider = LevelDbProvider::open(&path).unwrap();

        let mut column = ChunkColumn::new_air(-1, 2, 10);
        column.sub_chunks[8].set_block(0, 0, 0, 200);
        column.dirty = true;

        provider.save_chunk_dim(&column, 2).unwrap();
        provider.flush().unwrap();

        // End dimension
        let loaded = provider.load_chunk_dim(-1, 2, 2).unwrap();
        assert_eq!(loaded.sub_chunks[8].get_block(0, 0, 0), 200);

        // Same coords, different dimension = not found
        assert!(provider.load_chunk_dim(-1, 2, 0).is_none());
        assert!(provider.load_chunk_dim(-1, 2, 1).is_none());

        std::fs::remove_dir_all(&path).ok();
    }
}
