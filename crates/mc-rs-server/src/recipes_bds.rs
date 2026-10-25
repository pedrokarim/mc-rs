//! Recettes de BDS, importées de son CraftingData (`data/recipes/bds.json`,
//! produit par `scripts/import_bds_recipes.py` depuis une capture ; méthode :
//! `dev-docs/34-BDS-RETRO-INGENIERIE.md`).
//!
//! Toutes les recettes que BDS envoie au client y sont, y compris celles que
//! le jeu génère par code et que bedrock-samples ne contient pas (teintures,
//! anciennes recettes, potions, forge). Chacune garde son identité BDS
//! (`RecipeInfo`). Les tags d'ingrédients sont résolus avec
//! `data/recipes/item_tags.json` (dragonfly v0.11.5, Bedrock 1.26.50).

use std::collections::HashMap;

use mc_rs_nbt::tag::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_proto::packets::player::ItemStack;
use serde::Deserialize;
use serde_json::Value;

use crate::crafting::{
    ContainerMix, CraftingManager, FurnaceRecipe, MultiRecipe, PotionMix, RecipeInfo,
    RecipeIngredient, ShapedRecipe, ShapelessRecipe, SmithingRecipe,
};
use crate::item_registry;

const BDS_RECIPES_JSON: &str = include_str!("../data/recipes/bds.json");
const ITEM_TAGS_JSON: &str = include_str!("../data/recipes/item_tags.json");

/// Meta joker d'un ingrédient (toute meta acceptée).
const WILDCARD_META: i32 = 0x7fff;

/// Durée de cuisson et expérience des recettes de four : BDS ne les envoie
/// pas ; valeurs vanilla d'un four classique, comme l'import précédent.
const COOK_TIME_TICKS: u32 = 200;
const COOK_XP: f32 = 0.1;

#[derive(Deserialize)]
struct BdsRecipes {
    source: BdsSource,
    shaped: Vec<RawShaped>,
    shapeless: Vec<RawShapeless>,
    user_data_shapeless: Vec<RawShapeless>,
    multi: Vec<RawMulti>,
    smithing_transform: Vec<RawSmithing>,
    smithing_trim: Vec<RawSmithing>,
    potion_mixes: Vec<RawPotionMix>,
    container_mixes: Vec<RawContainerMix>,
}

#[derive(Deserialize)]
struct BdsSource {
    bds_version: String,
}

#[derive(Deserialize)]
struct RawInfo {
    id: String,
    net_id: u32,
    uuid: String,
    block: String,
    priority: i32,
}

#[derive(Deserialize)]
struct RawShaped {
    #[serde(flatten)]
    info: RawInfo,
    width: usize,
    height: usize,
    assume_symmetry: bool,
    input: Vec<Option<RawIngredient>>,
    output: Vec<RawItem>,
}

#[derive(Deserialize)]
struct RawShapeless {
    #[serde(flatten)]
    info: RawInfo,
    input: Vec<Option<RawIngredient>>,
    output: Vec<RawItem>,
}

#[derive(Deserialize)]
struct RawIngredient {
    item: Option<String>,
    meta: Option<i32>,
    tag: Option<String>,
    count: u16,
}

#[derive(Deserialize)]
struct RawItem {
    item: String,
    meta: u32,
    count: u16,
    /// BDS renseigne le bloc de la sortie (items-blocs seulement : pas un lit,
    /// une porte, un panneau suspendu…).
    block: bool,
    /// NBT typé (`{"compound": {...}}`), voir `bds-capture decode -typed`.
    nbt: Option<Value>,
}

#[derive(Deserialize)]
struct RawMulti {
    uuid: String,
    net_id: u32,
}

#[derive(Deserialize)]
struct RawSmithing {
    id: String,
    net_id: u32,
    block: String,
    template: Option<RawIngredient>,
    base: Option<RawIngredient>,
    addition: Option<RawIngredient>,
    output: Option<RawItem>,
}

#[derive(Deserialize)]
struct RawItemMeta {
    item: String,
    meta: i32,
}

#[derive(Deserialize)]
struct RawPotionMix {
    input: RawItemMeta,
    reagent: RawItemMeta,
    output: RawItemMeta,
}

#[derive(Deserialize)]
struct RawContainerMix {
    input: String,
    reagent: String,
    output: String,
}

/// Bilan du chargement, pour le journal du démarrage.
#[derive(Debug, Default)]
pub struct LoadSummary {
    pub bds_version: String,
    pub shaped: usize,
    pub shapeless: usize,
    pub user_data_shapeless: usize,
    pub furnace: usize,
    pub smithing: usize,
    pub potion_mixes: usize,
    pub skipped: Vec<String>,
}

/// Contexte de conversion : tags d'items résolus en identifiants réseau.
struct Loader {
    tags: HashMap<String, Vec<i32>>,
}

