//! `Worlds` — l'ensemble des 3 dimensions (overworld / nether / end).
//!
//! Les 3 [`ChunkCache`] partagent **une seule** base LevelDB (format vanilla Bedrock,
//! la dimension est encodée dans la clé — cf. `storage.rs` + `26-DIMENSIONS-CADRAGE.md`).
//! Un seul lock file LevelDB existe, d'où le `SharedStorage` (`Arc<Mutex<WorldStorage>>`)
//! partagé entre les caches.
//!
//! Câblage runtime (DIM-C) : `main()` détient un `Worlds`. Chaque `Connection` réassigne
//! son `chunk_cache` vers `worlds.get(current_dimension)` lors d'un transfert (DIM-D), de
//! sorte que tout le code chunk per-connexion opère automatiquement sur le bon monde.

use std::path::Path;
use std::sync::{Arc, Mutex};

use super::chunk_cache::{ChunkCache, SharedStorage};

/// Identifiant de générateur pour chaque dimension. Overworld vient de la config ;
/// nether/end sont fixes (générateurs implémentés en DIM-E / DIM-G — placeholder flat
/// d'ici là, cf. `ChunkCache::ensure_chunk_loaded`).
const NETHER_GENERATOR: &str = "nether";
const END_GENERATOR: &str = "end";

#[derive(Clone)]
pub struct Worlds {
    overworld: Arc<Mutex<ChunkCache>>,
    nether: Arc<Mutex<ChunkCache>>,
    end: Arc<Mutex<ChunkCache>>,
}

impl Worlds {
    /// Ouvre la LevelDB partagée une fois et crée les 3 caches de dimension.
    pub fn new(world_dir: &Path, seed: u64, overworld_generator: &str) -> Self {
        let storage = ChunkCache::open_shared_storage(world_dir);
        Self::from_shared_storage(storage, seed, overworld_generator)
    }

    /// Construit les 3 caches autour d'un storage déjà ouvert (ou `None` = génératif pur).
    pub fn from_shared_storage(
        storage: Option<SharedStorage>,
        seed: u64,
        overworld_generator: &str,
    ) -> Self {
        let mk = |generator: &str, dim: i32| {
            Arc::new(Mutex::new(ChunkCache::with_shared_storage(
                storage.clone(),
                seed,
                generator,
                dim,
            )))
        };
        Self {
            overworld: mk(overworld_generator, 0),
            nether: mk(NETHER_GENERATOR, 1),
            end: mk(END_GENERATOR, 2),
        }
    }

    /// Le cache overworld (dimension primaire — la game loop l'utilise par défaut).
    pub fn overworld(&self) -> &Arc<Mutex<ChunkCache>> {
        &self.overworld
    }

    /// Cache d'une dimension par id numérique. Les ids inconnus retombent sur l'overworld.
    pub fn get(&self, dim: i32) -> &Arc<Mutex<ChunkCache>> {
        match dim {
            1 => &self.nether,
            2 => &self.end,
            _ => &self.overworld,
        }
    }

    /// Clone l'`Arc` du cache d'une dimension (pour réassigner `Connection.chunk_cache`).
    pub fn arc(&self, dim: i32) -> Arc<Mutex<ChunkCache>> {
        Arc::clone(self.get(dim))
    }

    /// Sauve les chunks modifiés de toutes les dimensions (autosave + shutdown).
    pub fn save_all_dirty(&self) {
        for cache in [&self.overworld, &self.nether, &self.end] {
            if let Ok(mut c) = cache.lock() {
                c.save_dirty();
            }
        }
    }

    /// Itère les 3 caches (utilitaire pour des opérations cross-dimension).
    pub fn all(&self) -> [&Arc<Mutex<ChunkCache>>; 3] {
        [&self.overworld, &self.nether, &self.end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_routes_by_dimension_id() {
        // Caches purement génératifs (pas de storage, pas d'I/O disque).
        let worlds = Worlds::from_shared_storage(None, 1, "flat");
        assert_eq!(worlds.get(0).lock().unwrap().dimension(), 0);
        assert_eq!(worlds.get(1).lock().unwrap().dimension(), 1);
        assert_eq!(worlds.get(2).lock().unwrap().dimension(), 2);
        // Id inconnu → overworld.
        assert_eq!(worlds.get(99).lock().unwrap().dimension(), 0);
    }
}
