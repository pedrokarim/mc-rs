use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// LevelDB key tags for Bedrock chunk data.
mod chunk_tag {
    pub const SUBCHUNK: u8 = 0x2F;
    pub const VERSION: u8 = 0x2C;
    pub const FINALIZATION: u8 = 0x36;
    // Custom mc-rs tag storing serialized biome palette data for a chunk column.
    pub const BIOME_DATA: u8 = 0x90;
    // Block entities (NBT little-endian concaténé), une entrée par colonne. Vanilla '1'.
    pub const BLOCK_ENTITIES: u8 = 0x31;
}

/// LevelDB-backed world storage for Bedrock Edition chunks.
pub struct WorldStorage {
    db: rusty_leveldb::DB,
    path: PathBuf,
}

impl WorldStorage {
    /// Open or create a world database at the given path.
    pub fn open(world_dir: &Path) -> Result<Self, String> {
        let db_path = world_dir.join("db");
        std::fs::create_dir_all(&db_path).map_err(|e| format!("Failed to create db dir: {}", e))?;

        let opts = rusty_leveldb::Options::default();
        let db = rusty_leveldb::DB::open(&db_path, opts)
            .map_err(|e| format!("Failed to open LevelDB: {:?}", e))?;

        info!("World storage opened at {:?}", db_path);
        Ok(Self {
            db,
            path: world_dir.to_path_buf(),
        })
    }

    /// Build the chunk index key (réf dragonfly `mcdb/db.go`, format vanilla Bedrock).
    /// Overworld (dim 0) : `[x:i32_le][z:i32_le]` (8 octets, inchangé → pas de migration).
    /// Nether/End        : `[x:i32_le][z:i32_le][dim:u32_le]` (12 octets).
    fn chunk_index(dim: i32, x: i32, z: i32) -> Vec<u8> {
        let mut key = Vec::with_capacity(12);
        key.extend_from_slice(&x.to_le_bytes());
        key.extend_from_slice(&z.to_le_bytes());
        if dim != 0 {
            key.extend_from_slice(&(dim as u32).to_le_bytes());
        }
        key
    }

    /// Build a chunk data key: [index][tag:1]
    fn chunk_key(dim: i32, x: i32, z: i32, tag: u8) -> Vec<u8> {
        let mut key = Self::chunk_index(dim, x, z);
        key.push(tag);
        key
    }

    /// Build a sub-chunk key: [index][0x2F][y_index:1]
    fn sub_chunk_key(dim: i32, x: i32, z: i32, y_index: u8) -> Vec<u8> {
        let mut key = Self::chunk_index(dim, x, z);
        key.push(chunk_tag::SUBCHUNK);
        key.push(y_index);
        key
    }

    /// Check if a chunk exists in the database.
    pub fn has_chunk(&mut self, dim: i32, x: i32, z: i32) -> bool {
        let key = Self::chunk_key(dim, x, z, chunk_tag::VERSION);
        self.db.get(&key).is_some()
    }

    /// Load raw sub-chunk data from the database.
    /// Returns Vec of (y_index, raw_data) pairs.
    pub fn load_sub_chunks(&mut self, dim: i32, x: i32, z: i32) -> Vec<(u8, Vec<u8>)> {
        let count = crate::dimensions::DimensionId::from_id(dim)
            .map(|d| d.sub_chunk_count())
            .unwrap_or(24);
        let mut result = Vec::new();
        for y_idx in 0..count {
            let key = Self::sub_chunk_key(dim, x, z, y_idx);
            if let Some(data) = self.db.get(&key) {
                result.push((y_idx, data));
            }
        }
        result
    }

    /// Load serialized biome data for a chunk column.
    pub fn load_biome_data(&mut self, dim: i32, x: i32, z: i32) -> Option<Vec<u8>> {
        let key = Self::chunk_key(dim, x, z, chunk_tag::BIOME_DATA);
        self.db.get(&key)
    }

    /// Save a sub-chunk's raw data.
    pub fn save_sub_chunk(&mut self, dim: i32, x: i32, z: i32, y_index: u8, data: &[u8]) {
        let key = Self::sub_chunk_key(dim, x, z, y_index);
        self.db.put(&key, data).ok();
    }

    /// Delete a sub-chunk entry.
    pub fn delete_sub_chunk(&mut self, dim: i32, x: i32, z: i32, y_index: u8) {
        let key = Self::sub_chunk_key(dim, x, z, y_index);
        self.db.delete(&key).ok();
    }

    /// Save serialized biome data for a chunk column.
    pub fn save_biome_data(&mut self, dim: i32, x: i32, z: i32, data: &[u8]) {
        let key = Self::chunk_key(dim, x, z, chunk_tag::BIOME_DATA);
        self.db.put(&key, data).ok();
    }

    /// Load the chunk's block entities blob (NBT little-endian concaténé).
    pub fn load_block_entities(&mut self, dim: i32, x: i32, z: i32) -> Option<Vec<u8>> {
        let key = Self::chunk_key(dim, x, z, chunk_tag::BLOCK_ENTITIES);
        self.db.get(&key)
    }

    /// Save the chunk's block entities blob (vide => supprime la clé).
    pub fn save_block_entities(&mut self, dim: i32, x: i32, z: i32, data: &[u8]) {
        let key = Self::chunk_key(dim, x, z, chunk_tag::BLOCK_ENTITIES);
        if data.is_empty() {
            self.db.delete(&key).ok();
        } else {
            self.db.put(&key, data).ok();
        }
    }

    /// Save chunk version marker.
    pub fn save_chunk_version(&mut self, dim: i32, x: i32, z: i32) {
        let key = Self::chunk_key(dim, x, z, chunk_tag::VERSION);
        self.db.put(&key, &[39]).ok(); // version 39
    }

    /// Save finalization state.
    pub fn save_finalization(&mut self, dim: i32, x: i32, z: i32) {
        let key = Self::chunk_key(dim, x, z, chunk_tag::FINALIZATION);
        self.db.put(&key, &[2]).ok(); // 2 = done
    }

    /// Flush all pending writes to disk.
    pub fn flush(&mut self) {
        if let Err(e) = self.db.flush() {
            warn!("LevelDB flush error: {:?}", e);
        } else {
            debug!("World storage flushed");
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overworld_index_is_8_bytes_unchanged() {
        // Overworld must keep the legacy 8-byte key (no migration of existing worlds).
        let idx = WorldStorage::chunk_index(0, 1, -2);
        assert_eq!(idx.len(), 8);
        assert_eq!(&idx[0..4], &1i32.to_le_bytes());
        assert_eq!(&idx[4..8], &(-2i32).to_le_bytes());
    }

    #[test]
    fn nether_end_index_append_dimension() {
        let nether = WorldStorage::chunk_index(1, 3, 4);
        assert_eq!(nether.len(), 12);
        assert_eq!(&nether[8..12], &1u32.to_le_bytes());

        let end = WorldStorage::chunk_index(2, 3, 4);
        assert_eq!(end.len(), 12);
        assert_eq!(&end[8..12], &2u32.to_le_bytes());
    }

    #[test]
    fn sub_chunk_key_layout() {
        let k = WorldStorage::sub_chunk_key(1, 0, 0, 5);
        // [x:4][z:4][dim:4][0x2F][y]
        assert_eq!(k.len(), 14);
        assert_eq!(k[12], chunk_tag::SUBCHUNK);
        assert_eq!(k[13], 5);
    }
}
