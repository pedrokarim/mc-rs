use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// LevelDB key tags for Bedrock chunk data.
mod chunk_tag {
    pub const SUBCHUNK: u8 = 0x2F;
    pub const VERSION: u8 = 0x2C;
    pub const FINALIZATION: u8 = 0x36;
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

    /// Build the 8-byte chunk index key: [x:i32_le][z:i32_le]
    fn chunk_index(x: i32, z: i32) -> [u8; 8] {
        let mut key = [0u8; 8];
        key[0..4].copy_from_slice(&x.to_le_bytes());
        key[4..8].copy_from_slice(&z.to_le_bytes());
        key
    }

    /// Build a chunk data key: [index:8][tag:1]
    fn chunk_key(x: i32, z: i32, tag: u8) -> Vec<u8> {
        let idx = Self::chunk_index(x, z);
        let mut key = Vec::with_capacity(9);
        key.extend_from_slice(&idx);
        key.push(tag);
        key
    }

    /// Build a sub-chunk key: [index:8][0x2F][y_index:1]
    fn sub_chunk_key(x: i32, z: i32, y_index: u8) -> Vec<u8> {
        let idx = Self::chunk_index(x, z);
        let mut key = Vec::with_capacity(10);
        key.extend_from_slice(&idx);
        key.push(chunk_tag::SUBCHUNK);
        key.push(y_index);
        key
    }

    /// Check if a chunk exists in the database.
    pub fn has_chunk(&mut self, x: i32, z: i32) -> bool {
        let key = Self::chunk_key(x, z, chunk_tag::VERSION);
        self.db.get(&key).is_some()
    }

    /// Load raw sub-chunk data from the database.
    /// Returns Vec of (y_index, raw_data) pairs.
    pub fn load_sub_chunks(&mut self, x: i32, z: i32) -> Vec<(u8, Vec<u8>)> {
        let mut result = Vec::new();
        // Overworld: sub-chunks 0-23 (y=-64 to 319)
        for y_idx in 0..24u8 {
            let key = Self::sub_chunk_key(x, z, y_idx);
            if let Some(data) = self.db.get(&key) {
                result.push((y_idx, data));
            }
        }
        result
    }

    /// Save a sub-chunk's raw data.
    pub fn save_sub_chunk(&mut self, x: i32, z: i32, y_index: u8, data: &[u8]) {
        let key = Self::sub_chunk_key(x, z, y_index);
        self.db.put(&key, data).ok();
    }

    /// Save chunk version marker.
    pub fn save_chunk_version(&mut self, x: i32, z: i32) {
        let key = Self::chunk_key(x, z, chunk_tag::VERSION);
        self.db.put(&key, &[39]).ok(); // version 39
    }

    /// Save finalization state.
    pub fn save_finalization(&mut self, x: i32, z: i32) {
        let key = Self::chunk_key(x, z, chunk_tag::FINALIZATION);
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
