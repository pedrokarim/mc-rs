use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

use super::chunk_serializer::{self, SubChunk};
use super::flat_generator::block_ids;
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

/// In-memory chunk cache with LevelDB persistence.
pub struct ChunkCache {
    chunks: HashMap<(i32, i32), ChunkColumn>,
    dirty: HashSet<(i32, i32)>,
    storage: Option<WorldStorage>,
    seed: u64,
    generator: String,
}

impl ChunkCache {
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
                    let biome = chunk_serializer::serialize_biome_section_single(1);
                    let mut biome_data = Vec::with_capacity(biome.len() * 24);
                    for _ in 0..24 {
                        biome_data.extend_from_slice(&biome);
                    }
                    payload.extend_from_slice(&biome_data);
                    payload.push(0); // border blocks

                    let sub_count = stored_sub_chunks.len() as u32;
                    let (sub_chunks, _) =
                        chunk_serializer::parse_chunk_payload(&payload, sub_count, block_ids::AIR);

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

        let column = loaded.unwrap_or_else(|| {
            // Generate new chunk using the configured generator
            let (sub_count, payload) = if self.generator == "flat" {
                super::flat_generator::generate_flat_chunk()
            } else {
                terrain_generator::generate_terrain_chunk(cx, cz, self.seed)
            };

            let (sub_chunks, biome_data) =
                chunk_serializer::parse_chunk_payload(&payload, sub_count, block_ids::AIR);

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
    }

    /// Get a block's runtime ID at world coordinates.
    pub fn get_block(&mut self, world_x: i32, world_y: i32, world_z: i32) -> u32 {
        let cx = world_x.div_euclid(16);
        let cz = world_z.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let local_y_offset = world_y + 64;
        let local_z = world_z.rem_euclid(16) as usize;

        if !(0..384).contains(&local_y_offset) {
            return block_ids::AIR;
        }

        let sub_idx = local_y_offset as usize / 16;
        let local_y = local_y_offset as usize % 16;

        self.ensure_chunk_loaded(cx, cz);
        let chunk = self.chunks.get(&(cx, cz)).unwrap();

        if sub_idx >= chunk.sub_chunks.len() {
            return block_ids::AIR;
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
            chunk.sub_chunks.push(SubChunk::new_air(block_ids::AIR));
        }
        if sub_idx as u32 >= chunk.sub_chunk_count {
            chunk.sub_chunk_count = sub_idx as u32 + 1;
        }

        chunk.sub_chunks[sub_idx].set_block(local_x, local_y, local_z, runtime_id);
        chunk.payload_dirty = true;
        self.dirty.insert((cx, cz));
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
                storage.save_chunk_version(*cx, *cz);
                storage.save_finalization(*cx, *cz);
                // Save each sub-chunk individually
                for (i, sub) in chunk.sub_chunks.iter().enumerate() {
                    let serialized = sub.serialize();
                    storage.save_sub_chunk(*cx, *cz, i as u8, &serialized);
                }
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
