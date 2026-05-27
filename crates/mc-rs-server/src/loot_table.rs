//! Loot tables vanilla — chargées depuis `data/loot_tables/entities.json`
//! (généré à partir de `.reference/bedrock-samples/behavior_pack/loot_tables/entities/`,
//! Mojang officiel 1.26.10.4).
//!
//! Format source (Mojang) :
//! ```json
//! {
//!   "pools": [{
//!     "rolls": 1 | { "min": 0, "max": 2 },
//!     "conditions": [{ "condition": "killed_by_player_or_pets" }, ...],
//!     "entries": [{
//!       "type": "item" | "empty" | "loot_table",
//!       "name": "minecraft:rotten_flesh",
//!       "weight": 1,
//!       "functions": [{ "function": "set_count", "count": { "min": 0, "max": 2 } }]
//!     }]
//!   }]
//! }
//! ```

use std::collections::HashMap;
use std::sync::LazyLock;

use rand::Rng;
use serde::Deserialize;

const ENTITIES_JSON: &str = include_str!("../data/loot_tables/entities.json");
const CHESTS_JSON: &str = include_str!("../data/loot_tables/chests.json");
const EQUIPMENT_JSON: &str = include_str!("../data/loot_tables/equipment.json");
const GAMEPLAY_JSON: &str = include_str!("../data/loot_tables/gameplay.json");
const DISPENSERS_JSON: &str = include_str!("../data/loot_tables/dispensers.json");
const POTS_JSON: &str = include_str!("../data/loot_tables/pots.json");
const SPAWNERS_JSON: &str = include_str!("../data/loot_tables/spawners.json");

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum IntOrRange {
    Int(i32),
    Float(f32),
    Range { min: f32, max: f32 },
}

