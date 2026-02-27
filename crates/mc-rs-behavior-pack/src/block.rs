//! Bedrock custom block JSON parsing (blocks/*.json).

use std::collections::HashMap;

use serde::Deserialize;

/// Raw block file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockFile {
    pub format_version: String,
    #[serde(rename = "minecraft:block")]
    pub block: BlockDefinition,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDefinition {
    pub description: BlockDescription,
    #[serde(default)]
    pub components: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDescription {
    pub identifier: String,
}

/// Parsed block properties.
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub identifier: String,
    pub hardness: f32,
    pub is_solid: bool,
}

impl BlockFile {
    /// Parse from a JSON string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid block JSON: {e}"))
    }

    /// Extract usable properties.
    pub fn extract(&self) -> ParsedBlock {
        let hardness = self
            .block
            .components
            .get("minecraft:destructible_by_mining")
            .and_then(|v| v.get("seconds_to_destroy").and_then(|s| s.as_f64()))
            .unwrap_or(1.0) as f32;

        // Solid unless explicitly set to false via collision box component.
        let is_solid = self
            .block
            .components
            .get("minecraft:collision_box")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        ParsedBlock {
            identifier: self.block.description.identifier.clone(),
            hardness,
            is_solid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_custom_block() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:block": {
                "description": { "identifier": "custom:marble" },
                "components": {
                    "minecraft:destructible_by_mining": { "seconds_to_destroy": 2.5 }
                }
            }
        }"#;
        let file = BlockFile::parse_json(json).unwrap();
        let b = file.extract();
        assert_eq!(b.identifier, "custom:marble");
        assert_eq!(b.hardness, 2.5);
        assert!(b.is_solid);
    }

    #[test]
    fn default_hardness() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:block": {
                "description": { "identifier": "custom:soft" },
                "components": {}
            }
        }"#;
        let file = BlockFile::parse_json(json).unwrap();
        let b = file.extract();
        assert_eq!(b.hardness, 1.0);
        assert!(b.is_solid);
    }
}