impl Loader {
    fn new() -> Self {
        let raw: HashMap<String, Vec<String>> =
            serde_json::from_str(ITEM_TAGS_JSON).expect("item_tags.json valide");
        let tags = raw
            .into_iter()
            .map(|(tag, items)| {
                let ids = items
                    .iter()
                    .filter_map(|name| item_registry::network_id(name))
                    .collect();
                (tag, ids)
            })
            .collect();
        Self { tags }
    }

    fn item_id(name: &str) -> Result<i32, String> {
        item_registry::network_id(name).ok_or_else(|| format!("item inconnu : {name}"))
    }

    fn ingredient(&self, raw: Option<&RawIngredient>) -> Result<RecipeIngredient, String> {
        let Some(raw) = raw else {
            return Ok(RecipeIngredient::Empty);
        };
        if let Some(tag) = &raw.tag {
            let members = self
                .tags
                .get(tag)
                .filter(|m| !m.is_empty())
                .ok_or_else(|| format!("tag sans membre connu : {tag}"))?;
            return Ok(RecipeIngredient::Tag {
                tag: tag.clone(),
                members: members.clone(),
                count: raw.count,
            });
        }
        let name = raw.item.as_deref().ok_or("ingrédient sans item ni tag")?;
        let item_id = Self::item_id(name)?;
        Ok(match raw.meta.unwrap_or(WILDCARD_META) {
            WILDCARD_META => RecipeIngredient::AnyMeta {
                item_id,
                count: raw.count,
            },
            meta => RecipeIngredient::Exact {
                item_id,
                meta: meta.max(0) as u32,
                count: raw.count,
            },
        })
    }

    fn ingredients(&self, raw: &[Option<RawIngredient>]) -> Result<Vec<RecipeIngredient>, String> {
        raw.iter().map(|r| self.ingredient(r.as_ref())).collect()
    }

    /// Sortie de recette : item, meta, quantité, bloc d'un item-bloc et NBT
    /// (blob userData complet, comme les items de l'inventaire).
    fn output(raw: &RawItem) -> Result<ItemStack, String> {
        let id = Self::item_id(&raw.item)?;
        let block_runtime_id = if raw.block {
            crate::inventory::item_to_block(id)
                .map(|b| b as i32)
                .ok_or_else(|| format!("item-bloc sans bloc connu : {}", raw.item))?
        } else {
            0
        };
        let mut stack = ItemStack::new(id, raw.count, block_runtime_id);
        stack.meta = raw.meta;
        if let Some(nbt) = &raw.nbt {
            let NbtTag::Compound(compound) = typed_nbt(nbt)? else {
                return Err(format!("NBT de sortie non compound : {}", raw.item));
            };
            let mut nbt_le = Vec::new();
            mc_rs_nbt::write_nbt_le(
                &mut nbt_le,
                &NbtRoot {
                    name: String::new(),
                    compound,
                },
            );
            let is_shield = item_registry::network_id("minecraft:shield") == Some(id);
            stack.extra_data =
                mc_rs_proto::packets::world::item_user_data(Some(&nbt_le), is_shield);
        }
        Ok(stack)
    }

    fn outputs(raw: &[RawItem]) -> Result<Vec<ItemStack>, String> {
        raw.iter().map(Self::output).collect()
    }
}

fn info(raw: &RawInfo) -> Result<RecipeInfo, String> {
    Ok(RecipeInfo {
        id: raw.id.clone(),
        net_id: raw.net_id,
        uuid: parse_uuid(&raw.uuid)?,
        block: raw.block.clone(),
        priority: raw.priority,
    })
}

fn parse_uuid(s: &str) -> Result<[u8; 16], String> {
    uuid::Uuid::parse_str(s)
        .map(|u| *u.as_bytes())
        .map_err(|e| format!("UUID invalide {s} : {e}"))
}

