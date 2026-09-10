//! Migration des runtime IDs de blocs sauvegardés lors d'un changement de palette.
//!
//! Les sous-chunks sont persistés au format réseau (palette de runtime IDs,
//! cf. `storage.rs`) : quand la palette change (nouvelle version Bedrock), un
//! monde existant doit être réécrit, sinon chaque bloc change de signification.
//!
//! Chaque monde porte l'empreinte de la palette avec laquelle il a été écrit
//! (clé LevelDB [`PALETTE_MARKER_KEY`]). Les tables sont générées par
//! `cargo run -p mc-rs-server --bin generate_data_migrations`.

use super::block_registry_data::PALETTE_FINGERPRINT;
use super::palette_migration_data::PALETTE_MIGRATIONS;

/// Clé LevelDB contenant l'empreinte (u64 LE) de la palette du monde.
/// Hors de l'espace des clés de chunk (9 à 14 octets binaires).
pub const PALETTE_MARKER_KEY: &[u8] = b"mcrs:palette_fingerprint";

/// Libellé de la palette des mondes sans marqueur : le marqueur a été introduit
/// avec la palette 1.26.40, et la palette 1.26.20 était en service depuis le
/// commit `48c9595` (2026-05-19).
pub const UNMARKED_WORLD_LABEL: &str = "1.26.20";

/// Table de conversion d'une palette sauvegardée vers la palette courante.
pub struct PaletteMigration {
    pub label: &'static str,
    pub from_fingerprint: u64,
    pub to_fingerprint: u64,
    /// `table[ancien_id] = nouvel_id`.
    pub table: &'static [u32],
}

impl PaletteMigration {
    /// Convertit un runtime ID de l'ancienne palette (air si hors table).
    pub fn remap(&self, legacy_id: u32) -> u32 {
        self.table
            .get(legacy_id as usize)
            .copied()
            .unwrap_or_else(|| super::block_registry::BLOCKS.air)
    }
}

/// Migration depuis la palette d'empreinte `from` vers la palette courante.
pub fn migration_from(from: u64) -> Option<&'static PaletteMigration> {
    PALETTE_MIGRATIONS
        .iter()
        .find(|m| m.from_fingerprint == from && m.to_fingerprint == PALETTE_FINGERPRINT)
}

/// Migration des mondes sans marqueur (voir [`UNMARKED_WORLD_LABEL`]).
pub fn unmarked_world_migration() -> Option<&'static PaletteMigration> {
    PALETTE_MIGRATIONS
        .iter()
        .find(|m| m.label == UNMARKED_WORLD_LABEL && m.to_fingerprint == PALETTE_FINGERPRINT)
}

/// Empreinte de la palette courante (celle compilée dans le binaire).
pub fn current_fingerprint() -> u64 {
    PALETTE_FINGERPRINT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry_data::PALETTE_STATE_COUNT;

    #[test]
    fn unmarked_migration_targets_current_palette() {
        let m = unmarked_world_migration().expect("migration 1.26.20 → palette courante");
        assert_eq!(m.table.len(), 16_899);
        assert!(m.table.iter().all(|&id| id < PALETTE_STATE_COUNT));
    }

    #[test]
    fn migration_keeps_block_names() {
        // Les noms de blocs ne changent pas entre 1.26.20 et 1.26.40 : l'air
        // hérité doit tomber sur l'air courant.
        let m = unmarked_world_migration().unwrap();
        let legacy_air = 13_080; // runtime ID de minecraft:air dans la palette 1.26.20
        assert_eq!(
            m.remap(legacy_air),
            crate::world::block_registry::BLOCKS.air
        );
    }
}
