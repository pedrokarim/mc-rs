//! Biomes vanilla chargés depuis `data/biomes.json`
//! (extraction depuis `.reference/bedrock-samples/behavior_pack/biomes/*.biome.json`,
//! Mojang officiel 1.26.10.4).
//!
//! 87 biomes vanilla avec leurs caractéristiques clés : temperature,
//! downfall, surface materials (top/mid/foundation/sea), tags.
//!
//! Le générateur de terrain actuel utilise `biomes_registry` (11 biomes
//! codés à la main). Ce module fournit la référence vanilla complète
//! pour élargir progressivement la palette.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const BIOMES_JSON: &str = include_str!("../data/biomes.json");

#[derive(Deserialize, Debug, Clone)]
pub struct BiomeData {
    pub temperature: f32,
    pub downfall: f32,
    pub top_material: String,
    pub mid_material: String,
    pub foundation_material: String,
    pub sea_material: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

static BIOMES: LazyLock<HashMap<String, BiomeData>> =
    LazyLock::new(|| serde_json::from_str(BIOMES_JSON).expect("valid biomes.json"));

pub fn for_biome(identifier: &str) -> Option<&'static BiomeData> {
    BIOMES.get(identifier)
}

pub fn count() -> usize {
    BIOMES.len()
}

/// Retourne la liste des biomes ayant un tag donné.
/// Utilisé par le filtrage des spawn_rules (ex: `has_biome_tag` "monster").
pub fn biomes_with_tag(tag: &str) -> Vec<&'static String> {
    BIOMES
        .iter()
        .filter(|(_, b)| b.tags.iter().any(|t| t == tag))
        .map(|(id, _)| id)
        .collect()
}

/// Retourne le top_material d'un biome (utilisé par le générateur de terrain).
pub fn top_material(identifier: &str) -> Option<&'static str> {
    BIOMES.get(identifier).map(|b| b.top_material.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_all_vanilla_biomes() {
        assert!(count() >= 80);
    }

    #[test]
    fn plains_biome_has_expected_surface() {
        let plains = for_biome("minecraft:plains").expect("plains biome exists");
        assert_eq!(plains.top_material, "minecraft:grass_block");
        assert_eq!(plains.foundation_material, "minecraft:stone");
    }

    #[test]
    fn desert_is_hot_and_dry() {
        let desert = for_biome("minecraft:desert").expect("desert biome");
        assert!(desert.temperature > 1.5);
        assert_eq!(desert.downfall, 0.0);
    }

    #[test]
    fn plains_has_monster_tag() {
        let plains = for_biome("minecraft:plains").expect("plains");
        assert!(plains.tags.iter().any(|t| t == "monster"));
    }

    #[test]
    fn tag_filtering_works() {
        let monster_biomes = biomes_with_tag("monster");
        assert!(monster_biomes.len() > 20);
    }
}
