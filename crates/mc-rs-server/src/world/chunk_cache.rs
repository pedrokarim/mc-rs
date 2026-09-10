use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use super::block_registry::BLOCKS;
use super::chunk_serializer::{self, SubChunk};
use super::storage::WorldStorage;
use super::terrain_generator;

/// Shared LevelDB handle. The 3 dimension caches (overworld/nether/end) all point
/// to the SAME database (vanilla on-disk format, dimension encoded in the key).
/// A single LevelDB lock file exists, so sharing via `Arc<Mutex>` is mandatory.
pub type SharedStorage = Arc<Mutex<WorldStorage>>;

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
    /// Cache du LevelChunk packet déjà compressé en Zlib (shared batch),
    /// prêt à être passé à `prepare_for_send` (encryption per-player).
    /// Évite N compressions Zlib quand N joueurs chargent le même chunk.
    /// Invalidé à chaque set_block en même temps que `payload_dirty`.
    pub cached_zlib_batch: Option<Vec<u8>>,
    /// Block entities de la colonne (coffres, spawners, panneaux…). Inclus dans le
    /// payload réseau (après le border byte) et persistés en LevelDB (clé '1').
    pub block_entities: Vec<crate::block_entities::BlockEntity>,
}

impl ChunkColumn {
    /// Get the network-ready payload, rebuilding if needed.
    pub fn get_network_payload(&mut self) -> &[u8] {
        if self.payload_dirty {
            self.network_payload = chunk_serializer::rebuild_network_payload(
                &self.sub_chunks,
                &self.biome_data,
                &self.block_entities,
            );
            self.payload_dirty = false;
            // Le payload a changé → invalider la batch Zlib cachée.
            self.cached_zlib_batch = None;
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

/// Biome (identifiant vanilla `"minecraft:..."`) du worldgen moderne « noise »
/// à une position monde. Reproduit EXACTEMENT la passe de génération du chunk
/// (`ClimateSampler::sample(x>>2, y>>2, z>>2)` → `BiomeParameters::find`), via
/// les seules API **publiques** de `world::worldgen` — donc le biome est
/// identique à celui que voit le joueur, sans dépendre des internes du worldgen
/// (édités par ailleurs). La table climatique ne dépend pas du seed → static
/// globale ; le router de densité en dépend → caché et reconstruit au
/// changement de seed (construction coûteuse).
///
/// Le worldgen nomme les biomes « à la Java » (`badlands`, `snowy_plains`…),
/// alors que la table de tags `biomes_vanilla` utilise les noms Bedrock legacy
/// (`mesa`, `ice_plains`…). Le pont entre les deux est l'**ID réseau Bedrock**
/// (`bedrock_ids`, mapping Geyser validé) → `biome::biome_identifier`.
fn noise_biome_identifier(seed: u64, x: i32, y: i32, z: i32) -> &'static str {
    use crate::world::worldgen::climate::{ClimateSampler, OverworldBiomes};
    use crate::world::worldgen::density::{self, NoiseRouter};
    use std::sync::{LazyLock, Mutex};

    static BIOMES: LazyLock<OverworldBiomes> =
        LazyLock::new(crate::world::worldgen::climate::load_overworld);
    static ROUTER: LazyLock<Mutex<(u64, Option<NoiseRouter>)>> =
        LazyLock::new(|| Mutex::new((0, None)));

    let bedrock_id = {
        let mut guard = ROUTER.lock().unwrap();
        if guard.1.is_none() || guard.0 != seed {
            *guard = (seed, Some(density::build_overworld(seed)));
        }
        let climate = ClimateSampler::from_router(guard.1.as_ref().unwrap());
        let idx = BIOMES.params.find(&climate.sample(x >> 2, y >> 2, z >> 2));
        BIOMES.bedrock_ids[idx as usize]
    };
    // Pont ID Bedrock → identifiant legacy connu de `biomes_vanilla`. Les
    // biomes non couverts (rares, ex. certains caves/1.21) retombent sur
    // plains : pas de tag spécifique, mais identifiant valide.
    super::biome::biome_identifier(bedrock_id).unwrap_or("minecraft:plains")
}

/// In-memory chunk cache with LevelDB persistence.
pub struct ChunkCache {
    chunks: HashMap<(i32, i32), ChunkColumn>,
    dirty: HashSet<(i32, i32)>,
    storage: Option<SharedStorage>,
    seed: u64,
    generator: String,
    /// Numeric dimension id (0=overworld, 1=nether, 2=end) — used for LevelDB keys
    /// and the per-chunk `dimension_id` sent on the wire.
    dimension: i32,
}

impl ChunkCache {
    fn persist_chunk(storage: &mut WorldStorage, dim: i32, cx: i32, cz: i32, chunk: &ChunkColumn) {
        storage.save_chunk_version(dim, cx, cz);
        storage.save_finalization(dim, cx, cz);
        storage.save_biome_data(dim, cx, cz, &chunk.biome_data);
        storage.save_block_entities(
            dim,
            cx,
            cz,
            &crate::block_entities::serialize_le(&chunk.block_entities),
        );

        let kept_sub_chunks = usize::min(chunk.sub_chunks.len(), chunk.sub_chunk_count as usize);
        for (i, sub) in chunk.sub_chunks.iter().take(kept_sub_chunks).enumerate() {
            let serialized = sub.serialize();
            storage.save_sub_chunk(dim, cx, cz, i as u8, &serialized);
        }

        let max_sub_chunks = crate::dimensions::DimensionId::from_id(dim)
            .map(|d| d.sub_chunk_count() as usize)
            .unwrap_or(24);
        for y_index in kept_sub_chunks..max_sub_chunks {
            storage.delete_sub_chunk(dim, cx, cz, y_index as u8);
        }
    }

    /// Open the shared LevelDB at `world_dir`, or `None` if it can't be opened
    /// (the server then runs purely generative, no persistence).
    pub fn open_shared_storage(world_dir: &Path) -> Option<SharedStorage> {
        match WorldStorage::open(world_dir) {
            Ok(s) => {
                info!("World storage opened at {:?}", world_dir);
                Some(Arc::new(Mutex::new(s)))
            }
            Err(e) => {
                info!("No world storage (will generate all chunks): {}", e);
                None
            }
        }
    }

    /// Create a new overworld chunk cache that owns a freshly opened LevelDB.
    /// Convenience for the single-world path; multi-dimension setups use
    /// [`Self::with_shared_storage`] so all 3 caches share one database.
    pub fn new(world_dir: &Path, seed: u64, generator: &str) -> Self {
        let storage = Self::open_shared_storage(world_dir);
        Self::with_shared_storage(storage, seed, generator, 0)
    }

    /// Create a cache for a given dimension, backed by a shared LevelDB handle.
    pub fn with_shared_storage(
        storage: Option<SharedStorage>,
        seed: u64,
        generator: &str,
        dimension: i32,
    ) -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            storage,
            seed,
            generator: generator.to_lowercase(),
            dimension,
        }
    }

