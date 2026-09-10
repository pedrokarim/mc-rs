//! CreativeContent — inventaire créatif vanilla Bedrock 1.26.40 (inchangé en
//! 1.26.45), chargé depuis `data/creative_items.nbt` : dragonfly v0.11.4
//! `server/item/creative/creative_items.nbt`. PMMP/BedrockData s'arrête à
//! 1.26.30 (fin de support de PocketMine-MP).
//!
//! NBT réseau (VarInt), compound racine :
//! ```text
//! groups: [ { category: Int, name: String, icon: <entrée> } ]
//! items:  [ <entrée> + group_index: Int ]
//! <entrée> = { name: String, meta: Short, nbt: Compound, block_properties: Compound }
//! ```
//! `meta`, `nbt`, `block_properties` et `group_index` sont omis quand ils sont
//! vides ou nuls (dragonfly `creativeItemEntry`, tags `omitempty`). Un groupe
//! au nom vide est un groupe anonyme, son icône (`name` vide) est de l'air.
//!
//! Choix et limites :
//! - `nbt` (125 livres enchantés, bannières, fusées et étoiles de feu
//!   d'artifice) est transmise telle quelle dans le blob userData, en NBT
//!   little-endian disque, comme gophertunnel (`itemUserData`), Cloudburst
//!   (`writeItemInstance`) et PMMP (`ItemStackExtraData`) écrivent la NBT d'un
//!   item. Ce sont les tags vanilla (`ench`, `Type`, `Fireworks`…) : sans eux,
//!   les 125 livres enchantés seraient 125 livres identiques.
//! - `block_properties` est ignoré : notre registre de blocs ne résout qu'un
//!   nom vers son premier état (`BLOCKS.get`), pas un état précis. Les items
//!   qui désignent un état particulier reçoivent le block runtime id de ce
//!   premier état.

use std::sync::LazyLock;

use mc_rs_nbt::{NbtCompound, NbtRoot, NbtTag};
use mc_rs_proto::packets::player::ItemStack;
use mc_rs_proto::packets::world::{item_user_data, CreativeGroupEntry, CreativeItemEntry};

use crate::item_registry;
use crate::world::block_registry::BLOCKS;

const CREATIVE_ITEMS_NBT: &[u8] = include_bytes!("../data/creative_items.nbt");

/// Seul item dont le blob userData porte un `blockingTick` en plus.
const SHIELD_NAME: &str = "minecraft:shield";

/// Pile résolue (ids réseau courants), prête à être encodée.
pub struct OwnedStack {
    pub item_id: i32,
    pub meta: u32,
    pub block_runtime_id: i32,
    /// Blob userData complet (cf. `item_user_data`), vide pour l'air.
    pub user_data: Vec<u8>,
}

impl OwnedStack {
    const AIR: Self = Self {
        item_id: 0,
        meta: 0,
        block_runtime_id: 0,
        user_data: Vec::new(),
    };
}

pub struct OwnedGroup {
    /// Onglet : 1 Construction, 2 Nature, 3 Equipment, 4 Items.
    pub category_id: u8,
    pub category_name: String,
    pub icon: OwnedStack,
}

pub struct OwnedItem {
    /// `creativeItemNetworkId` : index de l'item dans la liste, à partir de 0
    /// (PMMP `CreativeInventoryCache`, clé `$k` de `getAllEntries()`).
    pub entry_id: u32,
    pub stack: OwnedStack,
    pub group_id: u32,
}

/// Groupes + items pré-construits au démarrage, prêts à être encodés.
pub struct CreativeData {
    pub groups: Vec<OwnedGroup>,
    pub items: Vec<OwnedItem>,
    /// Entrées ignorées (nom inconnu du registre d'items ou groupe invalide).
    pub skipped: usize,
}

static CREATIVE_DATA: LazyLock<CreativeData> = LazyLock::new(load_creative_data);

fn load_creative_data() -> CreativeData {
    let mut cursor = CREATIVE_ITEMS_NBT;
    let root = mc_rs_nbt::read_nbt_network(&mut cursor).expect("creative_items.nbt valide");
    let data = parse_creative_data(&root.compound);
    if data.skipped > 0 {
        tracing::warn!(
            "creative_items.nbt : {} items ignorés (inconnus du registre d'items)",
            data.skipped
        );
    }
    data
}

