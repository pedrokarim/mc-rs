//! Item registry mapping string IDs to numeric runtime IDs and properties.
//!
//! Loaded from canonical Bedrock data (pmmp/BedrockData `required_item_list.json`).
//! Provides max stack size, runtime ID lookup, and item table generation for StartGame.

use std::collections::HashMap;

use serde::Deserialize;

/// Canonical item list extracted from Bedrock Dedicated Server.
const ITEM_LIST_JSON: &str = include_str!("../data/item_list.json");

/// A single entry from the canonical item list JSON.
#[derive(Deserialize)]
#[allow(dead_code)]
struct RawItemEntry {
    runtime_id: i16,
    component_based: bool,
    /// Item version (0, 1, or 2). Added in newer BedrockData versions.
    #[serde(default)]
    version: u8,
    /// Base64-encoded NBT component data for items with components.
    #[serde(default)]
    component_nbt: Option<String>,
}

/// Properties for a single item type.
#[derive(Debug, Clone)]
pub struct ItemInfo {
    /// Namespaced item identifier, e.g. `"minecraft:stone"`.
    pub name: String,
    /// Protocol runtime ID (sent in StartGame item_table and NetworkItemStackDescriptor).
    pub numeric_id: i16,
    /// Maximum stack size (1, 16, or 64).
    pub max_stack_size: u8,
    /// Whether this is a component-based item (1.20+ custom items).
    pub is_component_based: bool,
}

/// Item table entry for the StartGame packet.
#[derive(Debug, Clone)]
pub struct ItemTableEntry {
    pub string_id: String,
    pub numeric_id: i16,
    pub is_component_based: bool,
}

/// Registry of all known Bedrock items.
pub struct ItemRegistry {
    by_name: HashMap<String, ItemInfo>,
    by_id: HashMap<i16, String>,
}

impl Default for ItemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemRegistry {
    /// Build the registry from the canonical item list JSON.
    pub fn new() -> Self {
        let raw: HashMap<String, RawItemEntry> =
            serde_json::from_str(ITEM_LIST_JSON).expect("invalid item_list.json");

        let mut by_name = HashMap::with_capacity(raw.len());
        let mut by_id = HashMap::with_capacity(raw.len());

        for (name, entry) in raw {
            let max_stack = max_stack_size_for(&name);
            by_id.insert(entry.runtime_id, name.clone());
            by_name.insert(
                name.clone(),
                ItemInfo {
                    name,
                    numeric_id: entry.runtime_id,
                    max_stack_size: max_stack,
                    is_component_based: entry.component_based,
                },
            );
        }

        Self { by_name, by_id }
    }

    /// Look up item info by string identifier.
    pub fn get_by_name(&self, name: &str) -> Option<&ItemInfo> {
        self.by_name.get(name)
    }

    /// Look up item name by numeric runtime ID.
    pub fn get_by_id(&self, id: i16) -> Option<&ItemInfo> {
        self.by_id.get(&id).and_then(|name| self.by_name.get(name))
    }

    /// Get max stack size for an item. Returns 64 for unknown items.
    pub fn max_stack_size(&self, id: i16) -> u8 {
        self.get_by_id(id)
            .map(|info| info.max_stack_size)
            .unwrap_or(64)
    }

    /// Generate the item table entries for the StartGame packet.
    pub fn item_table_entries(&self) -> Vec<ItemTableEntry> {
        self.by_name
            .values()
            .map(|info| ItemTableEntry {
                string_id: info.name.clone(),
                numeric_id: info.numeric_id,
                is_component_based: info.is_component_based,
            })
            .collect()
    }

