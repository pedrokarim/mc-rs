use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

use super::chunk_serializer::{self, SubChunk};
use super::block_registry::BLOCKS;
use super::storage::WorldStorage;
use super::terrain_generator;

/// A column of blocks in memory (16x16xN).
pub struct ChunkColumn {
    pub x: i32,
    pub z: i32,
    /// Decompressed sub-chunks with actual runtime IDs.
    pub sub_chunks: Vec<SubChunk>,
    /// Serialized biome data (24 biome sections).
    pub biome_data: Vec<u8>,
    /// Cached network payload, invalidated on block changes.
    pub network_payload: Vec<u8>,
    /// True if network_payload needs to be rebuilt from sub_chunks.
    pub payload_dirty: bool,
    /// Number of sub-chunks to send to the client.
    pub sub_chunk_count: u32,
}

impl ChunkColumn {
    /// Get the network-ready payload, rebuilding if needed.
    pub fn get_network_payload(&mut self) -> &[u8] {
        if self.payload_dirty {
            self.network_payload =
                chunk_serializer::rebuild_network_payload(&self.sub_chunks, &self.biome_data);
            self.payload_dirty = false;
        }
        &self.network_payload
    }
}

fn fallback_biome_data() -> Vec<u8> {
    let biome = chunk_serializer::serialize_biome_section_single(1);
    let mut biome_data = Vec::with_capacity(biome.len() * 24);
    for _ in 0..24 {
        biome_data.extend_from_slice(&biome);
    }
    biome_data
}

/// In-memory chunk cache with LevelDB persistence.
pub struct ChunkCache {
    chunks: HashMap<(i32, i32), ChunkColumn>,
    dirty: HashSet<(i32, i32)>,
    storage: Option<WorldStorage>,
    seed: u64,
    generator: String,
}

impl ChunkCache {
    fn persist_chunk(storage: &mut WorldStorage, cx: i32, cz: i32, chunk: &ChunkColumn) {
        storage.save_chunk_version(cx, cz);
        storage.save_finalization(cx, cz);
        storage.save_biome_data(cx, cz, &chunk.biome_data);

        let kept_sub_chunks = usize::min(chunk.sub_chunks.len(), chunk.sub_chunk_count as usize);
        for (i, sub) in chunk.sub_chunks.iter().take(kept_sub_chunks).enumerate() {
            let serialized = sub.serialize();
            storage.save_sub_chunk(cx, cz, i as u8, &serialized);
        }

        for y_index in kept_sub_chunks..24 {
            storage.delete_sub_chunk(cx, cz, y_index as u8);
        }
    }

