use std::collections::HashMap;
use std::sync::LazyLock;

use super::block_registry_data::BLOCK_NAME_TO_FIRST_RUNTIME_ID;

/// Global block registry backed by generated Rust data.
///
/// The source of truth is still the Bedrock block palette extracted from BDS,
/// but runtime no longer reparses `canonical_block_states.nbt` on startup.
pub static BLOCKS: LazyLock<BlockRegistry> = LazyLock::new(BlockRegistry::load);

pub struct BlockRegistry {
    name_to_id: HashMap<&'static str, u32>,

    // Core blocks
    pub air: u32,
    pub stone: u32,
    pub dirt: u32,
    pub grass_block: u32,
    pub bedrock: u32,
    pub water: u32,
    pub lava: u32,
    pub sand: u32,
    pub red_sand: u32,
    pub sandstone: u32,
    pub red_sandstone: u32,
    pub gravel: u32,
    pub cobblestone: u32,

    // Sub-surface
    pub deepslate: u32,
    pub tuff: u32,
    pub granite: u32,
    pub diorite: u32,
    pub andesite: u32,

    // Terrain variants
    pub snow_layer: u32,
    pub snow_block: u32,
    pub mycelium: u32,
    pub podzol: u32,
    pub coarse_dirt: u32,
    pub hardened_clay: u32,

    // Ores
    pub coal_ore: u32,
    pub iron_ore: u32,
    pub gold_ore: u32,
    pub diamond_ore: u32,
    pub redstone_ore: u32,
    pub lapis_ore: u32,

    // Logs
    pub oak_log: u32,
    pub birch_log: u32,
    pub spruce_log: u32,
    pub acacia_log: u32,
    pub dark_oak_log: u32,
    pub jungle_log: u32,

    // Leaves
    pub oak_leaves: u32,
    pub birch_leaves: u32,
    pub spruce_leaves: u32,
    pub acacia_leaves: u32,
    pub dark_oak_leaves: u32,
    pub jungle_leaves: u32,

    // Flowers & vegetation
    pub short_grass: u32,
    pub tall_grass: u32,
    pub fern: u32,
    pub large_fern: u32,
    pub dandelion: u32,
    pub poppy: u32,
    pub blue_orchid: u32,
    pub allium: u32,
    pub azure_bluet: u32,
    pub oxeye_daisy: u32,
    pub cornflower: u32,
    pub waterlily: u32,
    pub seagrass: u32,
    pub cactus: u32,
    pub deadbush: u32,
    pub brown_mushroom: u32,
    pub red_mushroom: u32,
    pub pumpkin: u32,
    pub reeds: u32,
    pub bamboo: u32,
}

impl BlockRegistry {
    fn lookup(mapping: &HashMap<&'static str, u32>, name: &str) -> u32 {
        *mapping.get(name).unwrap_or_else(|| {
            tracing::warn!("Block '{}' not found in generated block registry", name);
            mapping.get("minecraft:air").unwrap_or(&0)
        })
    }

