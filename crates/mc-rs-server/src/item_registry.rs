use std::collections::HashMap;
use std::sync::LazyLock;

use mc_rs_nbt::tag::{NbtRoot, NbtTag};
use mc_rs_proto::io::ProtoWriter;

use crate::item_migration_data::LEGACY_ITEM_IDS;

/// Table d'items Bedrock 1.26.40 (inchangée en 1.26.45) : dragonfly v0.11.4
/// `server/world/vanilla_items.nbt` (commit `0c2c4045`), NBT réseau
/// `{nom: {runtime_id: Int, component_based: Byte, version: Int, data?: Compound}}`.
/// PMMP/BedrockData s'arrête à 1.26.30 (fin de support de PocketMine-MP).
const VANILLA_ITEMS_NBT: &[u8] = include_bytes!("../data/vanilla_items.nbt");

struct ItemTypeEntry {
    string_id: String,
    runtime_id: i32,
    component_based: bool,
    version: i32,
    component_nbt: NbtRoot,
}

pub struct ItemRegistryData {
    entries: Vec<ItemTypeEntry>,
    payload: Vec<u8>,
    by_name: HashMap<String, i32>,
    by_runtime_id: HashMap<i32, String>,
}

fn int_field(entry: &HashMap<String, NbtTag>, key: &str, name: &str) -> i32 {
    entry
        .get(key)
        .and_then(NbtTag::as_int)
        .unwrap_or_else(|| panic!("vanilla_items.nbt: {name} sans champ {key}"))
}

impl ItemRegistryData {
    fn load() -> Self {
        let mut cursor = VANILLA_ITEMS_NBT;
        let root = mc_rs_nbt::read_nbt_network(&mut cursor).expect("vanilla_items.nbt valide");

        let mut entries: Vec<ItemTypeEntry> = root
            .compound
            .into_iter()
            .map(|(string_id, tag)| {
                let NbtTag::Compound(entry) = tag else {
                    panic!("vanilla_items.nbt: entrée {string_id} n'est pas un compound");
                };
                let runtime_id = int_field(&entry, "runtime_id", &string_id);
                let version = int_field(&entry, "version", &string_id);
                let component_based = entry
                    .get("component_based")
                    .and_then(NbtTag::as_byte)
                    .is_some_and(|b| b != 0);
                let data = match entry.get("data") {
                    Some(NbtTag::Compound(data)) => data.clone(),
                    _ => HashMap::new(),
                };
                ItemTypeEntry {
                    string_id,
                    runtime_id,
                    component_based,
                    version,
                    component_nbt: NbtRoot::new("", data),
                }
            })
            .collect();

        entries.sort_by(|left, right| left.string_id.cmp(&right.string_id));

        let mut payload = ProtoWriter::with_capacity(VANILLA_ITEMS_NBT.len());
        payload.write_var_u32(entries.len() as u32);
        for entry in &entries {
            payload.write_string(&entry.string_id);
            payload.write_i16_le(
                i16::try_from(entry.runtime_id).expect("item runtime ID must fit in i16"),
            );
            payload.write_bool(entry.component_based);
            payload.write_var_i32(entry.version);
            // gophertunnel ItemEntry.Marshal : le component NBT est écrit
            // directement en NBT réseau (pas de ByteArray englobant).
            let mut nbt = Vec::new();
            mc_rs_nbt::write_nbt_network(&mut nbt, &entry.component_nbt);
            payload.write_raw(&nbt);
        }

        let by_name = entries
            .iter()
            .map(|entry| (entry.string_id.clone(), entry.runtime_id))
            .collect();
        let by_runtime_id = entries
            .iter()
            .map(|entry| (entry.runtime_id, entry.string_id.clone()))
            .collect();

        Self {
            entries,
            payload: payload.into_bytes(),
            by_name,
            by_runtime_id,
        }
    }
}

pub static ITEM_REGISTRY: LazyLock<ItemRegistryData> = LazyLock::new(ItemRegistryData::load);

pub fn payload() -> &'static [u8] {
    &ITEM_REGISTRY.payload
}

pub fn item_name_by_id(id: i32) -> Option<&'static str> {
    ITEM_REGISTRY.by_runtime_id.get(&id).map(String::as_str)
}

#[cfg(test)]
pub fn entry_count() -> usize {
    ITEM_REGISTRY.entries.len()
}

pub fn network_id(name: &str) -> Option<i32> {
    ITEM_REGISTRY.by_name.get(name).copied()
}

pub fn is_known_network_id(id: i32) -> bool {
    ITEM_REGISTRY.by_runtime_id.contains_key(&id)
}

