//! Propriétés liquides des états de bloc, relevées sur BDS (dump d'Allay, `data/block_liquid.json`, produit par
//! `scripts/dev/build_block_liquid.py`).
//!
//! - **waterlogging** (`canContainLiquidSource`) : l'état peut garder une source d'eau ; Bedrock la range dans la
//!   seconde couche du sous-chunk (`SubChunk::liquid`), le bloc restant dans la première ;
//! - **réaction au contact d'un liquide qui coule** (`liquidReactionOnTouch`).

use std::sync::LazyLock;

use serde::Deserialize;

const DATA_JSON: &str = include_str!("../data/block_liquid.json");

/// Ce que fait un liquide qui coule sur un bloc (BDS `liquidReactionOnTouch`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidReaction {
    /// Le liquide s'arrête.
    Blocking,
    /// Le bloc est cassé et lâche ses drops.
    Popped,
    /// Le bloc est détruit sans drops.
    Broken,
    /// Le liquide traverse le bloc.
    NoReaction,
}

#[derive(Deserialize)]
struct RawData {
    state_count: u32,
    can_contain_liquid_source: Vec<[u32; 2]>,
    liquid_reaction_on_touch: std::collections::HashMap<String, Vec<[u32; 2]>>,
}

struct Data {
    can_contain: Vec<bool>,
    reaction: Vec<LiquidReaction>,
}

static DATA: LazyLock<Data> = LazyLock::new(|| {
    let raw: RawData = serde_json::from_str(DATA_JSON).expect("block_liquid.json valide");
    let count = crate::world::block_registry_data::PALETTE_STATE_COUNT;
    assert_eq!(
        raw.state_count, count,
        "block_liquid.json ne correspond pas à la palette : relancer scripts/dev/build_block_liquid.py"
    );
    let mut can_contain = vec![false; count as usize];
    for [first, last] in raw.can_contain_liquid_source {
        can_contain[first as usize..=last as usize].fill(true);
    }
    let mut reaction = vec![LiquidReaction::Blocking; count as usize];
    for (name, ranges) in raw.liquid_reaction_on_touch {
        let value = match name.as_str() {
            "POPPED" => LiquidReaction::Popped,
            "BROKEN" => LiquidReaction::Broken,
            "NOREACTION" => LiquidReaction::NoReaction,
            other => panic!("réaction liquide inconnue dans block_liquid.json : {other}"),
        };
        for [first, last] in ranges {
            reaction[first as usize..=last as usize].fill(value);
        }
    }
    Data {
        can_contain,
        reaction,
    }
});

/// Vrai si l'état peut contenir une source d'eau (waterlogging).
pub fn can_contain_liquid(runtime_id: u32) -> bool {
    DATA.can_contain
        .get(runtime_id as usize)
        .copied()
        .unwrap_or(false)
}

/// Vrai si l'état est une source d'eau (`water` ou `flowing_water` de profondeur 0), la seule eau qu'un bloc
/// peut garder. Les états de l'eau suivent `liquid_depth` dans la palette (`liquids::water_height_percent`).
pub fn is_water_source(runtime_id: u32) -> bool {
    use crate::world::block_registry::BLOCKS;
    runtime_id == BLOCKS.get("minecraft:water")
        || runtime_id == BLOCKS.get("minecraft:flowing_water")
}

/// Réaction de l'état à un liquide qui coule (`Blocking` hors palette).
pub fn liquid_reaction(runtime_id: u32) -> LiquidReaction {
    DATA.reaction
        .get(runtime_id as usize)
        .copied()
        .unwrap_or(LiquidReaction::Blocking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry::BLOCKS;

    #[test]
    fn known_blocks_follow_bds() {
        for name in [
            "minecraft:seagrass",
            "minecraft:kelp",
            "minecraft:sea_pickle",
            "minecraft:oak_stairs",
            "minecraft:oak_slab",
            "minecraft:chest",
            // BDS déclare l'air capable de contenir une source (voir `ChunkCache::set_block`).
            "minecraft:air",
        ] {
            assert!(can_contain_liquid(BLOCKS.get(name)), "{name}");
        }
        for name in [
            "minecraft:stone",
            "minecraft:sand",
            "minecraft:torch",
            "minecraft:water",
        ] {
            assert!(!can_contain_liquid(BLOCKS.get(name)), "{name}");
        }
        assert_eq!(
            liquid_reaction(BLOCKS.get("minecraft:torch")),
            LiquidReaction::Popped
        );
        assert_eq!(
            liquid_reaction(BLOCKS.get("minecraft:stone")),
            LiquidReaction::Blocking
        );
    }
}
