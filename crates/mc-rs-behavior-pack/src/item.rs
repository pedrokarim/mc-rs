//! Bedrock custom item JSON parsing (items/*.json).

use std::collections::HashMap;

use serde::Deserialize;

/// Raw item file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemFile {
    pub format_version: String,
    #[serde(rename = "minecraft:item")]
    pub item: ItemDefinition,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDefinition {
    pub description: ItemDescription,
    #[serde(default)]
    pub components: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDescription {
    pub identifier: String,
    #[serde(default)]
    pub category: Option<String>,
}

/// Parsed item properties.
#[derive(Debug, Clone)]
pub struct ParsedItem {
    pub identifier: String,
    pub max_stack_size: u8,
    /// All behavior pack items are component-based.
    pub is_component_based: bool,
}

impl ItemFile {
    /// Parse from a JSON string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid item JSON: {e}"))
    }

    /// Extract usable properties.
    pub fn extract(&self) -> ParsedItem {
        let max_stack = self
            .item
            .components
            .get("minecraft:max_stack_size")
            .and_then(|v| {
                // Can be { "value": 64 } or just 64
                v.get("value")
                    .and_then(|m| m.as_u64())
                    .or_else(|| v.as_u64())
            })
            .unwrap_or(64) as u8;

        ParsedItem {
            identifier: self.item.description.identifier.clone(),
            max_stack_size: max_stack,
            is_component_based: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_custom_item() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:item": {
                "description": { "identifier": "custom:ruby" },
                "components": {
                    "minecraft:max_stack_size": { "value": 16 }
                }
            }
        }"#;
        let file = ItemFile::parse_json(json).unwrap();
        let item = file.extract();
        assert_eq!(item.identifier, "custom:ruby");
        assert_eq!(item.max_stack_size, 16);
        assert!(item.is_component_based);
    }

    #[test]
    fn default_stack_size() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:item": {
                "description": { "identifier": "custom:gem" },
                "components": {}
            }
        }"#;
        let file = ItemFile::parse_json(json).unwrap();
        let item = file.extract();
        assert_eq!(item.max_stack_size, 64);
    }
}
