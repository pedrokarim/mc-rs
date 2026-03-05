//! Behavior pack loader — scans a directory and loads all packs.

use std::collections::HashMap;
use std::path::Path;

use tracing::{info, warn};

use crate::block::{BlockFile, ParsedBlock};
use crate::entity::{EntityFile, ParsedEntity};
use crate::item::{ItemFile, ParsedItem};
use crate::loot_table::LootTableFile;
use crate::manifest::BehaviorPackManifest;
use crate::recipe::RecipeFile;

/// A fully loaded behavior pack.
#[derive(Debug, Clone)]
pub struct LoadedBehaviorPack {
    pub manifest: BehaviorPackManifest,
    pub entities: Vec<ParsedEntity>,
    pub items: Vec<ParsedItem>,
    pub blocks: Vec<ParsedBlock>,
    pub recipes: Vec<RecipeFile>,
    pub loot_tables: HashMap<String, LootTableFile>,
    /// Raw .mcpack bytes for client transfer (if available).
    pub pack_bytes: Option<Vec<u8>>,
    pub pack_size: u64,
}

/// Load a single behavior pack from a directory.
pub fn load_behavior_pack(path: &Path) -> Result<LoadedBehaviorPack, String> {
    let manifest_path = path.join("manifest.json");
    let manifest_str =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("read manifest.json: {e}"))?;
    let manifest =
        BehaviorPackManifest::parse(&manifest_str).map_err(|e| format!("parse manifest: {e}"))?;

    let entities: Vec<ParsedEntity> = load_and_parse::<EntityFile>(path, "entities")
        .into_iter()
        .map(|f| f.extract())
        .collect();

    let items: Vec<ParsedItem> = load_and_parse::<ItemFile>(path, "items")
        .into_iter()
        .map(|f| f.extract())
        .collect();

    let blocks: Vec<ParsedBlock> = load_and_parse::<BlockFile>(path, "blocks")
        .into_iter()
        .map(|f| f.extract())
        .collect();

    let recipes = load_and_parse::<RecipeFile>(path, "recipes");

    let loot_tables = load_loot_tables(path);

    // Check for a pre-zipped .mcpack file alongside the directory.
    let pack_name = path.file_name().unwrap_or_default().to_string_lossy();
    let mcpack_path = path.with_extension("mcpack");
    let (pack_bytes, pack_size) = if mcpack_path.exists() {
        match std::fs::read(&mcpack_path) {
            Ok(bytes) => {
                let size = bytes.len() as u64;
                (Some(bytes), size)
            }
            Err(e) => {
                warn!("Failed to read {}: {e}", mcpack_path.display());
                (None, 0)
            }
        }
    } else {
        (None, 0)
    };

    info!(
        "Loaded behavior pack '{}' v{} ({} entities, {} items, {} blocks, {} recipes, {} loot tables{})",
        manifest.header.name,
        manifest.version_string(),
        entities.len(),
        items.len(),
        blocks.len(),
        recipes.len(),
        loot_tables.len(),
        if pack_bytes.is_some() {
            format!(", {pack_name}.mcpack for transfer")
        } else {
            String::new()
        }
    );

    Ok(LoadedBehaviorPack {
        manifest,
        entities,
        items,
        blocks,
        recipes,
        loot_tables,
        pack_bytes,
        pack_size,
    })
}

/// Scan a directory for behavior packs and load all of them.
pub fn load_all_packs(packs_dir: &Path) -> Vec<LoadedBehaviorPack> {
    let mut packs = Vec::new();

    let entries = match std::fs::read_dir(packs_dir) {
        Ok(e) => e,
        Err(_) => return packs,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("manifest.json").exists() {
            match load_behavior_pack(&path) {
                Ok(pack) => packs.push(pack),
                Err(e) => warn!("Failed to load behavior pack at {}: {e}", path.display()),
            }
        }
    }

    if !packs.is_empty() {
        info!(
            "Loaded {} behavior pack(s) from {}",
            packs.len(),
            packs_dir.display()
        );
    }

    packs
}

/// Load all JSON files from a subdirectory and deserialize them.
fn load_and_parse<T: serde::de::DeserializeOwned>(pack_root: &Path, subdir: &str) -> Vec<T> {
    let dir = pack_root.join(subdir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<T>(&content) {
                    Ok(parsed) => results.push(parsed),
                    Err(e) => warn!("Failed to parse {}: {e}", path.display()),
                },
                Err(e) => warn!("Failed to read {}: {e}", path.display()),
            }
        }
    }
    results
}

/// Load loot tables keyed by their relative path.
fn load_loot_tables(pack_root: &Path) -> HashMap<String, LootTableFile> {
    let dir = pack_root.join("loot_tables");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return HashMap::new(),
    };

    let mut tables = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let key = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            match std::fs::read_to_string(&path) {
                Ok(content) => match LootTableFile::parse_json(&content) {
                    Ok(table) => {
                        tables.insert(key, table);
                    }
                    Err(e) => warn!("Failed to parse loot table {}: {e}", path.display()),
                },
                Err(e) => warn!("Failed to read {}: {e}", path.display()),
            }
        }
    }
    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_empty_directory() {
        let dir = std::env::temp_dir().join("mc_rs_bp_test_empty");
        let _ = fs::create_dir_all(&dir);
        let packs = load_all_packs(&dir);
        assert!(packs.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_pack_with_manifest() {
        let dir = std::env::temp_dir().join("mc_rs_bp_test_pack");
        let pack_dir = dir.join("test_pack");
        let _ = fs::create_dir_all(&pack_dir);

        fs::write(
            pack_dir.join("manifest.json"),
            r#"{
                "format_version": 2,
                "header": {
                    "name": "Test",
                    "uuid": "00000000-0000-0000-0000-000000000001",
                    "version": [1, 0, 0]
                },
                "modules": [{"type": "data", "uuid": "00000000-0000-0000-0000-000000000002", "version": [1, 0, 0]}]
            }"#,
        )
        .unwrap();

        // Create an entity
        let entities_dir = pack_dir.join("entities");
        let _ = fs::create_dir_all(&entities_dir);
        fs::write(
            entities_dir.join("guard.json"),
            r#"{
                "format_version": "1.20.0",
                "minecraft:entity": {
                    "description": { "identifier": "custom:guard", "is_summonable": true },
                    "components": {
                        "minecraft:health": { "value": 40, "max": 40 },
                        "minecraft:movement": { "value": 0.3 }
                    }
                }
            }"#,
        )
        .unwrap();

        let packs = load_all_packs(&dir);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].manifest.header.name, "Test");
        assert_eq!(packs[0].entities.len(), 1);
        assert_eq!(packs[0].entities[0].identifier, "custom:guard");
        assert_eq!(packs[0].entities[0].max_health, 40.0);

        let _ = fs::remove_dir_all(&dir);
    }
}
