//! Migration des identifiants d'items legacy : port de PMMP
//! `ItemIdMetaUpgrader` (`src/data/bedrock/item/upgrade/`) et de ses schémas
//! `pmmp/BedrockItemUpgradeSchema` (`id_meta_upgrade_schema/*.json`).
//!
//! Les données Mojang (recettes de bedrock-samples…) emploient encore des noms
//! legacy (`minecraft:emptymap`, `minecraft:dye` + meta, `minecraft:chain`…) :
//! le schéma les convertit vers les identifiants de la table d'items courante.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

/// Schémas consolidés par `scripts/import_item_upgrade_schema.py`.
const SCHEMAS_JSON: &str = include_str!("../data/item_id_upgrade_schema.json");

#[derive(Deserialize)]
struct RawSchema {
    id: u32,
    #[serde(rename = "renamedIds", default)]
    renamed_ids: HashMap<String, String>,
    #[serde(rename = "remappedMetas", default)]
    remapped_metas: HashMap<String, HashMap<String, String>>,
}

struct Schema {
    id: u32,
    renamed_ids: HashMap<String, String>,
    remapped_metas: HashMap<String, HashMap<i32, String>>,
}

static SCHEMAS: LazyLock<Vec<Schema>> = LazyLock::new(|| {
    let mut raw: Vec<RawSchema> =
        serde_json::from_str(SCHEMAS_JSON).expect("item_id_upgrade_schema.json valide");
    raw.sort_by_key(|schema| schema.id);
    raw.into_iter()
        .map(|schema| Schema {
            id: schema.id,
            renamed_ids: schema.renamed_ids,
            remapped_metas: schema
                .remapped_metas
                .into_iter()
                .map(|(id, metas)| {
                    let metas = metas
                        .into_iter()
                        .filter_map(|(meta, new_id)| meta.parse().ok().map(|meta| (meta, new_id)))
                        .collect();
                    (id, metas)
                })
                .collect(),
        })
        .collect()
});

/// `ItemIdMetaUpgrader::upgrade` : applique les schémas dans l'ordre de leur
/// numéro ; pour chacun, un remappage par meta (qui remet la meta à 0)
/// l'emporte sur un renommage. Les clés sont comparées en minuscules ASCII
/// (`mb_strtolower($id, 'US-ASCII')`).
pub fn upgrade(id: &str, meta: i32) -> (String, i32) {
    upgrade_since(id, meta, 0)
}

/// Comme [`upgrade`], en n'appliquant que les schémas de numéro au moins égal
/// à `first_schema` : ceux que des données d'une version connue ne reflètent
/// pas encore (PMMP ne rejoue que les schémas postérieurs à la version des
/// données).
pub fn upgrade_since(id: &str, meta: i32, first_schema: u32) -> (String, i32) {
    let mut id = id.to_string();
    let mut meta = meta;
    for schema in SCHEMAS.iter().filter(|schema| schema.id >= first_schema) {
        let key = id.to_ascii_lowercase();
        if let Some(new_id) = schema.remapped_metas.get(&key).and_then(|m| m.get(&meta)) {
            id = new_id.clone();
            meta = 0;
        } else if let Some(new_id) = schema.renamed_ids.get(&key) {
            id = new_id.clone();
        }
    }
    (id, meta)
}

/// Nom d'item tel qu'écrit dans les données Mojang → (identifiant, meta
/// éventuelle). Sans espace de noms, `minecraft:` est implicite ; la forme
/// legacy `nom:meta` (`minecraft:coal:1`) porte la meta en suffixe.
pub fn parse_item_name(name: &str) -> (String, Option<i32>) {
    let (base, meta) = match name.rsplit_once(':') {
        Some((head, tail)) => match tail.parse::<i32>() {
            Ok(meta) => (head, Some(meta)),
            Err(_) => (name, None),
        },
        None => (name, None),
    };
    let id = if base.contains(':') {
        base.to_string()
    } else {
        format!("minecraft:{base}")
    };
    (id, meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_legacy_ids() {
        assert_eq!(
            upgrade("minecraft:emptymap", 0),
            ("minecraft:empty_map".to_string(), 0)
        );
        assert_eq!(
            upgrade("minecraft:chain", 0),
            ("minecraft:iron_chain".to_string(), 0)
        );
        assert_eq!(
            upgrade("minecraft:melon", 0),
            ("minecraft:melon_slice".to_string(), 0)
        );
    }

    #[test]
    fn upgrade_since_skips_older_schemas() {
        // 0281 (1.26.40 → 1.26.50) renomme la photo.
        assert_eq!(
            upgrade_since("minecraft:photo", 0, 281),
            ("minecraft:photo_item".to_string(), 0)
        );
        // Les schémas antérieurs ne sont pas rejoués : `minecraft:coal` meta 1
        // n'est plus remappé en charbon de bois.
        assert_eq!(
            upgrade_since("minecraft:coal", 1, 281),
            ("minecraft:coal".to_string(), 1)
        );
    }

    #[test]
    fn remaps_meta_variants_and_resets_meta() {
        assert_eq!(
            upgrade("minecraft:coal", 1),
            ("minecraft:charcoal".to_string(), 0)
        );
        // Meta sans remappage : identifiant et meta conservés.
        assert_eq!(
            upgrade("minecraft:stone", 0),
            ("minecraft:stone".to_string(), 0)
        );
    }

    #[test]
    fn parses_mojang_item_names() {
        assert_eq!(
            parse_item_name("stick"),
            ("minecraft:stick".to_string(), None)
        );
        assert_eq!(
            parse_item_name("minecraft:stick"),
            ("minecraft:stick".to_string(), None)
        );
        assert_eq!(
            parse_item_name("minecraft:coal:1"),
            ("minecraft:coal".to_string(), Some(1))
        );
        assert_eq!(
            parse_item_name("coal:1"),
            ("minecraft:coal".to_string(), Some(1))
        );
    }
}
