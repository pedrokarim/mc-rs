//! Chargeur de recettes vanilla depuis `data/recipes/vanilla.json`
//! (consolidé à partir de `.reference/bedrock-samples/behavior_pack/recipes/`,
//! Mojang officiel 1.26.40.5, par `scripts/import_recipes.py v1.26.40.05`).
//!
//! 3 variantes supportées : `minecraft:recipe_shaped` (965),
//! `minecraft:recipe_shapeless` (557), `minecraft:recipe_furnace` (149).
//! Brewing et smithing sont skipped (no-op).
//!
//! Les `key` et `ingredients` peuvent référencer des **tags**
//! (`minecraft:planks`, `minecraft:logs`, etc.) — résolus ici via
//! une table statique construite à partir de la connaissance vanilla.

use std::collections::{BTreeMap, HashMap};

use mc_rs_proto::packets::player::ItemStack;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::crafting::{
    CraftingManager, FurnaceRecipe, RecipeIngredient, ShapedRecipe, ShapelessRecipe,
};
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

#[derive(Deserialize, Debug, Clone, Default)]
struct RawDescription {
    #[serde(default)]
    identifier: String,
}

#[derive(Deserialize, Debug, Clone)]
struct RawShaped {
    #[serde(default)]
    description: RawDescription,
    pattern: Vec<String>,
    key: HashMap<String, RawItemOrTag>,
    result: ResultField,
}

#[derive(Deserialize, Debug, Clone)]
struct RawShapeless {
    #[serde(default)]
    description: RawDescription,
    ingredients: Vec<RawItemOrTag>,
    result: ResultField,
}

#[derive(Deserialize, Debug, Clone)]
struct RawFurnace {
    #[serde(default)]
    description: RawDescription,
    input: serde_json::Value, // string ou { "item": "..." }
    output: serde_json::Value,
    /// Blocs autorisés (`furnace`, `blast_furnace`, `smoker`, `campfire`,
    /// `soul_campfire`, `deprecated`). Détermine le routage de la recette.
    #[serde(default)]
    tags: Vec<String>,
}

/// Raison pour laquelle une recette vanilla n'est pas enregistrée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkipReason {
    /// Item absent de la table d'items (souvent un alias legacy Mojang,
    /// `minecraft:emptymap`…, dans une recette `deprecated`).
    UnknownItem(String),
    /// Tag sans membre connu (tag non résolu par `resolve_tag`).
    EmptyTag(String),
    /// Ingrédient sans `item` ni `tag`.
    EmptyIngredient,
    /// Symbole du motif absent de `key`.
    UnknownKey(String),
    /// Motif vide ou plus grand que 3×3.
    BadPattern,
    /// Recette de cuisson sans bloc de cuisson géré (`campfire`, `deprecated`…).
    NoFurnaceBlock,
}

impl SkipReason {
    /// Catégorie, pour le résumé du chargement.
    fn category(&self) -> &'static str {
        match self {
            Self::UnknownItem(_) => "item inconnu",
            Self::EmptyTag(_) => "tag vide",
            Self::EmptyIngredient => "ingrédient vide",
            Self::UnknownKey(_) => "symbole absent",
            Self::BadPattern => "motif invalide",
            Self::NoFurnaceBlock => "cuisson non gérée",
        }
    }
}

/// Bilan du chargement des recettes vanilla.
pub struct RegisterReport {
    pub shaped: usize,
    pub shapeless: usize,
    pub furnace: usize,
    /// `(identifiant de recette, raison)` des recettes ignorées.
    pub skipped: Vec<(String, SkipReason)>,
}

#[derive(Deserialize, Debug, Clone)]
struct AllRecipes {
    shaped: Vec<RawShaped>,
    shapeless: Vec<RawShapeless>,
    furnace: Vec<RawFurnace>,
}