    /// Set the dimension this cache belongs to (builder style).
    pub fn with_dimension(mut self, dimension: i32) -> Self {
        self.dimension = dimension;
        self
    }

    /// The numeric dimension id of this cache.
    pub fn dimension(&self) -> i32 {
        self.dimension
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
        let dim = self.dimension;
        let loaded = if let Some(ref storage_arc) = self.storage {
            let mut storage = storage_arc.lock().unwrap();
            if storage.has_chunk(dim, cx, cz) {
                let stored_sub_chunks = storage.load_sub_chunks(dim, cx, cz);
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
                    let mut biome_data = storage
                        .load_biome_data(dim, cx, cz)
                        .unwrap_or_else(fallback_biome_data);
                    // Biomes sauvegardés avec l'ancien format (sections de 64
                    // cases) : convertis une fois et réécrits sur disque.
                    if let Some(upgraded) = chunk_serializer::upgrade_legacy_biome_data(&biome_data)
                    {
                        storage.save_biome_data(dim, cx, cz, &upgraded);
                        biome_data = upgraded;
                    }
                    payload.extend_from_slice(&biome_data);
                    payload.push(0); // border blocks

                    let block_entities = storage
                        .load_block_entities(dim, cx, cz)
                        .map(|d| crate::block_entities::parse_le(&d))
                        .unwrap_or_default();

                    let sub_count = stored_sub_chunks.len() as u32;
                    let (sub_chunks, _) =
                        chunk_serializer::parse_chunk_payload(&payload, sub_count, BLOCKS.air);

                    Some(ChunkColumn {
                        x: cx,
                        z: cz,
                        sub_chunks,
                        biome_data,
                        network_payload: payload,
                        // Reconstruire le payload avec les block entities si présentes.
                        payload_dirty: !block_entities.is_empty(),
                        sub_chunk_count: sub_count,
                        cached_zlib_batch: None,
                        block_entities,
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
            // Generate new chunk using the configured generator. Seul le Nether émet
            // des block entities (forteresse) pour l'instant.
            let (sub_count, payload, block_entities) = match self.generator.as_str() {
                "flat" => {
                    let (c, p) = super::flat_generator::generate_flat_chunk();
                    (c, p, Vec::new())
                }
                "noise" => {
                    let (c, p) =
                        super::worldgen::noise_chunk::generate_noise_chunk(cx, cz, self.seed);
                    (c, p, Vec::new())
                }
                // Nether : terrain + décoration + forteresse (avec coffres/spawners).
                "nether" => super::nether_generator::generate_nether_chunk(cx, cz, self.seed),
                // End : île centrale flottante + piliers d'obsidienne + vide.
                "end" => {
                    let (c, p) = super::end_generator::generate_end_chunk(cx, cz, self.seed);
                    (c, p, Vec::new())
                }
                _ => {
                    let (c, p) = terrain_generator::generate_terrain_chunk(cx, cz, self.seed);
                    (c, p, Vec::new())
                }
            };

            let (sub_chunks, biome_data) =
                chunk_serializer::parse_chunk_payload(&payload, sub_count, BLOCKS.air);

            ChunkColumn {
                x: cx,
                z: cz,
                sub_chunks,
                biome_data,
                network_payload: payload,
                // Reconstruire le payload avec les block entities si présentes.
                payload_dirty: !block_entities.is_empty(),
                sub_chunk_count: sub_count,
                cached_zlib_batch: None,
                block_entities,
            }
        });

        self.chunks.insert((cx, cz), column);

        if generated && self.storage.is_some() {
            self.dirty.insert((cx, cz));
        }
    }

    /// Bornes Y (min_y, hauteur) de la dimension de ce cache. Les sub-chunks sont
    /// indexés depuis `min_y` : overworld -64 (24 sections), nether 0 (8), end 0 (16).
    fn y_bounds(&self) -> (i32, i32) {
        crate::dimensions::DimensionId::from_id(self.dimension)
            .map(|d| (d.min_y(), d.max_y() - d.min_y()))
            .unwrap_or((-64, 384))
    }

    /// Get a block's runtime ID at world coordinates.
    pub fn get_block(&mut self, world_x: i32, world_y: i32, world_z: i32) -> u32 {
        let cx = world_x.div_euclid(16);
        let cz = world_z.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let (min_y, height) = self.y_bounds();
        let local_y_offset = world_y - min_y;
        let local_z = world_z.rem_euclid(16) as usize;

        if !(0..height).contains(&local_y_offset) {
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

    /// Identifiant de biome (`"minecraft:forest"`, …) au point monde, calculé
    /// selon le **générateur actif** pour correspondre au biome réellement
    /// généré et visible par le joueur (comme Bedrock, qui teste le biome du
    /// bloc de spawn). Pour les tags vanilla, voir `crate::biomes_vanilla`.
    /// Utilisé par le spawner pour le filtrage par biome.
    pub fn biome_identifier_at(&self, world_x: i32, world_y: i32, world_z: i32) -> &'static str {
        match self.generator.as_str() {
            // Worldgen moderne : biome climatique (mêmes API publiques que la
            // génération du chunk → biome identique à celui que voit le joueur).
            "noise" => noise_biome_identifier(self.seed, world_x, world_y, world_z),
            // Flat : tout en plaines.
            "flat" => "minecraft:plains",
            // Dimensions (placeholder jusqu'à DIM-E / DIM-G).
            "nether" => "minecraft:nether_wastes",
            "end" => "minecraft:the_end",
            // Générateur Simplex hérité.
            _ => {
                let id = terrain_generator::get_biome_at(world_x, world_z, self.seed);
                super::biome::biome_identifier(id).unwrap_or("minecraft:plains")
            }
        }
    }

    /// Graine du monde.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Set a block's runtime ID at world coordinates.
    /// Marks the chunk as dirty and invalidates the network payload cache.
    pub fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, runtime_id: u32) {
        let cx = world_x.div_euclid(16);
        let cz = world_z.div_euclid(16);
        let local_x = world_x.rem_euclid(16) as usize;
        let (min_y, height) = self.y_bounds();
        let local_y_offset = world_y - min_y;
        let local_z = world_z.rem_euclid(16) as usize;

        if !(0..height).contains(&local_y_offset) {
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
        chunk.cached_zlib_batch = None;
        self.dirty.insert((cx, cz));
    }

    /// Récupère et **consomme** le butin du block entity (coffre) à `(wx,wy,wz)` :
    /// renvoie ses items et les vide (vanilla : loot consommé à la 1re ouverture).
    /// Le chunk est marqué dirty pour persister le coffre vidé. `None` si aucun butin.
    pub fn take_block_entity_loot(
        &mut self,
        wx: i32,
        wy: i32,
        wz: i32,
    ) -> Option<Vec<mc_rs_proto::packets::player::ItemStack>> {
        use mc_rs_proto::packets::player::ItemStack;
        let cx = wx.div_euclid(16);
        let cz = wz.div_euclid(16);
        self.ensure_chunk_loaded(cx, cz);
        let chunk = self.chunks.get_mut(&(cx, cz))?;
        let be = chunk
            .block_entities
            .iter_mut()
            .find(|b| b.position == [wx, wy, wz])?;
        if be.items.iter().all(|i| i.is_air()) {
            return None;
        }
        let n = be.items.len();
        let items = std::mem::replace(&mut be.items, vec![ItemStack::AIR; n]);
        self.dirty.insert((cx, cz));
        Some(items)
    }

    /// Positions des spawners de mobs (block entities `MobSpawner`) dans les chunks
    /// **chargés**, avec leur entité (`minecraft:blaze`…). Pour le tick des spawners.
    pub fn mob_spawner_positions(&self) -> Vec<([i32; 3], String)> {
        let mut out = Vec::new();
        for chunk in self.chunks.values() {
            for be in &chunk.block_entities {
                if be.kind == crate::block_entities::BlockEntityKind::MobSpawner {
                    if let Some(entity) = &be.spawn_entity {
                        out.push((be.position, entity.clone()));
                    }
                }
            }
        }
        out
    }

    /// Save one chunk immediately for durability-sensitive updates such as block edits.
    pub fn save_chunk_now(&mut self, cx: i32, cz: i32) -> bool {
        let dim = self.dimension;
        let Some(storage_arc) = self.storage.as_ref() else {
            return false;
        };
        let Some(chunk) = self.chunks.get(&(cx, cz)) else {
            return false;
        };

        {
            let mut storage = storage_arc.lock().unwrap();
            Self::persist_chunk(&mut storage, dim, cx, cz, chunk);
            storage.flush();
        }
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

        let dim = self.dimension;
        let Some(storage_arc) = self.storage.clone() else {
            self.dirty.clear();
            return;
        };

        let dirty_coords: Vec<(i32, i32)> = self.dirty.drain().collect();
        let mut saved = 0;

        let mut storage = storage_arc.lock().unwrap();
        for (cx, cz) in &dirty_coords {
            if let Some(chunk) = self.chunks.get(&(*cx, *cz)) {
                Self::persist_chunk(&mut storage, dim, *cx, *cz, chunk);
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
    fn noise_biome_identifier_is_known_to_vanilla_data() {
        // Le biome lu pour le spawn (générateur "noise") doit être un
        // identifiant vanilla connu de `biomes_vanilla` — sinon le filtrage par
        // biome (`biome_allows`) lirait des tags vides et serait inopérant.
        let cache = ChunkCache::new(&temp_world_dir("biome-noise"), 12345, "noise");
        let mut found_tagged = false;
        for (x, z) in [(0, 0), (512, -512), (-2000, 3000), (10000, 10000)] {
            let id = cache.biome_identifier_at(x, 72, z);
            assert!(
                id.starts_with("minecraft:"),
                "identifiant de biome inattendu: {id}"
            );
            let data = crate::biomes_vanilla::for_biome(id)
                .unwrap_or_else(|| panic!("biome {id} inconnu de biomes_vanilla"));
            if !data.tags.is_empty() {
                found_tagged = true;
            }
        }
        assert!(found_tagged, "au moins un biome doit porter des tags");
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

    /// Régression : `get_block`/`set_block` doivent utiliser le `min_y` de la
    /// dimension. Un bug historique offsetait toujours de +64 (overworld), ce qui
    /// désalignait les lectures Nether/End de 4 sub-chunks (joueur TP dans le vide).
    #[test]
    fn block_access_is_dimension_y_aligned() {
        // Nether (min_y 0, hauteur 128) : pas d'offset overworld.
        let mut nether = ChunkCache::with_shared_storage(None, 1, "nether", 1);
        nether.set_block(3, 70, 3, BLOCKS.bedrock);
        assert_eq!(nether.get_block(3, 70, 3), BLOCKS.bedrock);
        // y=70 ≠ y=6 : si le bug +64 revenait, set@70 écrirait là où get@6 lit.
        assert_ne!(nether.get_block(3, 6, 3), BLOCKS.bedrock);
        // Hors bornes Nether (≥128) → air.
        assert_eq!(nether.get_block(3, 200, 3), BLOCKS.air);

        // End (min_y 0, hauteur 256) : l'île centrale générée est solide vers y56,
        // et le vide (y0) est de l'air — exactement ce que safe_arrival doit voir.
        let mut end = ChunkCache::with_shared_storage(None, 1, "end", 2);
        assert_eq!(end.get_block(0, 56, 0), BLOCKS.get("minecraft:end_stone"));
        assert_eq!(end.get_block(0, 5, 0), BLOCKS.air);

        // Overworld : comportement inchangé (offset -64).
        let mut ow = ChunkCache::with_shared_storage(None, 1, "flat", 0);
        ow.set_block(1, 200, 1, BLOCKS.pumpkin);
        assert_eq!(ow.get_block(1, 200, 1), BLOCKS.pumpkin);
    }
}