/// NBT typé de `bds.json` (`{"int": 0}`, `{"list": [...]}`…) → `NbtTag`.
fn typed_nbt(v: &Value) -> Result<NbtTag, String> {
    let obj = v
        .as_object()
        .filter(|o| o.len() == 1)
        .ok_or_else(|| format!("valeur NBT typée invalide : {v}"))?;
    let (kind, val) = obj.iter().next().unwrap();
    let int = |val: &Value| {
        val.as_i64()
            .ok_or_else(|| format!("entier attendu : {val}"))
    };
    let array = |val: &Value| -> Result<Vec<i64>, String> {
        val.as_array()
            .ok_or_else(|| format!("tableau attendu : {val}"))?
            .iter()
            .map(int)
            .collect()
    };
    Ok(match kind.as_str() {
        "byte" => NbtTag::Byte(int(val)? as i8),
        "short" => NbtTag::Short(int(val)? as i16),
        "int" => NbtTag::Int(int(val)? as i32),
        "long" => NbtTag::Long(int(val)?),
        "float" => NbtTag::Float(val.as_f64().ok_or("flottant attendu")? as f32),
        "double" => NbtTag::Double(val.as_f64().ok_or("flottant attendu")?),
        "string" => NbtTag::String(val.as_str().ok_or("chaîne attendue")?.to_string()),
        "byte_array" => NbtTag::ByteArray(array(val)?.into_iter().map(|b| b as i8).collect()),
        "int_array" => NbtTag::IntArray(array(val)?.into_iter().map(|i| i as i32).collect()),
        "long_array" => NbtTag::LongArray(array(val)?),
        "list" => NbtTag::List(
            val.as_array()
                .ok_or("liste attendue")?
                .iter()
                .map(typed_nbt)
                .collect::<Result<_, _>>()?,
        ),
        "compound" => {
            let map = val.as_object().ok_or("compound attendu")?;
            let mut compound = NbtCompound::new();
            for (k, v) in map {
                compound.insert(k.clone(), typed_nbt(v)?);
            }
            NbtTag::Compound(compound)
        }
        other => return Err(format!("type NBT inconnu : {other}")),
    })
}

/// Charge toutes les recettes de BDS dans le manager. Une recette dont un
/// item ou un tag est inconnu est omise et listée dans le bilan.
pub fn register_all(mgr: &mut CraftingManager) -> LoadSummary {
    let data: BdsRecipes = serde_json::from_str(BDS_RECIPES_JSON).expect("bds.json valide");
    let loader = Loader::new();
    let mut summary = LoadSummary {
        bds_version: data.source.bds_version.clone(),
        ..LoadSummary::default()
    };

    for raw in &data.shaped {
        let recipe = (|| {
            Ok::<_, String>(ShapedRecipe {
                info: info(&raw.info)?,
                width: raw.width,
                height: raw.height,
                assume_symmetry: raw.assume_symmetry,
                input: loader.ingredients(&raw.input)?,
                output: Loader::outputs(&raw.output)?,
            })
        })();
        match recipe {
            Ok(r) => {
                mgr.register_shaped(r);
                summary.shaped += 1;
            }
            Err(e) => summary.skipped.push(format!("{} : {e}", raw.info.id)),
        }
    }

    for (raw_list, user_data) in [(&data.shapeless, false), (&data.user_data_shapeless, true)] {
        for raw in raw_list {
            let recipe = (|| {
                Ok::<_, String>(ShapelessRecipe {
                    info: info(&raw.info)?,
                    ingredients: loader.ingredients(&raw.input)?,
                    output: Loader::outputs(&raw.output)?,
                })
            })();
            let r = match recipe {
                Ok(r) => r,
                Err(e) => {
                    summary.skipped.push(format!("{} : {e}", raw.info.id));
                    continue;
                }
            };
            if user_data {
                mgr.register_user_data_shapeless(r);
                summary.user_data_shapeless += 1;
                continue;
            }
            register_cooking(mgr, &r, &mut summary);
            mgr.register_shapeless(r);
            summary.shapeless += 1;
        }
    }

    mgr.multi.extend(data.multi.iter().filter_map(|m| {
        Some(MultiRecipe {
            uuid: parse_uuid(&m.uuid).ok()?,
            net_id: m.net_id,
        })
    }));

    for (raw_list, transform) in [
        (&data.smithing_transform, true),
        (&data.smithing_trim, false),
    ] {
        for raw in raw_list {
            let recipe = (|| {
                Ok::<_, String>(SmithingRecipe {
                    id: raw.id.clone(),
                    net_id: raw.net_id,
                    block: raw.block.clone(),
                    template: loader.ingredient(raw.template.as_ref())?,
                    base: loader.ingredient(raw.base.as_ref())?,
                    addition: loader.ingredient(raw.addition.as_ref())?,
                    output: raw.output.as_ref().map(Loader::output).transpose()?,
                })
            })();
            match recipe {
                Ok(r) if transform => mgr.smithing_transform.push(r),
                Ok(r) => mgr.smithing_trim.push(r),
                Err(e) => {
                    summary.skipped.push(format!("{} : {e}", raw.id));
                    continue;
                }
            }
            summary.smithing += 1;
        }
    }

    let item_meta = |raw: &RawItemMeta| Loader::item_id(&raw.item).map(|id| (id, raw.meta));
    for raw in &data.potion_mixes {
        match (
            item_meta(&raw.input),
            item_meta(&raw.reagent),
            item_meta(&raw.output),
        ) {
            (Ok(input), Ok(reagent), Ok(output)) => {
                mgr.potion_mixes.push(PotionMix {
                    input,
                    reagent,
                    output,
                });
                summary.potion_mixes += 1;
            }
            _ => summary
                .skipped
                .push(format!("mélange {} + {}", raw.input.item, raw.reagent.item)),
        }
    }
    for raw in &data.container_mixes {
        match (
            Loader::item_id(&raw.input),
            Loader::item_id(&raw.reagent),
            Loader::item_id(&raw.output),
        ) {
            (Ok(input), Ok(reagent), Ok(output)) => mgr.container_mixes.push(ContainerMix {
                input,
                reagent,
                output,
            }),
            _ => summary
                .skipped
                .push(format!("contenant {} + {}", raw.input, raw.reagent)),
        }
    }

    summary
}

