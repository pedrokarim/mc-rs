//! Bedrock recipe JSON parsing (recipes/*.json).

use std::collections::HashMap;

use serde::Deserialize;

/// A recipe file can contain either a shaped or shapeless recipe.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeFile {
    pub format_version: String,
    #[serde(rename = "minecraft:recipe_shaped")]
    pub shaped: Option<ShapedRecipeDef>,
    #[serde(rename = "minecraft:recipe_shapeless")]
    pub shapeless: Option<ShapelessRecipeDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShapedRecipeDef {
    pub description: RecipeDescription,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Pattern rows, e.g. `["ABA", " C ", " C "]`.
    pub pattern: Vec<String>,
    /// Mapping of pattern characters to items.
    pub key: HashMap<String, RecipeKeyItem>,
    pub result: RecipeResultItem,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShapelessRecipeDef {
    pub description: RecipeDescription,
    #[serde(default)]
    pub tags: Vec<String>,
    pub ingredients: Vec<RecipeKeyItem>,
    pub result: RecipeResultItem,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeDescription {
    pub identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeKeyItem {
    pub item: String,
    #[serde(default)]
    pub data: i16,
    #[serde(default = "default_count")]
    pub count: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeResultItem {
    pub item: String,
    #[serde(default)]
    pub data: u16,
    #[serde(default = "default_count")]
    pub count: u8,
}

fn default_count() -> u8 {
    1
}

/// A flattened input slot for conversion to the internal recipe format.
#[derive(Debug, Clone)]
pub struct FlatInput {
    pub item_name: String,
    pub count: u8,
    pub metadata: i16,
}

impl ShapedRecipeDef {
    /// Convert the pattern+key format to a flat grid of inputs.
    ///
    /// Returns `(width, height, inputs)` where inputs has `width * height` entries.
    /// Empty pattern slots become entries with an empty item_name.
    pub fn flatten(&self) -> (u8, u8, Vec<FlatInput>) {
        let height = self.pattern.len() as u8;
        let width = self.pattern.iter().map(|r| r.len()).max().unwrap_or(0) as u8;

        let mut inputs = Vec::with_capacity((width as usize) * (height as usize));
        for row in &self.pattern {
            for (i, ch) in row.chars().enumerate() {
                let _ = i;
                if ch == ' ' {
                    inputs.push(FlatInput {
                        item_name: String::new(),
                        count: 0,
                        metadata: -1,
                    });
                } else if let Some(key_item) = self.key.get(&ch.to_string()) {
                    inputs.push(FlatInput {
                        item_name: key_item.item.clone(),
                        count: key_item.count,
                        metadata: key_item.data,
                    });
                } else {
                    inputs.push(FlatInput {
                        item_name: String::new(),
                        count: 0,
                        metadata: -1,
                    });
                }
            }
            // Pad row to width if shorter.
            for _ in row.len()..(width as usize) {
                inputs.push(FlatInput {
                    item_name: String::new(),
                    count: 0,
                    metadata: -1,
                });
            }
        }

        (width, height, inputs)
    }
}

impl RecipeFile {
    /// Parse from a JSON string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid recipe JSON: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shaped_recipe() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:recipe_shaped": {
                "description": { "identifier": "custom:ruby_block" },
                "tags": ["crafting_table"],
                "pattern": ["AAA", "AAA", "AAA"],
                "key": {
                    "A": { "item": "custom:ruby" }
                },
                "result": { "item": "custom:ruby_block", "count": 1 }
            }
        }"#;
        let file = RecipeFile::parse_json(json).unwrap();
        let shaped = file.shaped.unwrap();
        assert_eq!(shaped.description.identifier, "custom:ruby_block");
        let (w, h, inputs) = shaped.flatten();
        assert_eq!(w, 3);
        assert_eq!(h, 3);
        assert_eq!(inputs.len(), 9);
        assert!(inputs.iter().all(|i| i.item_name == "custom:ruby"));
    }

    #[test]
    fn parse_shapeless_recipe() {
        let json = r#"{
            "format_version": "1.20.0",
            "minecraft:recipe_shapeless": {
                "description": { "identifier": "custom:rubies_from_block" },
                "tags": ["crafting_table"],
                "ingredients": [
                    { "item": "custom:ruby_block" }
                ],
                "result": { "item": "custom:ruby", "count": 9 }
            }
        }"#;
        let file = RecipeFile::parse_json(json).unwrap();
        let shapeless = file.shapeless.unwrap();
        assert_eq!(shapeless.description.identifier, "custom:rubies_from_block");
        assert_eq!(shapeless.ingredients.len(), 1);
        assert_eq!(shapeless.result.count, 9);
    }
}