fn parse_creative_data(root: &NbtCompound) -> CreativeData {
    let raw_groups = root
        .get("groups")
        .and_then(NbtTag::as_list)
        .expect("creative_items.nbt : liste `groups` absente");
    let raw_items = root
        .get("items")
        .and_then(NbtTag::as_list)
        .expect("creative_items.nbt : liste `items` absente");

    // Aucun groupe n'est écarté : les items les référencent par index.
    let groups: Vec<OwnedGroup> = raw_groups
        .iter()
        .enumerate()
        .map(|(index, tag)| {
            let entry = tag
                .as_compound()
                .unwrap_or_else(|| panic!("creative_items.nbt : groupe {index} invalide"));
            let category = entry
                .get("category")
                .and_then(NbtTag::as_int)
                .and_then(|category| u8::try_from(category).ok())
                .unwrap_or_else(|| panic!("creative_items.nbt : groupe {index} sans catégorie"));
            let category_name = entry
                .get("name")
                .and_then(NbtTag::as_string)
                .unwrap_or_default()
                .to_owned();
            // Icône absente, anonyme ou inconnue du registre : air.
            let icon = entry
                .get("icon")
                .and_then(NbtTag::as_compound)
                .and_then(resolve_stack)
                .unwrap_or(OwnedStack::AIR);
            OwnedGroup {
                category_id: category,
                category_name,
                icon,
            }
        })
        .collect();

    let mut items = Vec::with_capacity(raw_items.len());
    let mut skipped = 0;
    for tag in raw_items {
        let Some(entry) = tag.as_compound() else {
            skipped += 1;
            continue;
        };
        let group_id = entry
            .get("group_index")
            .and_then(NbtTag::as_int)
            .unwrap_or(0);
        let Some(group_id) = u32::try_from(group_id)
            .ok()
            .filter(|&group_id| (group_id as usize) < groups.len())
        else {
            skipped += 1;
            continue;
        };
        let Some(stack) = resolve_stack(entry) else {
            tracing::debug!(
                "creative_items.nbt : item ignoré {:?}",
                entry.get("name").and_then(NbtTag::as_string)
            );
            skipped += 1;
            continue;
        };
        items.push(OwnedItem {
            entry_id: items.len() as u32,
            stack,
            group_id,
        });
    }

    CreativeData {
        groups,
        items,
        skipped,
    }
}

/// Résout une entrée `{name, meta, nbt}` en pile réseau. `None` si le nom est
/// vide ou inconnu du registre d'items.
fn resolve_stack(entry: &NbtCompound) -> Option<OwnedStack> {
    let name = entry.get("name").and_then(NbtTag::as_string)?;
    let item_id = item_registry::network_id(name)?;
    // dragonfly lit `meta` en int16 et l'envoie en `uint32(meta)`.
    let meta = entry.get("meta").and_then(NbtTag::as_short).unwrap_or(0) as u32;
    // Bloc → premier état connu pour ce nom (voir la limite en tête de module).
    let brid = BLOCKS.get(name);
    let block_runtime_id = if brid != BLOCKS.air { brid as i32 } else { 0 };
    let nbt_le = entry
        .get("nbt")
        .and_then(NbtTag::as_compound)
        .filter(|nbt| !nbt.is_empty())
        .map(|nbt| {
            let mut buf = Vec::new();
            mc_rs_nbt::write_nbt_le(&mut buf, &NbtRoot::new("", nbt.clone()));
            buf
        });
    Some(OwnedStack {
        item_id,
        meta,
        block_runtime_id,
        user_data: item_user_data(nbt_le.as_deref(), name == SHIELD_NAME),
    })
}

pub fn groups() -> Vec<CreativeGroupEntry<'static>> {
    CREATIVE_DATA
        .groups
        .iter()
        .map(|g| CreativeGroupEntry {
            category_id: g.category_id,
            category_name: g.category_name.as_str(),
            icon_item_id: g.icon.item_id,
            icon_meta: g.icon.meta,
            icon_block_runtime_id: g.icon.block_runtime_id,
            icon_user_data: &g.icon.user_data,
        })
        .collect()
}

