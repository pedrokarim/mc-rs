//! Spawn rules vanilla chargées depuis `data/spawn_rules.json`
//! (consolidées depuis `.reference/bedrock-samples/behavior_pack/spawn_rules/`,
//! Mojang officiel 1.26.10.4).
//!
//! 56 mobs avec leurs conditions de spawn (biome, lumière, surface,
//! difficulté, etc.). Câblé dans le spawner naturel (`mob_spawner`) :
//! `spawn_weight` (pondération), `brightness_range` (lumière) et
//! `biome_allows` (filtrage par tags de biome au point de spawn).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const SPAWN_RULES_JSON: &str = include_str!("../data/spawn_rules.json");

#[derive(Deserialize, Debug, Clone)]
pub struct SpawnRule {
    pub population_control: String,
    pub conditions: Vec<serde_json::Value>,
}

static SPAWN_RULES: LazyLock<HashMap<String, SpawnRule>> =
    LazyLock::new(|| serde_json::from_str(SPAWN_RULES_JSON).expect("valid spawn_rules.json"));

pub fn for_entity(entity_id: &str) -> Option<&'static SpawnRule> {
    SPAWN_RULES.get(entity_id)
}

pub fn count() -> usize {
    SPAWN_RULES.len()
}

pub fn spawn_weight(entity_id: &str) -> Option<u32> {
    let rule = SPAWN_RULES.get(entity_id)?;
    for cond in &rule.conditions {
        if let Some(weight) = cond.get("minecraft:weight").and_then(|w| w.get("default")) {
            return weight.as_u64().map(|v| v as u32);
        }
    }
    None
}

pub fn brightness_range(entity_id: &str) -> Option<(u8, u8)> {
    let rule = SPAWN_RULES.get(entity_id)?;
    for cond in &rule.conditions {
        if let Some(bf) = cond.get("minecraft:brightness_filter") {
            let min = bf.get("min").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let max = bf.get("max").and_then(|v| v.as_u64()).unwrap_or(15) as u8;
            return Some((min, max));
        }
    }
    None
}

pub fn is_surface_spawner(entity_id: &str) -> bool {
    let Some(rule) = SPAWN_RULES.get(entity_id) else {
        return false;
    };
    rule.conditions
        .iter()
        .any(|c| c.get("minecraft:spawns_on_surface").is_some())
}

pub fn is_underground_spawner(entity_id: &str) -> bool {
    let Some(rule) = SPAWN_RULES.get(entity_id) else {
        return false;
    };
    rule.conditions
        .iter()
        .any(|c| c.get("minecraft:spawns_underground").is_some())
}

pub fn is_water_spawner(entity_id: &str) -> bool {
    let Some(rule) = SPAWN_RULES.get(entity_id) else {
        return false;
    };
    rule.conditions
        .iter()
        .any(|c| c.get("minecraft:spawns_underwater").is_some())
}

/// Évalue un nœud de `minecraft:biome_filter` contre l'ensemble des tags du
/// biome au point de spawn. Formats Bedrock supportés :
///   - test simple : `{"test":"has_biome_tag","operator":"==","value":"desert"}`
///   - wrappers : `{"all_of":[...]}`, `{"any_of":[...]}`, `{"none_of":[...]}`
///   - tableau de tests (AND implicite) ou tableau de tableaux (OR-of-AND,
///     ex. stray : `[[==frozen,!=ocean],[==frozen,==ocean]]`).
fn eval_biome_node(node: &serde_json::Value, tags: &[String]) -> bool {
    if let Some(arr) = node.as_array() {
        // Tableau imbriqué → OR sur les éléments ; sinon AND.
        if arr.iter().any(|e| e.is_array()) {
            return arr.iter().any(|e| eval_biome_node(e, tags));
        }
        return arr.iter().all(|e| eval_biome_node(e, tags));
    }
    if let Some(inner) = node.get("all_of").and_then(|v| v.as_array()) {
        return inner.iter().all(|e| eval_biome_node(e, tags));
    }
    if let Some(inner) = node.get("any_of").and_then(|v| v.as_array()) {
        return inner.iter().any(|e| eval_biome_node(e, tags));
    }
    if let Some(inner) = node.get("none_of").and_then(|v| v.as_array()) {
        return !inner.iter().any(|e| eval_biome_node(e, tags));
    }
    // Test feuille `has_biome_tag`.
    let value = node.get("value").and_then(|v| v.as_str()).unwrap_or("");
    let op = node
        .get("operator")
        .and_then(|v| v.as_str())
        .unwrap_or("==");
    let has = tags.iter().any(|t| t == value);
    match op {
        "!=" => !has,
        _ => has, // "==" (et par défaut)
    }
}

