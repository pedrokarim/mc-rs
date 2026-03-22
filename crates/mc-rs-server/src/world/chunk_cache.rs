use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

use super::chunk_serializer;
use super::storage::WorldStorage;
use super::terrain_generator;

/// A column of blocks in memory (16x16xN).
pub struct ChunkColumn {
    pub x: i32,
    pub z: i32,
    /// Sub-chunk payloads (already serialized for network).
    /// Index 0 = Y[-64,-49], index 1 = Y[-48,-33], etc.
    pub sub_chunk_count: u32,
    /// Full serialized payload (sub-chunks + biomes + border blocks)
    pub network_payload: Vec<u8>,
}

/// In-memory chunk cache with LevelDB persistence.
pub struct ChunkCache {
    chunks: HashMap<(i32, i32), ChunkColumn>,
    dirty: HashSet<(i32, i32)>,
    storage: Option<WorldStorage>,
    seed: u64,
}

impl ChunkCache {
    /// Create a new chunk cache backed by LevelDB storage.
    pub fn new(world_dir: &Path, seed: u64) -> Self {
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
        }
    }

    /// Get a chunk, loading from disk or generating if needed.
    /// Returns the network-ready payload.
    pub fn get_chunk(&mut self, cx: i32, cz: i32) -> &ChunkColumn {
        if !self.chunks.contains_key(&(cx, cz)) {
            // Try loading from storage first
            let loaded = if let Some(ref mut storage) = self.storage {
                if storage.has_chunk(cx, cz) {
                    // Load from LevelDB
                    let sub_chunks = storage.load_sub_chunks(cx, cz);
                    if !sub_chunks.is_empty() {
                        debug!(
                            "Loaded chunk ({}, {}) from LevelDB ({} sub-chunks)",
                            cx,
                            cz,
                            sub_chunks.len()
                        );
                        // Reconstruct network payload from stored sub-chunks
                        let mut payload = Vec::with_capacity(4096);
                        for (_y_idx, data) in &sub_chunks {
                            payload.extend_from_slice(data);
                        }
                        // Add biome sections
                        let biome = chunk_serializer::serialize_biome_section_single(1);
                        for _ in 0..24 {
                            payload.extend_from_slice(&biome);
                        }
                        payload.push(0); // border blocks
                        Some(ChunkColumn {
                            x: cx,
                            z: cz,
                            sub_chunk_count: sub_chunks.len() as u32,
                            network_payload: payload,
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

            let column = loaded.unwrap_or_else(|| {
                // Generate new chunk
                let (sub_count, payload) =
                    terrain_generator::generate_terrain_chunk(cx, cz, self.seed);
                ChunkColumn {
                    x: cx,
                    z: cz,
                    sub_chunk_count: sub_count,
                    network_payload: payload,
                }
            });

            self.chunks.insert((cx, cz), column);
        }

        self.chunks.get(&(cx, cz)).unwrap()
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
                // Save the network payload as sub-chunk data
                // For now, save the entire payload as sub-chunk 0
                // TODO: properly split back into individual sub-chunks
                storage.save_chunk_version(*cx, *cz);
                storage.save_finalization(*cx, *cz);
                // Save the raw payload as a single blob for sub-chunk 0
                storage.save_sub_chunk(*cx, *cz, 0, &chunk.network_payload);
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