/// Une recette sans forme au bloc d'un four sert aussi à la cuisson
/// (`furnace.rs`) : une entrée, une sortie.
fn register_cooking(mgr: &mut CraftingManager, r: &ShapelessRecipe, summary: &mut LoadSummary) {
    let ([input], [output]) = (r.ingredients.as_slice(), r.output.as_slice()) else {
        return;
    };
    let recipe = FurnaceRecipe {
        input: input.clone(),
        output: output.clone(),
        cook_time_ticks: COOK_TIME_TICKS,
        xp: COOK_XP,
    };
    match r.info.block.as_str() {
        "furnace" => mgr.furnace.push(recipe),
        "blast_furnace" => mgr.blast_furnace.push(recipe),
        "smoker" => mgr.smoker.push(recipe),
        _ => return,
    }
    summary.furnace += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded() -> (CraftingManager, LoadSummary) {
        let mut mgr = CraftingManager::new();
        let summary = register_all(&mut mgr);
        (mgr, summary)
    }

    #[test]
    fn every_bds_recipe_loads() {
        let (mgr, summary) = loaded();
        assert!(summary.skipped.is_empty(), "omises : {:?}", summary.skipped);
        assert_eq!(mgr.shaped.len(), 1217);
        assert_eq!(mgr.shapeless.len(), 2068);
        assert_eq!(mgr.user_data_shapeless.len(), 1456);
        assert_eq!(mgr.multi.len(), 14);
        assert_eq!(mgr.smithing_transform.len() + mgr.smithing_trim.len(), 13);
        assert_eq!(mgr.potion_mixes.len(), 210);
        assert_eq!(mgr.container_mixes.len(), 2);
        assert!(!mgr.furnace.is_empty() && !mgr.blast_furnace.is_empty() && !mgr.smoker.is_empty());
    }

    #[test]
    fn recipes_keep_their_bds_identity() {
        let (mgr, _) = loaded();
        let barrel = mgr
            .shaped
            .iter()
            .find(|r| r.info.id == "minecraft:barrel")
            .unwrap();
        assert_eq!(barrel.info.net_id, 1126);
        assert_eq!(barrel.info.block, "deprecated");
        assert_eq!(barrel.info.priority, -1);
        // Toutes les recettes portent un identifiant réseau unique et non nul.
        let mut ids: Vec<u32> = mgr
            .shaped
            .iter()
            .map(|r| r.info.net_id)
            .chain(mgr.shapeless.iter().map(|r| r.info.net_id))
            .chain(mgr.user_data_shapeless.iter().map(|r| r.info.net_id))
            .collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
        assert!(ids[0] > 0);
    }

    #[test]
    fn firework_output_keeps_typed_nbt() {
        let (mgr, _) = loaded();
        let charge = mgr
            .shapeless
            .iter()
            .find(|r| r.info.id == "paper_sulphur_charge_0_recipeId")
            .unwrap();
        // Blob userData : marqueur -1, version 1, puis le NBT little-endian.
        let blob = &charge.output[0].extra_data;
        assert_eq!(&blob[..3], &[0xFF, 0xFF, 0x01]);
        let root = mc_rs_nbt::read_nbt_le(&mut &blob[3..]).unwrap();
        let Some(NbtTag::Compound(fireworks)) = root.compound.get("Fireworks") else {
            panic!("Fireworks absent");
        };
        assert!(matches!(fireworks.get("Flight"), Some(NbtTag::Byte(1))));
    }

    #[test]
    fn stonecutter_recipes_are_not_crafting_table_recipes() {
        let (mgr, _) = loaded();
        assert!(mgr.shapeless.iter().any(|r| r.info.block == "stonecutter"));
        // La grille d'établi ne doit jamais produire une recette de tailleur :
        // une pierre taillée seule n'a de recette qu'au tailleur (escaliers,
        // dalles, murets). Une pierre seule, elle, donne un bouton à l'établi.
        let stone = item_registry::network_id("minecraft:cobblestone").unwrap();
        let grid = vec![
            ItemStack::new(stone, 1, 0),
            ItemStack::AIR,
            ItemStack::AIR,
            ItemStack::AIR,
        ];
        assert!(mgr.match_crafting(&grid, 2).is_none());
    }
}
