//! CreativeContent — classification vanilla exacte issue de
//! `.reference/PocketMine-MP/vendor/pocketmine/bedrock-data/creative/*.json`.
//!
//! Chaque fichier JSON liste les groupes d'une catégorie :
//! ```json
//! [
//!   {
//!     "group_icon": "minecraft:oak_planks" | { "name": "...", "block_states": "base64" } | null,
//!     "group_name": "itemGroup.name.planks" | "",
//!     "items": [
//!       "minecraft:oak_planks",
//!       { "name": "minecraft:bed", "meta": 8, "block_states": "base64" }
//!     ]
//!   }, ...
//! ]
//! ```
//!
//! Un groupe `{ group_icon: null, group_name: "" }` = bac à items sans
//! sous-groupe (PMMP `CreativeGroupEntry` anonyme).
//!
//! `meta` et `block_states` sont ignorés pour l'instant — on émet l'item
//! en version "par défaut" (count=1, block_runtime_id = BLOCKS.get(name)).

use std::sync::LazyLock;

use mc_rs_proto::packets::world::{CreativeGroupEntry, CreativeItemEntry};
use serde::Deserialize;

use crate::item_registry;
use crate::world::block_registry::BLOCKS;

const CONSTRUCTION_JSON: &str = include_str!("../data/creative/construction.json");
const NATURE_JSON: &str = include_str!("../data/creative/nature.json");
const EQUIPMENT_JSON: &str = include_str!("../data/creative/equipment.json");
const ITEMS_JSON: &str = include_str!("../data/creative/items.json");

// PMMP CreativeContentPacket::CATEGORY_*
const CAT_CONSTRUCTION: i32 = 1;
const CAT_NATURE: i32 = 2;
const CAT_EQUIPMENT: i32 = 3;
const CAT_ITEMS: i32 = 4;

#[derive(Deserialize)]
#[serde(untagged)]
enum RawItem {
    Named(String),
    Full {
        name: String,
        #[serde(default)]
        #[allow(dead_code)]
        meta: Option<i32>,
        #[serde(default)]
        #[allow(dead_code)]
        block_states: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawIcon {
    Named(String),
    Full {
        name: String,
        #[allow(dead_code)]
        #[serde(default)]
        block_states: Option<String>,
    },
}

#[derive(Deserialize)]
struct RawGroup {
    group_icon: Option<RawIcon>,
    group_name: String,
    items: Vec<RawItem>,
}

impl RawItem {
    fn name(&self) -> &str {
        match self {
            RawItem::Named(s) => s.as_str(),
            RawItem::Full { name, .. } => name.as_str(),
        }
    }
}

impl RawIcon {
    fn name(&self) -> &str {
        match self {
            RawIcon::Named(s) => s.as_str(),
            RawIcon::Full { name, .. } => name.as_str(),
        }
    }
}

/// Groupes + items pré-construits au démarrage, prêts à être encodés.
pub struct CreativeData {
    pub groups: Vec<OwnedGroup>,
    pub items: Vec<OwnedItem>,
}

pub struct OwnedGroup {
    pub category_id: i32,
    pub category_name: String,
    pub icon_item_id: i32,
}

pub struct OwnedItem {
    pub entry_id: u32,
    pub item_id: i32,
    pub block_runtime_id: i32,
    pub group_id: u32,
}

static CREATIVE_DATA: LazyLock<CreativeData> = LazyLock::new(load_creative_data);

fn load_category(
    category_id: i32,
    json: &str,
    groups: &mut Vec<OwnedGroup>,
    items: &mut Vec<OwnedItem>,
    next_entry_id: &mut u32,
) {
    let raw_groups: Vec<RawGroup> =
        serde_json::from_str(json).expect("valid creative category json");

    for raw in raw_groups {
        let icon_item_id = raw
            .group_icon
            .as_ref()
            .and_then(|icon| item_registry::network_id(icon.name()))
            .unwrap_or(0);

        let group_index = groups.len() as u32;
        groups.push(OwnedGroup {
            category_id,
            category_name: raw.group_name,
            icon_item_id,
        });

        for raw_item in &raw.items {
            let name = raw_item.name();
            let Some(item_id) = item_registry::network_id(name) else {
                continue;
            };
            // Bloc → block_runtime_id, sinon 0.
            let brid = BLOCKS.get(name);
            let block_runtime_id = if brid != BLOCKS.air { brid as i32 } else { 0 };

            items.push(OwnedItem {
                entry_id: *next_entry_id,
                item_id,
                block_runtime_id,
                group_id: group_index,
            });
            *next_entry_id += 1;
        }
    }
}

fn load_creative_data() -> CreativeData {
    let mut groups = Vec::new();
    let mut items = Vec::new();
    let mut next_entry_id: u32 = 0;

    load_category(
        CAT_CONSTRUCTION,
        CONSTRUCTION_JSON,
        &mut groups,
        &mut items,
        &mut next_entry_id,
    );
    load_category(
        CAT_NATURE,
        NATURE_JSON,
        &mut groups,
        &mut items,
        &mut next_entry_id,
    );
    load_category(
        CAT_EQUIPMENT,
        EQUIPMENT_JSON,
        &mut groups,
        &mut items,
        &mut next_entry_id,
    );
    load_category(
        CAT_ITEMS,
        ITEMS_JSON,
        &mut groups,
        &mut items,
        &mut next_entry_id,
    );

    CreativeData { groups, items }
}

pub fn groups() -> Vec<CreativeGroupEntry<'static>> {
    CREATIVE_DATA
        .groups
        .iter()
        .map(|g| CreativeGroupEntry {
            category_id: g.category_id,
            category_name: g.category_name.as_str(),
            icon_item_id: g.icon_item_id,
        })
        .collect()
}

pub fn items() -> Vec<CreativeItemEntry> {
    CREATIVE_DATA
        .items
        .iter()
        .map(|it| CreativeItemEntry {
            entry_id: it.entry_id,
            item_id: it.item_id,
            block_runtime_id: it.block_runtime_id,
            group_id: it.group_id,
        })
        .collect()
}

pub fn stats() -> (usize, usize) {
    (CREATIVE_DATA.groups.len(), CREATIVE_DATA.items.len())
}

/// Résout l'item vanilla (name) à partir de son creative_item_network_id
/// (`entry_id`). Utilisé quand le client envoie CraftCreative.
pub fn item_name_by_entry_id(entry_id: u32) -> Option<&'static str> {
    CREATIVE_DATA
        .items
        .iter()
        .find(|it| it.entry_id == entry_id)
        .and_then(|it| item_registry::item_name_by_id(it.item_id))
}