pub fn items() -> Vec<CreativeItemEntry<'static>> {
    CREATIVE_DATA
        .items
        .iter()
        .map(|it| CreativeItemEntry {
            entry_id: it.entry_id,
            item_id: it.stack.item_id,
            meta: it.stack.meta,
            block_runtime_id: it.stack.block_runtime_id,
            user_data: &it.stack.user_data,
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
        .get(entry_id as usize)
        .and_then(|it| item_registry::item_name_by_id(it.stack.item_id))
}

/// Pile complète (id, meta, block runtime id, userData avec NBT) désignée par
/// un creative_item_network_id, avec la quantité voulue. À préférer à
/// [`item_name_by_entry_id`] pour CraftCreative : plusieurs entrées partagent
/// le même item et ne diffèrent que par la meta ou la NBT.
pub fn item_stack_by_entry_id(entry_id: u32, count: u16) -> Option<ItemStack> {
    CREATIVE_DATA
        .items
        .get(entry_id as usize)
        .map(|it| ItemStack {
            id: it.stack.item_id,
            count,
            meta: it.stack.meta,
            block_runtime_id: it.stack.block_runtime_id,
            extra_data: it.stack.user_data.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_proto::io::ProtoReader;
    use mc_rs_proto::packets::world::CreativeContent;

    #[test]
    fn creative_items_nbt_loads_every_entry() {
        let data = &*CREATIVE_DATA;
        assert_eq!(data.groups.len(), 123);
        assert_eq!(data.skipped, 0, "items inconnus du registre");
        assert_eq!(data.items.len(), 1875);
        assert!(data.groups.iter().all(|g| (1..=4).contains(&g.category_id)));
        for (index, item) in data.items.iter().enumerate() {
            assert_eq!(item.entry_id as usize, index);
            assert!((item.group_id as usize) < data.groups.len());
        }
        // Groupe anonyme : nom vide, icône air sans userData.
        let anonymous = data
            .groups
            .iter()
            .find(|g| g.category_name.is_empty())
            .expect("groupe anonyme");
        assert_eq!(anonymous.icon.item_id, 0);
        assert!(anonymous.icon.user_data.is_empty());
    }

    #[test]
    fn meta_and_nbt_are_kept() {
        let data = &*CREATIVE_DATA;
        let arrow = item_registry::required_item_id("minecraft:arrow");
        assert!(data
            .items
            .iter()
            .any(|it| it.stack.item_id == arrow && it.stack.meta == 6));

        let book = item_registry::required_item_id("minecraft:enchanted_book");
        let book = data
            .items
            .iter()
            .find(|it| it.stack.item_id == book)
            .expect("livre enchanté");
        let blob = &book.stack.user_data;
        assert_eq!(&blob[..3], &[0xFF, 0xFF, 0x01]);
        let mut nbt = &blob[3..blob.len() - 8];
        let root = mc_rs_nbt::read_nbt_le(&mut nbt).expect("NBT LE disque");
        assert!(nbt.is_empty());
        assert!(root.compound.contains_key("ench"));
        assert_eq!(&blob[blob.len() - 8..], &[0u8; 8]);

        let shield = item_registry::required_item_id("minecraft:shield");
        let shield = data
            .items
            .iter()
            .find(|it| it.stack.item_id == shield)
            .expect("bouclier");
        assert_eq!(shield.stack.user_data.len(), 18);
    }

    /// Relit le paquet complet au format 2169 et vérifie qu'il est consommé
    /// exactement.
    #[test]
    fn creative_content_packet_parses_back() {
        fn read_item_instance(r: &mut ProtoReader) -> (i32, u32) {
            let id = r.read_var_i32().unwrap();
            let count = r.read_u16_le().unwrap();
            let meta = r.read_var_u32().unwrap();
            let _block_runtime_id = r.read_var_i32().unwrap();
            let user_data = r.read_byte_array().unwrap();
            if id == 0 {
                assert_eq!((count, meta, user_data.len()), (0, 0, 0));
            } else {
                assert_eq!(count, 1);
                assert!(user_data.len() >= 10);
            }
            (id, meta)
        }

        let payload = CreativeContent::encode(&groups(), &items());
        let mut r = ProtoReader::new(&payload);
        let group_count = r.read_var_u32().unwrap();
        assert_eq!(group_count, 123);
        for _ in 0..group_count {
            let category = r.read_u8().unwrap();
            assert!((1..=4).contains(&category));
            r.read_string().unwrap();
            read_item_instance(&mut r);
        }
        let item_count = r.read_var_u32().unwrap();
        for expected_entry in 0..item_count {
            assert_eq!(r.read_var_u32().unwrap(), expected_entry);
            let (id, _) = read_item_instance(&mut r);
            assert_ne!(id, 0);
            assert!(r.read_var_u32().unwrap() < group_count);
        }
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn entry_id_resolves_full_stack() {
        let book = item_registry::required_item_id("minecraft:enchanted_book");
        let entry = CREATIVE_DATA
            .items
            .iter()
            .find(|it| it.stack.item_id == book)
            .expect("livre enchanté");
        let stack = item_stack_by_entry_id(entry.entry_id, 1).expect("entrée connue");
        assert_eq!(stack.id, book);
        assert_eq!(stack.extra_data, entry.stack.user_data);
        assert_eq!(
            item_name_by_entry_id(entry.entry_id),
            Some("minecraft:enchanted_book")
        );
        assert!(item_stack_by_entry_id(u32::MAX, 1).is_none());
    }
}