/// Le mob peut-il spawner dans un biome portant `tags` ?
///
/// `conditions` est une liste de *sets* (OR entre sets, AND dans un set). Un
/// mob est autorisé si AU MOINS un set dont le `biome_filter` matche les tags
/// (ou si aucun set n'a de `biome_filter` → pas de restriction de biome, ex.
/// blaze/wither_skeleton, déjà exclus de l'overworld par `MobKind::habitat`).
/// Mob inconnu (pas de règle) → autorisé (le poids le filtre déjà en amont).
pub fn biome_allows(entity_id: &str, tags: &[String]) -> bool {
    let Some(rule) = SPAWN_RULES.get(entity_id) else {
        return true;
    };
    let mut had_filter = false;
    for set in &rule.conditions {
        if let Some(bf) = set.get("minecraft:biome_filter") {
            had_filter = true;
            if eval_biome_node(bf, tags) {
                return true;
            }
        }
    }
    !had_filter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_enough_spawn_rules() {
        assert!(count() > 50);
    }

    #[test]
    fn zombie_is_surface_monster() {
        let rule = for_entity("minecraft:zombie").expect("zombie has rule");
        assert_eq!(rule.population_control, "monster");
        assert!(is_surface_spawner("minecraft:zombie"));
    }

    #[test]
    fn zombie_brightness_is_dark() {
        let (min, max) = brightness_range("minecraft:zombie").expect("zombie brightness");
        assert_eq!(min, 0);
        assert!(max <= 7);
    }

    #[test]
    fn cow_has_weight() {
        assert!(spawn_weight("minecraft:cow").is_some());
    }

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn husk_requires_desert_tag() {
        // husk : biome_filter == "desert".
        assert!(biome_allows(
            "minecraft:husk",
            &tags(&["desert", "monster"])
        ));
        assert!(!biome_allows(
            "minecraft:husk",
            &tags(&["animal", "monster"])
        ));
    }

    #[test]
    fn zombie_requires_monster_tag() {
        // zombie : biome_filter == "monster".
        assert!(biome_allows(
            "minecraft:zombie",
            &tags(&["animal", "monster"])
        ));
        assert!(!biome_allows("minecraft:zombie", &tags(&["frozen", "ice"])));
    }

    #[test]
    fn stray_or_of_ands_resolves_to_frozen() {
        // stray : [[==frozen,!=ocean],[==frozen,==ocean]] ⇒ frozen.
        assert!(biome_allows("minecraft:stray", &tags(&["frozen"])));
        assert!(biome_allows("minecraft:stray", &tags(&["frozen", "ocean"])));
        assert!(!biome_allows(
            "minecraft:stray",
            &tags(&["animal", "monster"])
        ));
    }

    #[test]
    fn unknown_mob_is_unrestricted() {
        assert!(biome_allows("minecraft:not_a_mob", &tags(&["whatever"])));
    }

    #[test]
    fn cow_needs_animal_tag_not_desert() {
        // cow : biome_filter == "animal" → pas de vache au désert.
        assert!(biome_allows("minecraft:cow", &tags(&["animal", "monster"])));
        assert!(!biome_allows(
            "minecraft:cow",
            &tags(&["desert", "monster"])
        ));
    }
}