    fn load() -> Self {
        let mapping = build_block_mapping();
        let m = &mapping;

        let result = Self {
            air: Self::lookup(m, "minecraft:air"),
            stone: Self::lookup(m, "minecraft:stone"),
            dirt: Self::lookup(m, "minecraft:dirt"),
            grass_block: Self::lookup(m, "minecraft:grass_block"),
            bedrock: Self::lookup(m, "minecraft:bedrock"),
            water: Self::lookup(m, "minecraft:water"),
            lava: Self::lookup(m, "minecraft:lava"),
            sand: Self::lookup(m, "minecraft:sand"),
            red_sand: Self::lookup(m, "minecraft:red_sand"),
            sandstone: Self::lookup(m, "minecraft:sandstone"),
            red_sandstone: Self::lookup(m, "minecraft:red_sandstone"),
            gravel: Self::lookup(m, "minecraft:gravel"),
            cobblestone: Self::lookup(m, "minecraft:cobblestone"),

            deepslate: Self::lookup(m, "minecraft:deepslate"),
            tuff: Self::lookup(m, "minecraft:tuff"),
            granite: Self::lookup(m, "minecraft:granite"),
            diorite: Self::lookup(m, "minecraft:diorite"),
            andesite: Self::lookup(m, "minecraft:andesite"),

            snow_layer: Self::lookup(m, "minecraft:snow_layer"),
            snow_block: Self::lookup(m, "minecraft:snow"),
            mycelium: Self::lookup(m, "minecraft:mycelium"),
            podzol: Self::lookup(m, "minecraft:podzol"),
            coarse_dirt: Self::lookup(m, "minecraft:coarse_dirt"),
            hardened_clay: Self::lookup(m, "minecraft:hardened_clay"),

            coal_ore: Self::lookup(m, "minecraft:coal_ore"),
            iron_ore: Self::lookup(m, "minecraft:iron_ore"),
            gold_ore: Self::lookup(m, "minecraft:gold_ore"),
            diamond_ore: Self::lookup(m, "minecraft:diamond_ore"),
            redstone_ore: Self::lookup(m, "minecraft:redstone_ore"),
            lapis_ore: Self::lookup(m, "minecraft:lapis_ore"),

            oak_log: Self::lookup(m, "minecraft:oak_log"),
            birch_log: Self::lookup(m, "minecraft:birch_log"),
            spruce_log: Self::lookup(m, "minecraft:spruce_log"),
            acacia_log: Self::lookup(m, "minecraft:acacia_log"),
            dark_oak_log: Self::lookup(m, "minecraft:dark_oak_log"),
            jungle_log: Self::lookup(m, "minecraft:jungle_log"),

            oak_leaves: Self::lookup(m, "minecraft:oak_leaves"),
            birch_leaves: Self::lookup(m, "minecraft:birch_leaves"),
            spruce_leaves: Self::lookup(m, "minecraft:spruce_leaves"),
            acacia_leaves: Self::lookup(m, "minecraft:acacia_leaves"),
            dark_oak_leaves: Self::lookup(m, "minecraft:dark_oak_leaves"),
            jungle_leaves: Self::lookup(m, "minecraft:jungle_leaves"),

            short_grass: Self::lookup(m, "minecraft:short_grass"),
            tall_grass: Self::lookup(m, "minecraft:tall_grass"),
            fern: Self::lookup(m, "minecraft:fern"),
            large_fern: Self::lookup(m, "minecraft:large_fern"),
            dandelion: Self::lookup(m, "minecraft:dandelion"),
            poppy: Self::lookup(m, "minecraft:poppy"),
            blue_orchid: Self::lookup(m, "minecraft:blue_orchid"),
            allium: Self::lookup(m, "minecraft:allium"),
            azure_bluet: Self::lookup(m, "minecraft:azure_bluet"),
            oxeye_daisy: Self::lookup(m, "minecraft:oxeye_daisy"),
            cornflower: Self::lookup(m, "minecraft:cornflower"),
            waterlily: Self::lookup(m, "minecraft:waterlily"),
            seagrass: Self::lookup(m, "minecraft:seagrass"),
            cactus: Self::lookup(m, "minecraft:cactus"),
            deadbush: Self::lookup(m, "minecraft:deadbush"),
            brown_mushroom: Self::lookup(m, "minecraft:brown_mushroom"),
            red_mushroom: Self::lookup(m, "minecraft:red_mushroom"),
            pumpkin: Self::lookup(m, "minecraft:pumpkin"),
            reeds: Self::lookup(m, "minecraft:reeds"),
            bamboo: Self::lookup(m, "minecraft:bamboo"),
            name_to_id: mapping,
        };
        result
    }

    pub fn get(&self, name: &str) -> u32 {
        self.name_to_id.get(name).copied().unwrap_or(self.air)
    }

    /// Retourne le nom (`minecraft:xxx`) du block pour un `block_id` donné.
    /// Recherche linéaire dans name_to_id — utilisé rarement (hardness /
    /// debug), pas d'index inverse maintenu.
    pub fn name_for(&self, block_id: u32) -> Option<&str> {
        self.name_to_id
            .iter()
            .find(|(_, id)| **id == block_id)
            .map(|(name, _)| *name)
    }

    /// Vrai si `block_id` correspond à un bed (toutes couleurs confondues).
    /// Utilisé pour détecter les right-clicks sur lit (sleep / spawn override).
    pub fn is_bed(&self, block_id: u32) -> bool {
        const BED_COLORS: &[&str] = &[
            "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray",
            "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black",
        ];
        for color in BED_COLORS {
            let name = format!("minecraft:{}_bed", color);
            if self.get(&name) == block_id && block_id != self.air {
                return true;
            }
        }
        // Legacy name (protocol < 1.13).
        self.get("minecraft:bed") == block_id && block_id != self.air
    }
}

/// Build the block name → first runtime ID mapping from generated Rust data.
fn build_block_mapping() -> HashMap<&'static str, u32> {
    let mapping = BLOCK_NAME_TO_FIRST_RUNTIME_ID.iter().copied().collect();

    tracing::info!(
        "Block registry: loaded {} unique block names from generated data",
        BLOCK_NAME_TO_FIRST_RUNTIME_ID.len()
    );

    mapping
}