impl IntOrRange {
    fn roll<R: Rng>(&self, rng: &mut R) -> i32 {
        match self {
            IntOrRange::Int(v) => *v,
            IntOrRange::Float(v) => v.round() as i32,
            IntOrRange::Range { min, max } => {
                let min_i = min.round() as i32;
                let max_i = max.round() as i32;
                if max_i >= min_i {
                    rng.gen_range(min_i..=max_i)
                } else {
                    min_i
                }
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RawFunction {
    function: String,
    #[serde(default)]
    count: Option<IntOrRange>,
    #[serde(default)]
    levels: Option<IntOrRange>,
    #[serde(default)]
    damage: Option<IntOrRange>,
    #[serde(default)]
    data: Option<IntOrRange>,
    #[serde(default)]
    values: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct RawCondition {
    condition: String,
    #[serde(default)]
    chance: Option<f32>,
    #[serde(default)]
    looting_multiplier: Option<f32>,
    #[serde(default)]
    entity_type: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawEntry {
    #[serde(rename = "type", default)]
    entry_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    weight: Option<i32>,
    #[serde(default)]
    count: Option<IntOrRange>,
    #[serde(default)]
    functions: Vec<RawFunction>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawPool {
    #[serde(default)]
    rolls: Option<IntOrRange>,
    #[serde(default)]
    conditions: Vec<RawCondition>,
    #[serde(default)]
    entries: Vec<RawEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawLootTable {
    #[serde(default)]
    pools: Vec<RawPool>,
}

/// Contexte d'évaluation — booleans passés par l'appelant pour les conditions.
#[derive(Debug, Clone, Copy, Default)]
pub struct LootContext {
    pub killed_by_player: bool,
    pub killed_by_pet: bool,
    pub is_baby: bool,
    pub on_fire: bool,
    pub looting_level: u32,
}

/// Résultat d'un roll — liste d'(item_name, count).
pub type LootDrops = Vec<(String, u32)>;

static ENTITY_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(ENTITIES_JSON).expect("valid entities.json loot data"));

static CHEST_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(CHESTS_JSON).expect("valid chests.json loot data"));

static EQUIPMENT_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(EQUIPMENT_JSON).expect("valid equipment.json loot data"));

static GAMEPLAY_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(GAMEPLAY_JSON).expect("valid gameplay.json loot data"));

static DISPENSERS_LOOT: LazyLock<HashMap<String, RawLootTable>> = LazyLock::new(|| {
    serde_json::from_str(DISPENSERS_JSON).expect("valid dispensers.json loot data")
});

static POTS_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(POTS_JSON).expect("valid pots.json loot data"));

static SPAWNERS_LOOT: LazyLock<HashMap<String, RawLootTable>> =
    LazyLock::new(|| serde_json::from_str(SPAWNERS_JSON).expect("valid spawners.json loot data"));

fn check_condition<R: Rng>(cond: &RawCondition, ctx: &LootContext, rng: &mut R) -> bool {
    match cond.condition.as_str() {
        "killed_by_player" => ctx.killed_by_player,
        "killed_by_player_or_pets" => ctx.killed_by_player || ctx.killed_by_pet,
        "is_baby" => ctx.is_baby,
        "on_fire" | "entity_on_fire" => ctx.on_fire,
        "random_chance" => {
            let chance = cond.chance.unwrap_or(0.0);
            rng.gen::<f32>() < chance
        }
        "random_chance_with_looting" => {
            let chance = cond.chance.unwrap_or(0.0);
            let mult = cond.looting_multiplier.unwrap_or(0.0);
            let effective = chance + mult * ctx.looting_level as f32;
            rng.gen::<f32>() < effective
        }
        "random_difficulty_chance" | "random_regional_difficulty_chance" => {
            // Approx : chance de 0.1 (normal difficulty).
            let chance = cond.chance.unwrap_or(0.1);
            rng.gen::<f32>() < chance
        }
        // Conditions non-supportées (passenger_of_entity, damaged_by_entity,
        // killed_by_entity, entity_killed, bool_property, has_variant) → false
        // (le pool ne rolle pas, ce qui est sûr par défaut).
        _ => false,
    }
}

fn apply_functions<R: Rng>(
    funcs: &[RawFunction],
    item_name: &str,
    base_count: u32,
    ctx: &LootContext,
    rng: &mut R,
) -> u32 {
    let mut count = base_count;
    for f in funcs {
        match f.function.as_str() {
            "set_count" => {
                if let Some(c) = &f.count {
                    let v = c.roll(rng);
                    count = v.max(0) as u32;
                }
            }
            "looting_enchant" => {
                if let Some(c) = &f.count {
                    let bonus = c.roll(rng) * ctx.looting_level as i32;
                    count = ((count as i32) + bonus).max(0) as u32;
                }
            }
            // Non-supportées : furnace_smelt, enchant_random_gear,
            // enchant_with_levels, specific_enchants, random_aux_value,
            // set_data, set_damage, set_banner_details, set_potion,
            // set_ominous_bottle_amplifier, set_data_from_color_index,
            // minecraft:set_*. On garde l'item + count tels quels.
            _ => {
                let _ = item_name; // silence
            }
        }
    }
    count
}

fn pick_weighted_entry<'a, R: Rng>(entries: &'a [RawEntry], rng: &mut R) -> Option<&'a RawEntry> {
    let total: i32 = entries.iter().map(|e| e.weight.unwrap_or(1).max(0)).sum();
    if total <= 0 {
        return None;
    }
    let mut roll = rng.gen_range(0..total);
    for entry in entries {
        let w = entry.weight.unwrap_or(1).max(0);
        if roll < w {
            return Some(entry);
        }
        roll -= w;
    }
    entries.last()
}

/// Rolle la loot table pour une entité (format `minecraft:zombie`).
/// Retourne la liste de drops.
pub fn roll_entity_loot(entity_name: &str, ctx: LootContext) -> LootDrops {
    let mut rng = rand::thread_rng();
    let Some(table) = ENTITY_LOOT.get(entity_name) else {
        return Vec::new();
    };

    let mut drops: LootDrops = Vec::new();
    for pool in &table.pools {
        // Check conditions du pool.
        let mut pool_ok = true;
        for c in &pool.conditions {
            if !check_condition(c, &ctx, &mut rng) {
                pool_ok = false;
                break;
            }
        }
        if !pool_ok {
            continue;
        }

        // Nombre de rolls.
        let rolls = pool.rolls.as_ref().map(|r| r.roll(&mut rng)).unwrap_or(1);
        for _ in 0..rolls.max(0) {
            let Some(entry) = pick_weighted_entry(&pool.entries, &mut rng) else {
                continue;
            };
            if entry.entry_type != "item" {
                continue;
            }
            let Some(name) = entry.name.as_deref() else {
                continue;
            };
            let base_count = entry
                .count
                .as_ref()
                .map(|c| c.roll(&mut rng).max(0) as u32)
                .unwrap_or(1);
            let final_count = apply_functions(&entry.functions, name, base_count, &ctx, &mut rng);
            if final_count == 0 {
                continue;
            }
            drops.push((name.to_string(), final_count));
        }
    }

    drops
}

pub fn has_loot_table(entity_name: &str) -> bool {
    ENTITY_LOOT.contains_key(entity_name)
}

pub fn entity_count() -> usize {
    ENTITY_LOOT.len()
}

/// Rolle la loot table d'un coffre vanilla (ex. `minecraft:simple_dungeon`).
pub fn roll_chest_loot(chest_name: &str) -> LootDrops {
    let mut rng = rand::thread_rng();
    let Some(table) = CHEST_LOOT.get(chest_name) else {
        return Vec::new();
    };

    let ctx = LootContext {
        killed_by_player: true, // coffres : conditions joueur toujours true
        ..Default::default()
    };
    let mut drops: LootDrops = Vec::new();
    for pool in &table.pools {
        let mut pool_ok = true;
        for c in &pool.conditions {
            if !check_condition(c, &ctx, &mut rng) {
                pool_ok = false;
                break;
            }
        }
        if !pool_ok {
            continue;
        }
        let rolls = pool.rolls.as_ref().map(|r| r.roll(&mut rng)).unwrap_or(1);
        for _ in 0..rolls.max(0) {
            let Some(entry) = pick_weighted_entry(&pool.entries, &mut rng) else {
                continue;
            };
            if entry.entry_type != "item" {
                continue;
            }
            let Some(name) = entry.name.as_deref() else {
                continue;
            };
            let base_count = entry
                .count
                .as_ref()
                .map(|c| c.roll(&mut rng).max(0) as u32)
                .unwrap_or(1);
            let final_count = apply_functions(&entry.functions, name, base_count, &ctx, &mut rng);
            if final_count == 0 {
                continue;
            }
            drops.push((name.to_string(), final_count));
        }
    }
    drops
}

pub fn chest_count() -> usize {
    CHEST_LOOT.len()
}

/// Roll générique — cherche la table dans toutes les catégories.
pub fn roll_any(table_name: &str) -> LootDrops {
    for dict in [
        &*ENTITY_LOOT,
        &*CHEST_LOOT,
        &*EQUIPMENT_LOOT,
        &*GAMEPLAY_LOOT,
        &*DISPENSERS_LOOT,
        &*POTS_LOOT,
        &*SPAWNERS_LOOT,
    ] {
        if dict.contains_key(table_name) {
            return roll_table_generic(dict, table_name);
        }
    }
    Vec::new()
}

fn roll_table_generic(dict: &HashMap<String, RawLootTable>, table_name: &str) -> LootDrops {
    let mut rng = rand::thread_rng();
    let Some(table) = dict.get(table_name) else {
        return Vec::new();
    };
    let ctx = LootContext {
        killed_by_player: true,
        ..Default::default()
    };
    let mut drops: LootDrops = Vec::new();
    for pool in &table.pools {
        let mut pool_ok = true;
        for c in &pool.conditions {
            if !check_condition(c, &ctx, &mut rng) {
                pool_ok = false;
                break;
            }
        }
        if !pool_ok {
            continue;
        }
        let rolls = pool.rolls.as_ref().map(|r| r.roll(&mut rng)).unwrap_or(1);
        for _ in 0..rolls.max(0) {
            let Some(entry) = pick_weighted_entry(&pool.entries, &mut rng) else {
                continue;
            };
            if entry.entry_type != "item" {
                continue;
            }
            let Some(name) = entry.name.as_deref() else {
                continue;
            };
            let base_count = entry
                .count
                .as_ref()
                .map(|c| c.roll(&mut rng).max(0) as u32)
                .unwrap_or(1);
            let final_count = apply_functions(&entry.functions, name, base_count, &ctx, &mut rng);
            if final_count == 0 {
                continue;
            }
            drops.push((name.to_string(), final_count));
        }
    }
    drops
}

pub fn total_table_count() -> usize {
    ENTITY_LOOT.len()
        + CHEST_LOOT.len()
        + EQUIPMENT_LOOT.len()
        + GAMEPLAY_LOOT.len()
        + DISPENSERS_LOOT.len()
        + POTS_LOOT.len()
        + SPAWNERS_LOOT.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_has_loot_table() {
        assert!(has_loot_table("minecraft:zombie"));
    }

    #[test]
    fn zombie_drops_rotten_flesh_eventually() {
        let ctx = LootContext {
            killed_by_player: true,
            ..Default::default()
        };
        let mut found = false;
        for _ in 0..200 {
            let drops = roll_entity_loot("minecraft:zombie", ctx);
            if drops.iter().any(|(n, _)| n == "minecraft:rotten_flesh") {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn skeleton_loot_table_exists() {
        assert!(has_loot_table("minecraft:skeleton"));
    }

    #[test]
    fn nonexistent_entity_empty_drops() {
        let drops = roll_entity_loot("minecraft:foobar", LootContext::default());
        assert!(drops.is_empty());
    }

    #[test]
    fn chest_loot_tables_load() {
        assert!(chest_count() > 20);
    }

    #[test]
    fn all_categories_loaded() {
        assert!(total_table_count() > 150);
    }

    #[test]
    fn simple_dungeon_chest_drops_something() {
        let mut found = false;
        for _ in 0..50 {
            let drops = roll_chest_loot("minecraft:simple_dungeon");
            if !drops.is_empty() {
                found = true;
                break;
            }
        }
        assert!(found);
    }
}
