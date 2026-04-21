//! Métadonnées vanilla de 77 items Bedrock depuis `data/items_vanilla.json`
//! (extrait de `.reference/bedrock-samples/behavior_pack/items/*.json`,
//! Mojang 1.26.10.4).
//!
//! Couvre les items où les propriétés (food, durability, max_stack,
//! tags) sont définies en data-driven côté Bedrock (principalement
//! nourriture et bundles).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const ITEMS_JSON: &str = include_str!("../data/items_vanilla.json");

#[derive(Deserialize, Debug, Clone)]
pub struct ItemMeta {
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub max_stack_size: Option<u32>,
    pub durability: Option<u32>,
    pub nutrition: Option<u32>,
    /// Saturation peut être un f32 OU un enum string ("supernatural", "meat", etc.)
    pub saturation: Option<serde_json::Value>,
    pub is_food: bool,
}

static ITEMS: LazyLock<HashMap<String, ItemMeta>> =
    LazyLock::new(|| serde_json::from_str(ITEMS_JSON).expect("valid items_vanilla.json"));

pub fn for_item(id: &str) -> Option<&'static ItemMeta> {
    ITEMS.get(id)
}

pub fn count() -> usize {
    ITEMS.len()
}

pub fn is_food(id: &str) -> bool {
    ITEMS
        .get(id)
        .is_some_and(|i| i.is_food || i.nutrition.is_some())
}

pub fn nutrition(id: &str) -> Option<u32> {
    ITEMS.get(id).and_then(|i| i.nutrition)
}

pub fn saturation(id: &str) -> Option<f32> {
    ITEMS
        .get(id)
        .and_then(|i| i.saturation.as_ref())
        .and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_f64().map(|f| f as f32),
            // Enum strings Bedrock → valeurs approximatives.
            serde_json::Value::String(s) => match s.as_str() {
                "poor" => Some(0.2),
                "low" => Some(0.6),
                "normal" => Some(1.2),
                "good" => Some(1.6),
                "max" => Some(2.0),
                "supernatural" => Some(2.4),
                _ => None,
            },
            _ => None,
        })
}

pub fn max_stack_size(id: &str) -> Option<u32> {
    ITEMS.get(id).and_then(|i| i.max_stack_size)
}

pub fn durability(id: &str) -> Option<u32> {
    ITEMS.get(id).and_then(|i| i.durability)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_loaded() {
        assert!(count() >= 70);
    }

    #[test]
    fn apple_is_food() {
        assert!(is_food("minecraft:apple"));
        assert_eq!(nutrition("minecraft:apple"), Some(4));
    }

    #[test]
    fn bread_is_food() {
        assert!(is_food("minecraft:bread"));
    }
}