    /// Total number of registered items.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Register a custom item (e.g. from a behavior pack).
    ///
    /// Assigns the next available numeric_id automatically.
    pub fn register_item(&mut self, name: String, max_stack_size: u8, is_component_based: bool) {
        if self.by_name.contains_key(&name) {
            return;
        }
        let next_id = self.by_id.keys().copied().max().unwrap_or(0) + 1;
        self.by_id.insert(next_id, name.clone());
        self.by_name.insert(
            name.clone(),
            ItemInfo {
                name,
                numeric_id: next_id,
                max_stack_size,
                is_component_based,
            },
        );
    }
}

/// Determine max stack size based on item name patterns.
///
/// Uses Bedrock conventions:
/// - Tools, weapons, armor → 1
/// - Boats, minecarts → 1
/// - Signs, banners, eggs, ender pearls, snowballs, honey bottles → 16
/// - Everything else → 64
fn max_stack_size_for(name: &str) -> u8 {
    // Strip "minecraft:" prefix for pattern matching
    let short = name.strip_prefix("minecraft:").unwrap_or(name);

    // === Stack size 1 (non-stackable) ===

    // Tools
    if short.ends_with("_sword")
        || short.ends_with("_pickaxe")
        || short.ends_with("_axe")
        || short.ends_with("_shovel")
        || short.ends_with("_hoe")
        || short.ends_with("_spear")
    {
        return 1;
    }

    // Armor
    if short.ends_with("_helmet")
        || short.ends_with("_chestplate")
        || short.ends_with("_leggings")
        || short.ends_with("_boots")
        || short.ends_with("_horse_armor")
        || short.ends_with("_nautilus_armor")
    {
        return 1;
    }

    // Vehicles
    if short.ends_with("_boat") || short.ends_with("_raft") || short.ends_with("_minecart") {
        return 1;
    }

    // Specific non-stackable items
    if matches!(
        short,
        "bow"
            | "crossbow"
            | "trident"
            | "shield"
            | "flint_and_steel"
            | "shears"
            | "fishing_rod"
            | "carrot_on_a_stick"
            | "warped_fungus_on_a_stick"
            | "elytra"
            | "totem_of_undying"
            | "debug_stick"
            | "spyglass"
            | "brush"
            | "goat_horn"
            | "recovery_compass"
            | "compass"
            | "clock"
            | "empty_map"
            | "filled_map"
            | "turtle_helmet"
            | "bed"
            | "cauldron"
            | "brewing_stand"
            | "cake"
            | "enchanted_book"
            | "knowledge_book"
            | "written_book"
            | "writable_book"
            | "potion"
            | "splash_potion"
            | "lingering_potion"
            | "music_disc_13"
            | "music_disc_cat"
            | "music_disc_blocks"
            | "music_disc_chirp"
            | "music_disc_far"
            | "music_disc_mall"
            | "music_disc_mellohi"
            | "music_disc_stal"
            | "music_disc_strad"
            | "music_disc_ward"
            | "music_disc_11"
            | "music_disc_wait"
            | "music_disc_otherside"
            | "music_disc_5"
            | "music_disc_pigstep"
            | "music_disc_relic"
            | "music_disc_creator"
            | "music_disc_creator_music_box"
            | "music_disc_precipice"
            | "chest_minecart"
            | "command_block_minecart"
            | "hopper_minecart"
            | "tnt_minecart"
            | "saddle"
            | "armor_stand"
            | "bucket"
            | "water_bucket"
            | "lava_bucket"
            | "milk_bucket"
            | "cod_bucket"
            | "salmon_bucket"
            | "tropical_fish_bucket"
            | "pufferfish_bucket"
            | "powder_snow_bucket"
            | "axolotl_bucket"
            | "tadpole_bucket"
    ) {
        return 1;
    }

    // === Stack size 16 ===
    if short.ends_with("_sign") && !short.contains("standing") && !short.contains("wall") {
        return 16;
    }

    if short.ends_with("_banner") && !short.contains("banner_pattern") {
        return 16;
    }

    if matches!(
        short,
        "egg"
            | "snowball"
            | "ender_pearl"
            | "ender_eye"
            | "honey_bottle"
            | "banner"
            | "wind_charge"
    ) {
        return 16;
    }

    // === Default: 64 ===
    64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_successfully() {
        let registry = ItemRegistry::new();
        assert!(
            registry.len() > 1000,
            "expected 1000+ items, got {}",
            registry.len()
        );
    }

    #[test]
    fn lookup_by_name() {
        let registry = ItemRegistry::new();
        let stone = registry.get_by_name("minecraft:stone").unwrap();
        assert_eq!(stone.numeric_id, 1);
        assert_eq!(stone.max_stack_size, 64);
    }

    #[test]
    fn lookup_by_id() {
        let registry = ItemRegistry::new();
        let info = registry.get_by_id(1).unwrap();
        assert_eq!(info.name, "minecraft:stone");
    }

    #[test]
    fn unknown_item_returns_none() {
        let registry = ItemRegistry::new();
        assert!(registry.get_by_name("minecraft:nonexistent").is_none());
        assert!(registry.get_by_id(i16::MAX).is_none());
    }

    #[test]
    fn max_stack_sizes() {
        let registry = ItemRegistry::new();

        // Tools → 1
        let sword = registry.get_by_name("minecraft:diamond_sword").unwrap();
        assert_eq!(sword.max_stack_size, 1);

        // Armor → 1
        let helmet = registry.get_by_name("minecraft:diamond_helmet").unwrap();
        assert_eq!(helmet.max_stack_size, 1);

        // Blocks → 64
        let dirt = registry.get_by_name("minecraft:dirt").unwrap();
        assert_eq!(dirt.max_stack_size, 64);

        // Eggs → 16
        let egg = registry.get_by_name("minecraft:egg").unwrap();
        assert_eq!(egg.max_stack_size, 16);

        // Ender pearls → 16
        let pearl = registry.get_by_name("minecraft:ender_pearl").unwrap();
        assert_eq!(pearl.max_stack_size, 16);
    }

    #[test]
    fn item_table_entries_complete() {
        let registry = ItemRegistry::new();
        let entries = registry.item_table_entries();
        assert_eq!(entries.len(), registry.len());
        // Verify at least one entry has correct data
        let stone_entry = entries
            .iter()
            .find(|e| e.string_id == "minecraft:stone")
            .unwrap();
        assert_eq!(stone_entry.numeric_id, 1);
        assert!(!stone_entry.is_component_based);
    }

    #[test]
    fn no_id_collisions() {
        let registry = ItemRegistry::new();
        let entries = registry.item_table_entries();
        let mut seen_ids: HashMap<i16, &str> = HashMap::new();
        for entry in &entries {
            if let Some(existing) = seen_ids.get(&entry.numeric_id) {
                panic!(
                    "ID collision: {} and {} both have numeric_id {}",
                    existing, entry.string_id, entry.numeric_id
                );
            }
            seen_ids.insert(entry.numeric_id, &entry.string_id);
        }
    }

    #[test]
    fn boats_not_stackable() {
        let registry = ItemRegistry::new();
        let boat = registry.get_by_name("minecraft:acacia_boat").unwrap();
        assert_eq!(boat.max_stack_size, 1);
    }

    #[test]
    fn register_custom_item() {
        let mut registry = ItemRegistry::new();
        let old_len = registry.len();
        registry.register_item("custom:ruby".to_string(), 64, true);
        assert_eq!(registry.len(), old_len + 1);
        let ruby = registry.get_by_name("custom:ruby").unwrap();
        assert_eq!(ruby.max_stack_size, 64);
        assert!(ruby.is_component_based);
        // Should also be in item table entries
        let entries = registry.item_table_entries();
        assert!(entries.iter().any(|e| e.string_id == "custom:ruby"));
        // Registering same item again should be a no-op
        registry.register_item("custom:ruby".to_string(), 16, true);
        assert_eq!(registry.len(), old_len + 1);
    }
}