/// Résolution des tags utilisés comme ingrédients dans les recettes vanilla.
/// (`minecraft:is_pickaxe` n'apparaît que dans une condition de déblocage,
/// sans effet : les recettes sont envoyées déjà débloquées.)
fn resolve_tag(tag: &str) -> Vec<&'static str> {
    match tag {
        // Recette `furnace_log` (bûches qui brûlent : sans tiges du Nether ni
        // bambou). Liste identique dans dragonfly
        // `server/item/recipe/item_tags.json` et Nukkit `item_tags.json`.
        "minecraft:logs_that_burn" => vec![
            "minecraft:acacia_log",
            "minecraft:acacia_wood",
            "minecraft:birch_log",
            "minecraft:birch_wood",
            "minecraft:cherry_log",
            "minecraft:cherry_wood",
            "minecraft:dark_oak_log",
            "minecraft:dark_oak_wood",
            "minecraft:jungle_log",
            "minecraft:jungle_wood",
            "minecraft:mangrove_log",
            "minecraft:mangrove_wood",
            "minecraft:oak_log",
            "minecraft:oak_wood",
            "minecraft:pale_oak_log",
            "minecraft:pale_oak_wood",
            "minecraft:spruce_log",
            "minecraft:spruce_wood",
            "minecraft:stripped_acacia_log",
            "minecraft:stripped_acacia_wood",
            "minecraft:stripped_birch_log",
            "minecraft:stripped_birch_wood",
            "minecraft:stripped_cherry_log",
            "minecraft:stripped_cherry_wood",
            "minecraft:stripped_dark_oak_log",
            "minecraft:stripped_dark_oak_wood",
            "minecraft:stripped_jungle_log",
            "minecraft:stripped_jungle_wood",
            "minecraft:stripped_mangrove_log",
            "minecraft:stripped_mangrove_wood",
            "minecraft:stripped_oak_log",
            "minecraft:stripped_oak_wood",
            "minecraft:stripped_pale_oak_log",
            "minecraft:stripped_pale_oak_wood",
            "minecraft:stripped_spruce_log",
            "minecraft:stripped_spruce_wood",
        ],
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
        "minecraft:stone_crafting_materials" => {
            vec!["minecraft:cobblestone", "minecraft:cobbled_deepslate"]
        }
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

/// Résout un nom d'item des données Mojang : espace de noms implicite, meta
/// en suffixe legacy (`minecraft:coal:1`) ou dans le champ `data`, puis
/// migration des identifiants legacy (`item_upgrade`, port de PMMP
/// `ItemIdMetaUpgrader`). Retourne l'ID réseau et la meta à exiger (`None` :
/// toute meta). Une meta consommée par un remappage désigne un item à part
/// entière (`coal:1` → `charcoal`) : elle n'est alors plus exigée.
fn resolve_item(name: &str, data: Option<i32>) -> Result<(i32, Option<i32>), SkipReason> {
    let (id, suffix_meta) = crate::item_upgrade::parse_item_name(name);
    let requested = data.or(suffix_meta);
    let (new_id, new_meta) = crate::item_upgrade::upgrade(&id, requested.unwrap_or(0));
    let network_id = item_registry::network_id(&new_id)
        .ok_or_else(|| SkipReason::UnknownItem(name.to_string()))?;
    let meta = match requested {
        Some(meta) if meta != 0 && new_meta == 0 => None,
        Some(_) => Some(new_meta),
        None => None,
    };
    Ok((network_id, meta))
}

fn build_ingredient(raw: &RawItemOrTag) -> Result<RecipeIngredient, SkipReason> {
    let count = raw.count.unwrap_or(1) as u16;
    if let Some(item_name) = &raw.item {
        let (id, meta) = resolve_item(item_name, raw.data)?;
        return Ok(match meta {
            Some(meta) => RecipeIngredient::Exact {
                item_id: id,
                meta: meta.max(0) as u32,
                count,
            },
            None => RecipeIngredient::AnyMeta { item_id: id, count },
        });
    }
    if let Some(tag) = &raw.tag {
        // Résout tous les membres du tag présents dans le registry. Le tag est
        // conservé end-to-end : `members` sert au matching serveur (n'importe
        // quel membre matche), `tag` est renvoyé tel quel au client via
        // `TagItemDescriptor` dans le CraftingData (fidèle à Bedrock officiel).
        let members: Vec<i32> = resolve_tag(tag)
            .into_iter()
            .filter_map(item_registry::network_id)
            .collect();
        if members.is_empty() {
            return Err(SkipReason::EmptyTag(tag.clone()));
        }
        return Ok(RecipeIngredient::Tag {
            tag: tag.clone(),
            members,
            count,
        });
    }
    Err(SkipReason::EmptyIngredient)
}

fn build_result(raw: &RawResult) -> Result<Vec<ItemStack>, SkipReason> {
    let (id, meta) = resolve_item(&raw.item, raw.data)?;
    let count = raw.count.unwrap_or(1).min(64) as u16;
    let mut stack = ItemStack::new(id, count, 0);
    stack.meta = meta.unwrap_or(0).max(0) as u32;
    Ok(vec![stack])
}

fn first_result(field: &ResultField) -> Result<Vec<ItemStack>, SkipReason> {
    field
        .first()
        .ok_or(SkipReason::EmptyIngredient)
        .and_then(build_result)
}

fn furnace_item_name(v: &serde_json::Value) -> Option<&str> {
    match v {
        serde_json::Value::String(s) => Some(s.as_str()),
        serde_json::Value::Object(map) => map.get("item").and_then(|v| v.as_str()),
        _ => None,
    }
}

fn register_shaped(mgr: &mut CraftingManager, raw: &RawShaped) -> Result<(), SkipReason> {
    let height = raw.pattern.len();
    if height == 0 || height > 3 {
        return Err(SkipReason::BadPattern);
    }
    let width = raw
        .pattern
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    if width == 0 || width > 3 {
        return Err(SkipReason::BadPattern);
    }

    let mut input = Vec::with_capacity(width * height);
    for row in &raw.pattern {
        let chars: Vec<char> = row.chars().collect();
        for x in 0..width {
            let ing = if x >= chars.len() || chars[x] == ' ' {
                RecipeIngredient::Empty
            } else {
                let key = chars[x].to_string();
                let raw_ing = raw
                    .key
                    .get(&key)
                    .ok_or_else(|| SkipReason::UnknownKey(key.clone()))?;
                build_ingredient(raw_ing)?
            };
            input.push(ing);
        }
    }

    let result = first_result(&raw.result)?;
    mgr.register_shaped(ShapedRecipe::new(width, height, input, result));
    Ok(())
}

fn register_shapeless(mgr: &mut CraftingManager, raw: &RawShapeless) -> Result<(), SkipReason> {
    let ingredients = raw
        .ingredients
        .iter()
        .map(build_ingredient)
        .collect::<Result<Vec<_>, _>>()?;
    let result = first_result(&raw.result)?;
    mgr.register_shapeless(ShapelessRecipe::new(ingredients, result));
    Ok(())
}

/// Entrée d'une recette de cuisson : un item (chaîne ou `{ "item" }`) ou un
/// tag (`{ "tag" }`, ex. `minecraft:logs_that_burn` de `furnace_log`).
fn furnace_input(v: &serde_json::Value) -> Result<RecipeIngredient, SkipReason> {
    if let Some(tag) = v.get("tag").and_then(|t| t.as_str()) {
        let members: Vec<i32> = resolve_tag(tag)
            .into_iter()
            .filter_map(item_registry::network_id)
            .collect();
        if members.is_empty() {
            return Err(SkipReason::EmptyTag(tag.to_string()));
        }
        return Ok(RecipeIngredient::Tag {
            tag: tag.to_string(),
            members,
            count: 1,
        });
    }
    let name = furnace_item_name(v).ok_or(SkipReason::EmptyIngredient)?;
    let (item_id, _) = resolve_item(name, None)?;
    Ok(RecipeIngredient::AnyMeta { item_id, count: 1 })
}

fn register_furnace_recipe(mgr: &mut CraftingManager, raw: &RawFurnace) -> Result<(), SkipReason> {
    let input = furnace_input(&raw.input)?;
    let output_name = furnace_item_name(&raw.output).ok_or(SkipReason::EmptyIngredient)?;
    let (out_id, _) = resolve_item(output_name, None)?;

    // Routage par tags (cf. données Mojang). Une recette taguée `blast_furnace`
    // ET `furnace` est enregistrée dans les deux vecs : elle fond dans les deux
    // blocs. Les recettes uniquement `deprecated` (items legacy) sont ignorées.
    let to_furnace = raw.tags.iter().any(|t| t == "furnace");
    let to_blast = raw.tags.iter().any(|t| t == "blast_furnace");
    let to_smoker = raw.tags.iter().any(|t| t == "smoker");
    if !to_furnace && !to_blast && !to_smoker {
        return Err(SkipReason::NoFurnaceBlock);
    }

    // Fabrique une recette fraîche (les vecs ne partagent pas l'instance).
    let make = || FurnaceRecipe {
        input: input.clone(),
        output: ItemStack::new(out_id, 1, 0),
        cook_time_ticks: 200,
        xp: 0.1,
    };
    if to_furnace {
        mgr.register_furnace(make());
    }
    if to_blast {
        mgr.register_blast(make());
    }
    if to_smoker {
        mgr.register_smoker(make());
    }
    Ok(())
}

/// Charge toutes les recipes vanilla dans le CraftingManager et journalise
/// les recettes ignorées (résumé en info, détail en debug).
/// Retourne (shaped, shapeless, furnace) avec le nombre effectivement registré.
pub fn register_all(mgr: &mut CraftingManager) -> (usize, usize, usize) {
    let report = register_all_with_report(mgr);
    if !report.skipped.is_empty() {
        let mut by_category: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, reason) in &report.skipped {
            *by_category.entry(reason.category()).or_default() += 1;
        }
        let summary = by_category
            .iter()
            .map(|(category, n)| format!("{n} {category}"))
            .collect::<Vec<_>>()
            .join(", ");
        info!(
            "Vanilla recipes skipped: {} ({})",
            report.skipped.len(),
            summary
        );
        for (identifier, reason) in &report.skipped {
            debug!("Vanilla recipe {} skipped: {:?}", identifier, reason);
        }
    }
    (report.shaped, report.shapeless, report.furnace)
}

