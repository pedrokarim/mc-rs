//! Paquets de pure donnée repris tels quels de BDS 1.26.52.3
//! (`data/bds_packets/`, extraits de sa capture par
//! `scripts/import_bds_packets.py` ; provenance et SHA-256 dans
//! `manifest.json` ; méthode : `dev-docs/34-BDS-RETRO-INGENIERIE.md`).
//!
//! Ils ne contiennent que des définitions du jeu, sans identifiant propre au
//! serveur. Le client 1.26.50+ exige JigsawStructureData : sans lui, il se
//! déconnecte avec `MissingStructureData` (écran « Boat »,
//! `InitialConnection-148`). BDS les envoie juste avant StartGame.

/// Structures jigsaw du jeu (camps abandonnés, villages…), NBT réseau.
pub const JIGSAW_STRUCTURE_DATA: &[u8] =
    include_bytes!("../data/bds_packets/jigsaw_structure_data.bin");

/// Table des formes de blocs vanilla (dalles, murets, escaliers…).
pub const VOXEL_SHAPES: &[u8] = include_bytes!("../data/bds_packets/voxel_shapes.bin");
