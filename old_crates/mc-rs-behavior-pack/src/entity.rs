//! Bedrock entity JSON parsing (entities/*.json).

use std::collections::HashMap;

use serde::Deserialize;

/// Raw entity file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityFile {
    pub format_version: String,
    #[serde(rename = "minecraft:entity")]
    pub entity: EntityDefinition,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDefinition {
    pub description: EntityDescription,
    #[serde(default)]
    pub components: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDescription {
    pub identifier: String,
    #[serde(default)]
    pub is_spawnable: bool,
    #[serde(default)]
    pub is_summonable: bool,
}

/// Parsed entity stats extracted from JSON components.
#[derive(Debug, Clone)]
pub struct ParsedEntity {
    pub identifier: String,
    pub is_spawnable: bool,
    pub is_summonable: bool,
    pub max_health: f32,
    pub movement_speed: f32,
    pub attack_damage: f32,
    pub bb_width: f32,
    pub bb_height: f32,
    /// Names of `minecraft:behavior.*` components found.
    pub behaviors: Vec<String>,
}

impl EntityFile {
    /// Parse from a JSON string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid entity JSON: {e}"))
    }

    /// Extract usable stats from the raw JSON components.
    pub fn extract(&self) -> ParsedEntity {
        let comps = &self.entity.components;

        let max_health = comps
            .get("minecraft:health")
            .and_then(|v| v.get("max").and_then(|m| m.as_f64()))
            .or_else(|| {
                comps
                    .get("minecraft:health")
                    .and_then(|v| v.get("value").and_then(|m| m.as_f64()))
            })
            .unwrap_or(10.0) as f32;

        let movement_speed = comps
            .get("minecraft:movement")
            .and_then(|v| v.get("value").and_then(|m| m.as_f64()))
            .unwrap_or(0.25) as f32;

        let attack_damage = comps
            .get("minecraft:attack")
            .and_then(|v| v.get("damage").and_then(|d| d.as_f64()))
            .unwrap_or(0.0) as f32;

        let bb_width = comps
            .get("minecraft:collision_box")
            .and_then(|v| v.get("width").and_then(|w| w.as_f64()))
            .unwrap_or(0.6) as f32;

        let bb_height = comps
            .get("minecraft:collision_box")
            .and_then(|v| v.get("height").and_then(|h| h.as_f64()))
            .unwrap_or(1.8) as f32;

        let behaviors: Vec<String> = comps
            .keys()
            .filter_map(|k| k.strip_prefix("minecraft:behavior.").map(String::from))
            .collect();

        ParsedEntity {
            identifier: self.entity.description.identifier.clone(),
            is_spawnable: self.entity.description.is_spawnable,
            is_summonable: self.entity.description.is_summonable,
            max_health,
            movement_speed,
            attack_damage,
            bb_width,
            bb_height,
            behaviors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_entity() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:entity": {
                "description": {
                    "identifier": "custom:guard",
                    "is_spawnable": true,
                    "is_summonable": true
                },
                "components": {
                    "minecraft:health": { "value": 30, "max": 30 },
                    "minecraft:movement": { "value": 0.35 },
                    "minecraft:attack": { "damage": 5.0 },
                    "minecraft:collision_box": { "width": 0.7, "height": 2.0 },
                    "minecraft:behavior.random_stroll": { "priority": 6 },
                    "minecraft:behavior.look_at_player": { "priority": 7 }
                }
            }
        }"#;
        let file = EntityFile::parse_json(json).unwrap();
        let e = file.extract();
        assert_eq!(e.identifier, "custom:guard");
        assert!(e.is_spawnable);
        assert!(e.is_summonable);
        assert_eq!(e.max_health, 30.0);
        assert_eq!(e.movement_speed, 0.35);
        assert_eq!(e.attack_damage, 5.0);
        assert_eq!(e.bb_width, 0.7);
        assert_eq!(e.bb_height, 2.0);
        assert_eq!(e.behaviors.len(), 2);
    }

    #[test]
    fn parse_entity_defaults() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:entity": {
                "description": { "identifier": "custom:simple" },
                "components": {}
            }
        }"#;
        let file = EntityFile::parse_json(json).unwrap();
        let e = file.extract();
        assert_eq!(e.identifier, "custom:simple");
        assert!(!e.is_spawnable);
        assert_eq!(e.max_health, 10.0);
        assert_eq!(e.movement_speed, 0.25);
        assert_eq!(e.attack_damage, 0.0);
        assert_eq!(e.bb_width, 0.6);
        assert_eq!(e.bb_height, 1.8);
        assert!(e.behaviors.is_empty());
    }

    #[test]
    fn parse_invalid_entity() {
        assert!(EntityFile::parse_json("not json").is_err());
    }
}