    /// Create a new chunk cache backed by LevelDB storage.
    pub fn new(world_dir: &Path, seed: u64, generator: &str) -> Self {
        let storage = match WorldStorage::open(world_dir) {
            Ok(s) => {
                info!("World storage opened at {:?}", world_dir);
                Some(s)
            }
            Err(e) => {
                info!("No world storage (will generate all chunks): {}", e);
                None
            }
        };

        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            storage,
            seed,
            generator: generator.to_lowercase(),
        }
    }

    /// Get a chunk, loading from disk or generating if needed.
    pub fn get_chunk(&mut self, cx: i32, cz: i32) -> &ChunkColumn {
        self.ensure_chunk_loaded(cx, cz);
        self.chunks.get(&(cx, cz)).unwrap()
    }

    /// Get a mutable reference to a chunk.
    pub fn get_chunk_mut(&mut self, cx: i32, cz: i32) -> &mut ChunkColumn {
        self.ensure_chunk_loaded(cx, cz);
        self.chunks.get_mut(&(cx, cz)).unwrap()
    }

    /// Ensure a chunk is loaded in the cache.
    fn ensure_chunk_loaded(&mut self, cx: i32, cz: i32) {
        if self.chunks.contains_key(&(cx, cz)) {
            return;
        }

        // Try loading from storage first
        let loaded = if let Some(ref mut storage) = self.storage {
            if storage.has_chunk(cx, cz) {
                let stored_sub_chunks = storage.load_sub_chunks(cx, cz);
                if !stored_sub_chunks.is_empty() {
                    debug!(
                        "Loaded chunk ({}, {}) from LevelDB ({} sub-chunks)",
                        cx,
                        cz,
                        stored_sub_chunks.len()
                    );
                    // Reconstruct from stored sub-chunk data
                    let mut payload = Vec::with_capacity(4096);
                    for (_y_idx, data) in &stored_sub_chunks {
                        payload.extend_from_slice(data);
                    }
                    let biome_data = storage.load_biome_data(cx, cz).unwrap_or_else(fallback_biome_data);
                    payload.extend_from_slice(&biome_data);
                    payload.push(0); // border blocks

                    let sub_count = stored_sub_chunks.len() as u32;
                    let (sub_chunks, _) =
                        chunk_serializer::parse_chunk_payload(&payload, sub_count, BLOCKS.air);

                    Some(ChunkColumn {
                        x: cx,
                        z: cz,
                        sub_chunks,
                        biome_data,
                        network_payload: payload,
                        payload_dirty: false,
                        sub_chunk_count: sub_count,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let generated = loaded.is_none();
        let column = loaded.unwrap_or_else(|| {
            // Generate new chunk using the configured generator
            let (sub_count, payload) = if self.generator == "flat" {
                super::flat_generator::generate_flat_chunk()
            } else {
                terrain_generator::generate_terrain_chunk(cx, cz, self.seed)
            };

            let (sub_chunks, biome_data) =
                chunk_serializer::parse_chunk_payload(&payload, sub_count, BLOCKS.air);

            ChunkColumn {
                x: cx,
                z: cz,
                sub_chunks,
                biome_data,
                network_payload: payload,
                payload_dirty: false,
                sub_chunk_count: sub_count,
            }
        });

        self.chunks.insert((cx, cz), column);

        if generated && self.storage.is_some() {
            self.dirty.insert((cx, cz));
        }
    }

    /// Get a block's runtime ID at world coordinates.
    pub fn get_block(&mut self, world_x: i32, world_y: i32, world_z: i32) -> u32 {
        let cx = world_x.div_euclid(16);
        let cz = world_z.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let local_y_offset = world_y + 64;
        let local_z = world_z.rem_euclid(16) as usize;

        if !(0..384).contains(&local_y_offset) {
            return BLOCKS.air;
        }

        let sub_idx = local_y_offset as usize / 16;
        let local_y = local_y_offset as usize % 16;

        self.ensure_chunk_loaded(cx, cz);
        let chunk = self.chunks.get(&(cx, cz)).unwrap();

        if sub_idx >= chunk.sub_chunks.len() {
            return BLOCKS.air;
        }

        chunk.sub_chunks[sub_idx].get_block(local_x, local_y, local_z)
    }

    /// Set a block's runtime ID at world coordinates.
    /// Marks the chunk as dirty and invalidates the network payload cache.
    pub fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, runtime_id: u32) {
        let cx = world_x.div_euclid(16);
        let cz = world_z.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let local_y_offset = world_y + 64;
        let local_z = world_z.rem_euclid(16) as usize;

        if !(0..384).contains(&local_y_offset) {
            return;
        }

        let sub_idx = local_y_offset as usize / 16;
        let local_y = local_y_offset as usize % 16;

        self.ensure_chunk_loaded(cx, cz);
        let chunk = self.chunks.get_mut(&(cx, cz)).unwrap();

        // Extend sub_chunks if needed
        while chunk.sub_chunks.len() <= sub_idx {
            chunk.sub_chunks.push(SubChunk::new_air(BLOCKS.air));
        }
        if sub_idx as u32 >= chunk.sub_chunk_count {
            chunk.sub_chunk_count = sub_idx as u32 + 1;
        }

        chunk.sub_chunks[sub_idx].set_block(local_x, local_y, local_z, runtime_id);
        chunk.payload_dirty = true;
        self.dirty.insert((cx, cz));
    }

    /// Save one chunk immediately for durability-sensitive updates such as block edits.
    pub fn save_chunk_now(&mut self, cx: i32, cz: i32) -> bool {
        let Some(storage) = self.storage.as_mut() else {
            return false;
        };
        let Some(chunk) = self.chunks.get(&(cx, cz)) else {
            return false;
        };

        Self::persist_chunk(storage, cx, cz, chunk);
        storage.flush();
        self.dirty.remove(&(cx, cz));
        true
    }

    /// Mark a chunk as modified (needs saving).
    pub fn mark_dirty(&mut self, cx: i32, cz: i32) {
        self.dirty.insert((cx, cz));
    }

    /// Save all dirty chunks to LevelDB.
    pub fn save_dirty(&mut self) {
        if self.dirty.is_empty() {
            return;
        }

        let Some(ref mut storage) = self.storage else {
            self.dirty.clear();
            return;
        };

        let dirty_coords: Vec<(i32, i32)> = self.dirty.drain().collect();
        let mut saved = 0;

        for (cx, cz) in &dirty_coords {
            if let Some(chunk) = self.chunks.get(&(*cx, *cz)) {
                Self::persist_chunk(storage, *cx, *cz, chunk);
                saved += 1;
            }
        }

        storage.flush();
        if saved > 0 {
            info!("Saved {} dirty chunks", saved);
        }
    }

    /// Get the number of cached chunks.
    pub fn cached_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get the number of dirty chunks.
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_world_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("mc-rs-{name}-{unique}"))
    }

    #[test]
    fn test_chunk_persists_block_changes_and_biome_data() {
        let world_dir = temp_world_dir("chunk-cache");
        let original_payload = {
            let mut cache = ChunkCache::new(&world_dir, 42, "normal");
            cache.set_block(0, 80, 0, BLOCKS.pumpkin);
            let payload = {
                let chunk = cache.get_chunk_mut(0, 0);
                chunk.get_network_payload().to_vec()
            };
            assert!(cache.save_chunk_now(0, 0));
            payload
        };

        let mut reloaded = ChunkCache::new(&world_dir, 42, "normal");
        let block = reloaded.get_block(0, 80, 0);
        let reloaded_payload = {
            let chunk = reloaded.get_chunk_mut(0, 0);
            chunk.get_network_payload().to_vec()
        };

        assert_eq!(block, BLOCKS.pumpkin);
        assert_eq!(reloaded_payload, original_payload);

        fs::remove_dir_all(&world_dir).ok();
    }
}
