//! Métadonnées vanilla de 126 entités depuis `data/entities.json`
//! (extrait depuis `.reference/bedrock-samples/behavior_pack/entities/*.json`,
//! Mojang 1.26.10.4).
//!
//! Expose les infos clés de chaque mob : runtime_identifier, spawnable,
//! summonable, type_family, health, attack, scale. Permet de construire
//! les entités vanilla avec les bonnes stats sans copier-coller.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const ENTITIES_JSON: &str = include_str!("../data/entities.json");

#[derive(Deserialize, Debug, Clone)]
pub struct EntityMeta {
    pub runtime_identifier: String,
    pub is_spawnable: bool,
    pub is_summonable: bool,
    pub is_experimental: bool,
    #[serde(default)]
    pub family: Vec<String>,
    /// Health peut être f32 OU { range_min, range_max } selon les mobs.
    pub health: Option<serde_json::Value>,
    pub attack: Option<serde_json::Value>,
    pub scale: Option<serde_json::Value>,
}

fn extract_f32(v: &serde_json::Value) -> Option<f32> {
    match v {
        serde_json::Value::Number(n) => n.as_f64().map(|f| f as f32),
        serde_json::Value::Object(m) => {
            let min = m
                .get("range_min")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let max = m
                .get("range_max")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            match (min, max) {
                (Some(a), Some(b)) => Some((a + b) / 2.0),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            }
        }
        _ => None,
    }
}

static ENTITIES: LazyLock<HashMap<String, EntityMeta>> =
    LazyLock::new(|| serde_json::from_str(ENTITIES_JSON).expect("valid entities.json"));

pub fn for_identifier(id: &str) -> Option<&'static EntityMeta> {
    ENTITIES.get(id)
}

pub fn count() -> usize {
    ENTITIES.len()
}

pub fn is_spawnable(id: &str) -> bool {
    ENTITIES.get(id).is_some_and(|e| e.is_spawnable)
}

pub fn is_summonable(id: &str) -> bool {
    ENTITIES.get(id).is_some_and(|e| e.is_summonable)
}

pub fn health(id: &str) -> Option<f32> {
    ENTITIES
        .get(id)
        .and_then(|e| e.health.as_ref())
        .and_then(extract_f32)
}

pub fn families(id: &str) -> &'static [String] {
    ENTITIES.get(id).map(|e| e.family.as_slice()).unwrap_or(&[])
}

pub fn has_family(id: &str, family: &str) -> bool {
    families(id).iter().any(|f| f == family)
}

pub fn all_identifiers() -> Vec<&'static String> {
    ENTITIES.keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_vanilla_entities_loaded() {
        assert!(count() >= 120, "count={}", count());
    }

    #[test]
    fn zombie_has_undead_family() {
        assert!(has_family("minecraft:zombie", "undead"));
        assert!(has_family("minecraft:zombie", "monster"));
    }

    #[test]
    fn cow_is_spawnable() {
        assert!(is_spawnable("minecraft:cow"));
    }

    #[test]
    fn cow_has_health() {
        assert!(health("minecraft:cow").is_some());
    }
}
