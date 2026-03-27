use std::collections::HashMap;
use std::sync::LazyLock;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use mc_rs_proto::io::ProtoWriter;
use serde::Deserialize;

const REQUIRED_ITEM_LIST_JSON: &str = include_str!("../data/required_item_list.json");
const EMPTY_COMPONENT_NBT: &[u8] = &[0x0A, 0x00, 0x00, 0x00];

#[derive(Debug, Deserialize)]
struct RawItemTypeEntry {
    runtime_id: i32,
    component_based: bool,
    version: i32,
    component_nbt: Option<String>,
}

struct ItemTypeEntry {
    string_id: String,
    runtime_id: i32,
    component_based: bool,
    version: i32,
    component_nbt: Vec<u8>,
}

pub struct ItemRegistryData {
    entries: Vec<ItemTypeEntry>,
    payload: Vec<u8>,
    by_name: HashMap<String, i32>,
    by_runtime_id: HashMap<i32, String>,
}

impl ItemRegistryData {
    fn load() -> Self {
        let raw: HashMap<String, RawItemTypeEntry> =
            serde_json::from_str(REQUIRED_ITEM_LIST_JSON).expect("valid required_item_list.json");

        let mut entries: Vec<ItemTypeEntry> = raw
            .into_iter()
            .map(|(string_id, entry)| {
                let component_nbt = entry
                    .component_nbt
                    .as_deref()
                    .map(|encoded| {
                        BASE64_STANDARD
                            .decode(encoded)
                            .expect("valid base64 component_nbt")
                    })
                    .unwrap_or_else(|| EMPTY_COMPONENT_NBT.to_vec());

                ItemTypeEntry {
                    string_id,
                    runtime_id: entry.runtime_id,
                    component_based: entry.component_based,
                    version: entry.version,
                    component_nbt,
                }
            })
            .collect();

        entries.sort_by(|left, right| left.string_id.cmp(&right.string_id));

        let mut payload = ProtoWriter::with_capacity(REQUIRED_ITEM_LIST_JSON.len() / 2);
        payload.write_var_u32(entries.len() as u32);
        for entry in &entries {
            payload.write_string(&entry.string_id);
            payload.write_i16_le(
                i16::try_from(entry.runtime_id).expect("item runtime ID must fit in i16"),
            );
            payload.write_bool(entry.component_based);
            payload.write_var_i32(entry.version);
            payload.write_raw(&entry.component_nbt);
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

pub fn entry_count() -> usize {
    ITEM_REGISTRY.entries.len()
}

pub fn network_id(name: &str) -> Option<i32> {
    ITEM_REGISTRY.by_name.get(name).copied()
}

pub fn is_known_network_id(id: i32) -> bool {
    ITEM_REGISTRY.by_runtime_id.contains_key(&id)
}

pub fn required_item_id(name: &str) -> i32 {
    network_id(name).unwrap_or_else(|| panic!("missing required item registry entry for {name}"))
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

    #[test]
    fn required_item_registry_contains_core_entries() {
        assert_eq!(network_id("minecraft:dirt"), Some(3));
        assert_eq!(network_id("minecraft:compass"), Some(423));
        assert!(entry_count() > 1000);
        assert!(!payload().is_empty());
    }

    #[test]
    fn legacy_item_ids_normalize_to_bedrock_network_ids() {
        assert_eq!(migrate_legacy_item_id(345), 423);
        assert_eq!(migrate_legacy_item_id(3), 3);
    }
}
