//! Chargeur de recettes vanilla depuis `data/recipes/vanilla.json`
//! (consolidé à partir de `.reference/bedrock-samples/behavior_pack/recipes/`,
//! Mojang officiel 1.26.10.4).
//!
//! 3 variantes supportées : `minecraft:recipe_shaped` (939),
//! `minecraft:recipe_shapeless` (513), `minecraft:recipe_furnace` (149).
//! Brewing et smithing sont skipped (no-op).
//!
//! Les `key` et `ingredients` peuvent référencer des **tags**
//! (`minecraft:planks`, `minecraft:logs`, etc.) — résolus ici via
//! une table statique construite à partir de la connaissance vanilla.

use std::collections::HashMap;

use mc_rs_proto::packets::player::ItemStack;
use serde::Deserialize;
use tracing::warn;

use crate::crafting::{CraftingManager, FurnaceRecipe, RecipeIngredient, ShapedRecipe, ShapelessRecipe};
use crate::item_registry;

const VANILLA_JSON: &str = include_str!("../data/recipes/vanilla.json");

#[derive(Deserialize, Debug, Clone)]
struct RawItemOrTag {
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    data: Option<i32>,
    #[serde(default)]
    count: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawResult {
    item: String,
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<i32>,
    #[serde(default)]
    count: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum ResultField {
    One(RawResult),
    Many(Vec<RawResult>),
}

impl ResultField {
    fn first(&self) -> Option<&RawResult> {
        match self {
            ResultField::One(r) => Some(r),
            ResultField::Many(v) => v.first(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct RawShaped {
    pattern: Vec<String>,
    key: HashMap<String, RawItemOrTag>,
    result: ResultField,
}

#[derive(Deserialize, Debug, Clone)]
struct RawShapeless {
    ingredients: Vec<RawItemOrTag>,
    result: ResultField,
}

#[derive(Deserialize, Debug, Clone)]
struct RawFurnace {
    input: serde_json::Value, // string ou { "item": "..." }
    output: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
struct AllRecipes {
    shaped: Vec<RawShaped>,
    shapeless: Vec<RawShapeless>,
    furnace: Vec<RawFurnace>,
}

/// Résolution des 10 tags effectivement utilisés dans les recipes vanilla.
fn resolve_tag(tag: &str) -> Vec<&'static str> {
    match tag {
        "minecraft:planks" => vec![
            "minecraft:oak_planks",
            "minecraft:spruce_planks",
            "minecraft:birch_planks",
            "minecraft:jungle_planks",
            "minecraft:acacia_planks",
            "minecraft:dark_oak_planks",
            "minecraft:mangrove_planks",
            "minecraft:cherry_planks",
            "minecraft:pale_oak_planks",
            "minecraft:bamboo_planks",
            "minecraft:crimson_planks",
            "minecraft:warped_planks",
        ],
        "minecraft:logs" => vec![
            "minecraft:oak_log",
            "minecraft:spruce_log",
            "minecraft:birch_log",
            "minecraft:jungle_log",
            "minecraft:acacia_log",
            "minecraft:dark_oak_log",
            "minecraft:mangrove_log",
            "minecraft:cherry_log",
            "minecraft:pale_oak_log",
            "minecraft:bamboo_block",
            "minecraft:crimson_stem",
            "minecraft:warped_stem",
            "minecraft:stripped_oak_log",
            "minecraft:stripped_spruce_log",
            "minecraft:stripped_birch_log",
            "minecraft:stripped_jungle_log",
            "minecraft:stripped_acacia_log",
            "minecraft:stripped_dark_oak_log",
            "minecraft:stripped_mangrove_log",
            "minecraft:stripped_cherry_log",
            "minecraft:stripped_pale_oak_log",
            "minecraft:stripped_bamboo_block",
            "minecraft:stripped_crimson_stem",
            "minecraft:stripped_warped_stem",
        ],
        "minecraft:coals" => vec!["minecraft:coal", "minecraft:charcoal"],
        "minecraft:metal_nuggets" => {
            vec!["minecraft:iron_nugget", "minecraft:gold_nugget"]
        }
        "minecraft:wool" => vec![
            "minecraft:white_wool",
            "minecraft:orange_wool",
            "minecraft:magenta_wool",
            "minecraft:light_blue_wool",
            "minecraft:yellow_wool",
            "minecraft:lime_wool",
            "minecraft:pink_wool",
            "minecraft:gray_wool",
            "minecraft:light_gray_wool",
            "minecraft:cyan_wool",
            "minecraft:purple_wool",
            "minecraft:blue_wool",
            "minecraft:brown_wool",
            "minecraft:green_wool",
            "minecraft:red_wool",
            "minecraft:black_wool",
        ],
        "minecraft:egg" => vec!["minecraft:egg", "minecraft:brown_egg", "minecraft:blue_egg"],
        "minecraft:wooden_slabs" => vec![
            "minecraft:oak_slab",
            "minecraft:spruce_slab",
            "minecraft:birch_slab",
            "minecraft:jungle_slab",
            "minecraft:acacia_slab",
            "minecraft:dark_oak_slab",
            "minecraft:mangrove_slab",
            "minecraft:cherry_slab",
            "minecraft:pale_oak_slab",
            "minecraft:bamboo_slab",
            "minecraft:bamboo_mosaic_slab",
            "minecraft:crimson_slab",
            "minecraft:warped_slab",
        ],
        "minecraft:stone_crafting_materials" => vec![
            "minecraft:cobblestone",
            "minecraft:cobbled_deepslate",
        ],
        "minecraft:stone_tool_materials" => vec![
            "minecraft:cobblestone",
            "minecraft:cobbled_deepslate",
            "minecraft:blackstone",
        ],
        "minecraft:soul_fire_base_blocks" => {
            vec!["minecraft:soul_sand", "minecraft:soul_soil"]
        }
        _ => Vec::new(),
    }
}

fn build_ingredient(raw: &RawItemOrTag) -> Option<RecipeIngredient> {
    let count = raw.count.unwrap_or(1) as u16;
    if let Some(item_name) = &raw.item {
        let id = item_registry::network_id(item_name)?;
        if let Some(meta) = raw.data {
            return Some(RecipeIngredient::Exact {
                item_id: id,
                meta: meta.max(0) as u32,
                count,
            });
        }
        return Some(RecipeIngredient::AnyMeta { item_id: id, count });
    }
    if let Some(tag) = &raw.tag {
        // Pour un match tag, prend le premier membre qui existe dans le registry
        // en AnyMeta. (Approx : on ne supporte pas vraiment "matcher N'IMPORTE
        // QUEL plank" sans boucler ; on prend le plus courant.)
        for name in resolve_tag(tag) {
            if let Some(id) = item_registry::network_id(name) {
                return Some(RecipeIngredient::AnyMeta { item_id: id, count });
            }
        }
        return None;
    }
    None
}

fn build_result(raw: &RawResult) -> Option<Vec<ItemStack>> {
    let id = item_registry::network_id(&raw.item)?;
    let count = raw.count.unwrap_or(1).min(64) as u16;
    Some(vec![ItemStack::new(id, count, 0)])
}

fn furnace_item_name(v: &serde_json::Value) -> Option<&str> {
    match v {
        serde_json::Value::String(s) => Some(s.as_str()),
        serde_json::Value::Object(map) => {
            map.get("item").and_then(|v| v.as_str())
        }
        _ => None,
    }
}

fn register_shaped(mgr: &mut CraftingManager, raw: &RawShaped) -> bool {
    let height = raw.pattern.len();
    if height == 0 || height > 3 {
        return false;
    }
    let width = raw.pattern.iter().map(|row| row.chars().count()).max().unwrap_or(0);
    if width == 0 || width > 3 {
        return false;
    }

    let mut input = Vec::with_capacity(width * height);
    for row in &raw.pattern {
        let chars: Vec<char> = row.chars().collect();
        for x in 0..width {
            let ing = if x >= chars.len() || chars[x] == ' ' {
                RecipeIngredient::Empty
            } else {
                let key = chars[x].to_string();
                match raw.key.get(&key) {
                    Some(k) => match build_ingredient(k) {
                        Some(i) => i,
                        None => return false,
                    },
                    None => return false,
                }
            };
            input.push(ing);
        }
    }

    let Some(result) = raw.result.first().and_then(build_result) else {
        return false;
    };

    mgr.register_shaped(ShapedRecipe::new(width, height, input, result));
    true
}

fn register_shapeless(mgr: &mut CraftingManager, raw: &RawShapeless) -> bool {
    let mut ingredients = Vec::with_capacity(raw.ingredients.len());
    for ing in &raw.ingredients {
        match build_ingredient(ing) {
            Some(i) => ingredients.push(i),
            None => return false,
        }
    }
    let Some(result) = raw.result.first().and_then(build_result) else {
        return false;
    };
    mgr.register_shapeless(ShapelessRecipe::new(ingredients, result));
    true
}

fn register_furnace_recipe(mgr: &mut CraftingManager, raw: &RawFurnace) -> bool {
    let Some(input_name) = furnace_item_name(&raw.input) else {
        return false;
    };
    let Some(output_name) = furnace_item_name(&raw.output) else {
        return false;
    };
    let Some(in_id) = item_registry::network_id(input_name) else {
        return false;
    };
    let Some(out_id) = item_registry::network_id(output_name) else {
        return false;
    };
    mgr.register_furnace(FurnaceRecipe {
        input: RecipeIngredient::AnyMeta {
            item_id: in_id,
            count: 1,
        },
        output: ItemStack::new(out_id, 1, 0),
        cook_time_ticks: 200,
        xp: 0.1,
    });
    true
}

/// Charge toutes les recipes vanilla dans le CraftingManager.
/// Retourne (shaped, shapeless, furnace) avec le nombre effectivement registré.
pub fn register_all(mgr: &mut CraftingManager) -> (usize, usize, usize) {
    let all: AllRecipes = match serde_json::from_str(VANILLA_JSON) {
        Ok(v) => v,
        Err(e) => {
            warn!("failed to parse vanilla recipes: {}", e);
            return (0, 0, 0);
        }
    };

    let mut ok_shaped = 0;
    let mut ok_shapeless = 0;
    let mut ok_furnace = 0;

    for r in &all.shaped {
        if register_shaped(mgr, r) {
            ok_shaped += 1;
        }
    }
    for r in &all.shapeless {
        if register_shapeless(mgr, r) {
            ok_shapeless += 1;
        }
    }
    for r in &all.furnace {
        if register_furnace_recipe(mgr, r) {
            ok_furnace += 1;
        }
    }

    (ok_shaped, ok_shapeless, ok_furnace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_registers_recipes() {
        let mut mgr = CraftingManager::new();
        let (s, l, f) = register_all(&mut mgr);
        assert!(s > 500, "shaped: {}", s);
        assert!(l > 200, "shapeless: {}", l);
        assert!(f > 100, "furnace: {}", f);
    }
}