/// Retourne tous les (name, runtime_id) connus. Utilisé pour populer
/// l'inventaire créatif ou la soft-enum commands.
pub fn all_entries() -> Vec<(&'static str, i32)> {
    ITEM_REGISTRY
        .entries
        .iter()
        .map(|e| (e.string_id.as_str(), e.runtime_id))
        .collect()
}

pub fn required_item_id(name: &str) -> i32 {
    network_id(name).unwrap_or_else(|| panic!("missing required item registry entry for {name}"))
}

/// Convertit un ID réseau de la table d'items héritée (sauvegardes antérieures
/// à la table courante, cf. `item_migration_data.rs`) en ID courant, via le nom.
/// Retourne 0 (air) si l'item n'existe plus.
pub fn migrate_saved_item_id(legacy_id: i32) -> i32 {
    if legacy_id == 0 {
        return 0;
    }
    LEGACY_ITEM_IDS
        .binary_search_by_key(&legacy_id, |&(id, _)| id)
        .ok()
        .and_then(|i| network_id(LEGACY_ITEM_IDS[i].1))
        .unwrap_or(0)
}

pub fn legacy_item_name(id: i32) -> Option<&'static str> {
    match id {
        1 => Some("minecraft:stone"),
        2 => Some("minecraft:grass_block"),
        3 => Some("minecraft:dirt"),
        4 => Some("minecraft:cobblestone"),
        7 => Some("minecraft:bedrock"),
        12 => Some("minecraft:sand"),
        13 => Some("minecraft:gravel"),
        14 => Some("minecraft:gold_ore"),
        15 => Some("minecraft:iron_ore"),
        16 => Some("minecraft:coal_ore"),
        17 => Some("minecraft:oak_log"),
        18 => Some("minecraft:oak_leaves"),
        21 => Some("minecraft:lapis_ore"),
        24 => Some("minecraft:sandstone"),
        31 => Some("minecraft:short_grass"),
        32 => Some("minecraft:deadbush"),
        39 => Some("minecraft:brown_mushroom"),
        40 => Some("minecraft:red_mushroom"),
        56 => Some("minecraft:diamond_ore"),
        73 => Some("minecraft:redstone_ore"),
        78 => Some("minecraft:snow_layer"),
        80 => Some("minecraft:snow"),
        81 => Some("minecraft:cactus"),
        82 => Some("minecraft:clay"),
        86 => Some("minecraft:pumpkin"),
        110 => Some("minecraft:mycelium"),
        111 => Some("minecraft:waterlily"),
        172 => Some("minecraft:hardened_clay"),
        179 => Some("minecraft:red_sandstone"),
        243 => Some("minecraft:podzol"),
        345 => Some("minecraft:compass"),
        _ => None,
    }
}

pub fn network_id_from_legacy(id: i32) -> Option<i32> {
    legacy_item_name(id).and_then(network_id)
}

pub fn migrate_legacy_item_id(id: i32) -> i32 {
    network_id_from_legacy(id).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_proto::io::ProtoReader;

    #[test]
    fn required_item_registry_contains_core_entries() {
        assert_eq!(network_id("minecraft:dirt"), Some(3));
        assert!(network_id("minecraft:compass").is_some());
        // Contenu 1.26.40 : famille poplar et cushions.
        assert!(network_id("minecraft:poplar_planks").is_some());
        assert!(network_id("minecraft:white_cushion").is_some());
        assert!(entry_count() > 1900);
        assert!(!payload().is_empty());
    }

    #[test]
    fn legacy_item_ids_normalize_to_bedrock_network_ids() {
        assert_eq!(
            migrate_legacy_item_id(345),
            network_id("minecraft:compass").unwrap()
        );
        assert_eq!(migrate_legacy_item_id(3), 3);
    }

    #[test]
    fn saved_item_ids_migrate_by_name() {
        // 423 = minecraft:compass dans la table 1.26.0.
        assert_eq!(
            migrate_saved_item_id(423),
            network_id("minecraft:compass").unwrap()
        );
        assert_eq!(migrate_saved_item_id(0), 0);
    }

    #[test]
    fn item_registry_payload_header_is_var_u32_count() {
        // Format (gophertunnel ItemEntry) : count en VarU32 + { String, I16LE,
        // Bool, VarI32, NBT réseau } × count.
        let mut reader = ProtoReader::new(payload());
        let count = reader.read_var_u32().unwrap() as usize;
        assert_eq!(count, ITEM_REGISTRY.entries.len());
    }
}