/// Comme [`register_all`], avec la liste des recettes ignorées.
pub fn register_all_with_report(mgr: &mut CraftingManager) -> RegisterReport {
    let mut report = RegisterReport {
        shaped: 0,
        shapeless: 0,
        furnace: 0,
        skipped: Vec::new(),
    };
    let all: AllRecipes = match serde_json::from_str(VANILLA_JSON) {
        Ok(v) => v,
        Err(e) => {
            warn!("failed to parse vanilla recipes: {}", e);
            return report;
        }
    };

    for r in &all.shaped {
        match register_shaped(mgr, r) {
            Ok(()) => report.shaped += 1,
            Err(reason) => report
                .skipped
                .push((r.description.identifier.clone(), reason)),
        }
    }
    for r in &all.shapeless {
        match register_shapeless(mgr, r) {
            Ok(()) => report.shapeless += 1,
            Err(reason) => report
                .skipped
                .push((r.description.identifier.clone(), reason)),
        }
    }
    for r in &all.furnace {
        match register_furnace_recipe(mgr, r) {
            Ok(()) => report.furnace += 1,
            Err(reason) => report
                .skipped
                .push((r.description.identifier.clone(), reason)),
        }
    }
    report
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

    #[test]
    fn skipped_recipes_report() {
        let mut mgr = CraftingManager::new();
        let report = register_all_with_report(&mut mgr);
        let mut counts: BTreeMap<SkipReason, usize> = BTreeMap::new();
        for (_, reason) in &report.skipped {
            *counts.entry(reason.clone()).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        for (reason, n) in sorted.iter().take(40) {
            println!("{n:4} {reason:?}");
        }
        println!("total skipped: {}", report.skipped.len());
    }

    #[test]
    fn legacy_and_unprefixed_names_resolve() {
        // Nom sans espace de noms.
        assert!(resolve_item("stick", None).is_ok());
        // Suffixe de meta legacy remappé vers un item dédié : meta non exigée.
        let (charcoal, meta) = resolve_item("minecraft:coal:1", None).unwrap();
        assert_eq!(
            item_registry::item_name_by_id(charcoal),
            Some("minecraft:charcoal")
        );
        assert_eq!(meta, None);
        // Renommage legacy.
        let (chain, _) = resolve_item("minecraft:chain", None).unwrap();
        assert_eq!(
            item_registry::item_name_by_id(chain),
            Some("minecraft:iron_chain")
        );
    }
}
