//! Spawn rules vanilla chargées depuis `data/spawn_rules.json`
//! (consolidées depuis `.reference/bedrock-samples/behavior_pack/spawn_rules/`,
//! Mojang officiel 1.26.10.4).
//!
//! 56 mobs avec leurs conditions de spawn (biome, lumière, surface,
//! difficulté, etc.). Le système de spawn naturel côté serveur n'est pas
//! encore wiré — ce module expose juste les données.

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
}
