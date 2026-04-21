//! Registres vanilla statiques (effets, enchantements, potions, dimensions)
//! depuis `data/vanilla/*.json` (extrait de
//! `.reference/bedrock-samples/metadata/vanilladata_modules/`, Mojang 1.26.10.4).
//!
//! Utile pour valider des noms au runtime (ex: /effect, /enchant) et
//! énumérer les valeurs autorisées.

use std::collections::HashSet;
use std::sync::LazyLock;

const EFFECTS_JSON: &str = include_str!("../data/vanilla/effects.json");
const ENCHANTMENTS_JSON: &str = include_str!("../data/vanilla/enchantments.json");
const POTION_EFFECTS_JSON: &str = include_str!("../data/vanilla/potion-effects.json");
const POTION_TYPES_JSON: &str = include_str!("../data/vanilla/potion-types.json");
const DIMENSIONS_JSON: &str = include_str!("../data/vanilla/dimensions.json");

fn parse(json: &'static str) -> Vec<String> {
    serde_json::from_str(json).expect("valid vanilla registry list")
}

pub static EFFECTS: LazyLock<Vec<String>> = LazyLock::new(|| parse(EFFECTS_JSON));
pub static ENCHANTMENTS: LazyLock<Vec<String>> = LazyLock::new(|| parse(ENCHANTMENTS_JSON));
pub static POTION_EFFECTS: LazyLock<Vec<String>> = LazyLock::new(|| parse(POTION_EFFECTS_JSON));
pub static POTION_TYPES: LazyLock<Vec<String>> = LazyLock::new(|| parse(POTION_TYPES_JSON));
pub static DIMENSIONS: LazyLock<Vec<String>> = LazyLock::new(|| parse(DIMENSIONS_JSON));

static EFFECTS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| EFFECTS.iter().map(String::as_str).collect());
static ENCHANTMENTS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ENCHANTMENTS.iter().map(String::as_str).collect());
static POTION_EFFECTS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| POTION_EFFECTS.iter().map(String::as_str).collect());
static POTION_TYPES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| POTION_TYPES.iter().map(String::as_str).collect());
static DIMENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| DIMENSIONS.iter().map(String::as_str).collect());

pub fn is_effect(name: &str) -> bool {
    EFFECTS_SET.contains(name)
}
pub fn is_enchantment(name: &str) -> bool {
    ENCHANTMENTS_SET.contains(name)
}
pub fn is_potion_effect(name: &str) -> bool {
    POTION_EFFECTS_SET.contains(name)
}
pub fn is_potion_type(name: &str) -> bool {
    POTION_TYPES_SET.contains(name)
}
pub fn is_dimension(name: &str) -> bool {
    DIMENSIONS_SET.contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_loaded() {
        assert!(EFFECTS.len() > 30);
        assert!(is_effect("minecraft:speed"));
        assert!(!is_effect("minecraft:nonexistent"));
    }

    #[test]
    fn enchantments_loaded() {
        assert!(ENCHANTMENTS.len() > 30);
        assert!(is_enchantment("minecraft:sharpness"));
    }

    #[test]
    fn potions_loaded() {
        assert!(POTION_EFFECTS.len() > 40);
        assert!(is_potion_effect("minecraft:healing"));
    }

    #[test]
    fn dimensions_loaded() {
        assert_eq!(DIMENSIONS.len(), 3);
        assert!(is_dimension("minecraft:overworld"));
        assert!(is_dimension("minecraft:nether"));
        assert!(is_dimension("minecraft:the_end"));
    }
}
