//! Block property registry mapping runtime IDs (FNV-1a hashes) to block info.
//!
//! Provides hardness, solidity, and tool type data for all vanilla Bedrock blocks.
//! Unknown blocks default to solid with unknown hardness.

use std::collections::HashMap;

use crate::block_hash::hash_block_state;

/// Tool types relevant for mining speed calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    None,
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
    Shears,
}

/// Properties for a single block type.
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Namespaced block identifier, e.g. `"minecraft:stone"`.
    pub name: &'static str,
    /// Mining hardness. `-1.0` = unbreakable, `0.0` = instant break.
    pub hardness: f32,
    /// Whether entities collide with this block.
    pub is_solid: bool,
    /// The preferred tool type for faster mining.
    pub tool_type: ToolType,
}

/// Registry mapping block runtime ID hashes to block info.
pub struct BlockRegistry {
    blocks: HashMap<u32, &'static BlockInfo>,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockRegistry {
    /// Build the registry by hashing each block name and mapping to its info.
    pub fn new() -> Self {
        let mut blocks = HashMap::with_capacity(BLOCK_DATA.len());
        for info in BLOCK_DATA {
            let hash = hash_block_state(info.name);
            blocks.insert(hash, info);
        }
        Self { blocks }
    }

    /// Look up full block info by runtime ID hash.
    pub fn get(&self, hash: u32) -> Option<&&'static BlockInfo> {
        self.blocks.get(&hash)
    }

    /// Check if a block is solid. Defaults to `true` for unknown blocks.
    pub fn is_solid(&self, hash: u32) -> bool {
        self.blocks
            .get(&hash)
            .map(|info| info.is_solid)
            .unwrap_or(true)
    }

    /// Get block hardness. Returns `None` for unknown blocks.
    pub fn hardness(&self, hash: u32) -> Option<f32> {
        self.blocks.get(&hash).map(|info| info.hardness)
    }

    /// Calculate expected mining time in seconds (bare hand, no tool).
    pub fn expected_mining_secs(&self, hash: u32) -> Option<f32> {
        self.blocks.get(&hash).map(|info| {
            if info.hardness <= 0.0 {
                return 0.0;
            }
            match info.tool_type {
                ToolType::None => info.hardness * 1.5,
                _ => info.hardness * 5.0, // wrong tool penalty
            }
        })
    }
}

// Helper macro to reduce boilerplate in the data array.
macro_rules! block {
    ($name:expr, $hardness:expr, solid, $tool:ident) => {
        BlockInfo {
            name: $name,
            hardness: $hardness,
            is_solid: true,
            tool_type: ToolType::$tool,
        }
    };
    ($name:expr, $hardness:expr, non_solid, $tool:ident) => {
        BlockInfo {
            name: $name,
            hardness: $hardness,
            is_solid: false,
            tool_type: ToolType::$tool,
        }
    };
}

/// Static data array of all vanilla Bedrock block properties.
/// Hardness values from Minecraft Wiki (Bedrock Edition).
static BLOCK_DATA: &[BlockInfo] = &[
    // ===== Special =====
    block!("minecraft:air", 0.0, non_solid, None),
    block!("minecraft:bedrock", -1.0, solid, None),
    block!("minecraft:barrier", -1.0, solid, None),
    block!("minecraft:light_block", 0.0, non_solid, None),
    block!("minecraft:structure_void", 0.0, non_solid, None),
    // ===== Stone variants =====
    block!("minecraft:stone", 1.5, solid, Pickaxe),
    block!("minecraft:granite", 1.5, solid, Pickaxe),
    block!("minecraft:polished_granite", 1.5, solid, Pickaxe),
    block!("minecraft:diorite", 1.5, solid, Pickaxe),
    block!("minecraft:polished_diorite", 1.5, solid, Pickaxe),
    block!("minecraft:andesite", 1.5, solid, Pickaxe),
    block!("minecraft:polished_andesite", 1.5, solid, Pickaxe),
    block!("minecraft:cobblestone", 2.0, solid, Pickaxe),
    block!("minecraft:mossy_cobblestone", 2.0, solid, Pickaxe),
    block!("minecraft:smooth_stone", 2.0, solid, Pickaxe),
    block!("minecraft:stone_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:mossy_stone_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:cracked_stone_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:chiseled_stone_bricks", 1.5, solid, Pickaxe),
    // ===== Deepslate =====
    block!("minecraft:deepslate", 3.0, solid, Pickaxe),
    block!("minecraft:cobbled_deepslate", 3.5, solid, Pickaxe),
    block!("minecraft:polished_deepslate", 3.5, solid, Pickaxe),
    block!("minecraft:deepslate_bricks", 3.5, solid, Pickaxe),
    block!("minecraft:cracked_deepslate_bricks", 3.5, solid, Pickaxe),
    block!("minecraft:deepslate_tiles", 3.5, solid, Pickaxe),
    block!("minecraft:cracked_deepslate_tiles", 3.5, solid, Pickaxe),
    block!("minecraft:chiseled_deepslate", 3.5, solid, Pickaxe),
    block!("minecraft:reinforced_deepslate", -1.0, solid, None),
    // ===== Other stone types =====
    block!("minecraft:tuff", 1.5, solid, Pickaxe),
    block!("minecraft:polished_tuff", 1.5, solid, Pickaxe),
    block!("minecraft:tuff_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:chiseled_tuff_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:chiseled_tuff", 1.5, solid, Pickaxe),
    block!("minecraft:calcite", 0.75, solid, Pickaxe),
    block!("minecraft:dripstone_block", 1.5, solid, Pickaxe),
    block!("minecraft:pointed_dripstone", 1.5, solid, Pickaxe),
    block!("minecraft:basalt", 1.25, solid, Pickaxe),
    block!("minecraft:polished_basalt", 1.25, solid, Pickaxe),
    block!("minecraft:smooth_basalt", 1.25, solid, Pickaxe),
    block!("minecraft:blackstone", 1.5, solid, Pickaxe),
    block!("minecraft:polished_blackstone", 2.0, solid, Pickaxe),
    block!("minecraft:polished_blackstone_bricks", 1.5, solid, Pickaxe),
    block!(
        "minecraft:cracked_polished_blackstone_bricks",
        1.5,
        solid,
        Pickaxe
    ),
    block!(
        "minecraft:chiseled_polished_blackstone",
        1.5,
        solid,
        Pickaxe
    ),
    block!("minecraft:gilded_blackstone", 1.5, solid, Pickaxe),
    // ===== Dirt, grass, soil =====
    block!("minecraft:dirt", 0.5, solid, Shovel),
    block!("minecraft:coarse_dirt", 0.5, solid, Shovel),
    block!("minecraft:rooted_dirt", 0.5, solid, Shovel),
    block!("minecraft:grass_block", 0.6, solid, Shovel),
    block!("minecraft:grass_path", 0.65, solid, Shovel),
    block!("minecraft:dirt_with_roots", 0.5, solid, Shovel),
    block!("minecraft:mycelium", 0.6, solid, Shovel),
    block!("minecraft:podzol", 0.5, solid, Shovel),
    block!("minecraft:farmland", 0.6, solid, Shovel),
    block!("minecraft:mud", 0.5, solid, Shovel),
    block!("minecraft:packed_mud", 1.0, solid, None),
    block!("minecraft:mud_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:soul_sand", 0.5, solid, Shovel),
    block!("minecraft:soul_soil", 0.5, solid, Shovel),
    // ===== Sand, gravel, clay =====
    block!("minecraft:sand", 0.5, solid, Shovel),
    block!("minecraft:red_sand", 0.5, solid, Shovel),
    block!("minecraft:gravel", 0.6, solid, Shovel),
    block!("minecraft:clay", 0.6, solid, Shovel),
    block!("minecraft:suspicious_sand", 0.25, solid, None),
    block!("minecraft:suspicious_gravel", 0.25, solid, None),
    // ===== Ores =====
    block!("minecraft:coal_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_coal_ore", 4.5, solid, Pickaxe),
    block!("minecraft:iron_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_iron_ore", 4.5, solid, Pickaxe),
    block!("minecraft:gold_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_gold_ore", 4.5, solid, Pickaxe),
    block!("minecraft:diamond_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_diamond_ore", 4.5, solid, Pickaxe),
    block!("minecraft:emerald_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_emerald_ore", 4.5, solid, Pickaxe),
    block!("minecraft:lapis_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_lapis_ore", 4.5, solid, Pickaxe),
    block!("minecraft:redstone_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_redstone_ore", 4.5, solid, Pickaxe),
    block!("minecraft:lit_redstone_ore", 3.0, solid, Pickaxe),
    block!("minecraft:lit_deepslate_redstone_ore", 4.5, solid, Pickaxe),
    block!("minecraft:copper_ore", 3.0, solid, Pickaxe),
    block!("minecraft:deepslate_copper_ore", 4.5, solid, Pickaxe),
    block!("minecraft:nether_gold_ore", 3.0, solid, Pickaxe),
    block!("minecraft:ancient_debris", 30.0, solid, Pickaxe),
    block!("minecraft:quartz_ore", 3.0, solid, Pickaxe),
    // ===== Metal blocks =====
    block!("minecraft:iron_block", 5.0, solid, Pickaxe),
    block!("minecraft:gold_block", 3.0, solid, Pickaxe),
    block!("minecraft:diamond_block", 5.0, solid, Pickaxe),
    block!("minecraft:emerald_block", 5.0, solid, Pickaxe),
    block!("minecraft:lapis_block", 3.0, solid, Pickaxe),
    block!("minecraft:redstone_block", 5.0, solid, Pickaxe),
    block!("minecraft:netherite_block", 50.0, solid, Pickaxe),
    block!("minecraft:copper_block", 3.0, solid, Pickaxe),
    block!("minecraft:raw_iron_block", 5.0, solid, Pickaxe),
    block!("minecraft:raw_gold_block", 5.0, solid, Pickaxe),
    block!("minecraft:raw_copper_block", 5.0, solid, Pickaxe),
    block!("minecraft:amethyst_block", 1.5, solid, Pickaxe),
    block!("minecraft:budding_amethyst", 1.5, solid, Pickaxe),
    // ===== Copper oxidation variants =====
    block!("minecraft:exposed_copper", 3.0, solid, Pickaxe),
    block!("minecraft:weathered_copper", 3.0, solid, Pickaxe),
    block!("minecraft:oxidized_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_exposed_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_weathered_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_oxidized_copper", 3.0, solid, Pickaxe),
    block!("minecraft:cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:exposed_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:weathered_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:oxidized_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_exposed_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_weathered_cut_copper", 3.0, solid, Pickaxe),
    block!("minecraft:waxed_oxidized_cut_copper", 3.0, solid, Pickaxe),
    // ===== Wood — Oak =====
    block!("minecraft:oak_log", 2.0, solid, Axe),
    block!("minecraft:stripped_oak_log", 2.0, solid, Axe),
    block!("minecraft:oak_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_oak_wood", 2.0, solid, Axe),
    block!("minecraft:oak_planks", 2.0, solid, Axe),
    block!("minecraft:oak_slab", 2.0, solid, Axe),
    block!("minecraft:oak_stairs", 2.0, solid, Axe),
    block!("minecraft:oak_fence", 2.0, solid, Axe),
    block!("minecraft:oak_fence_gate", 2.0, solid, Axe),
    block!("minecraft:oak_door", 3.0, solid, Axe),
    block!("minecraft:oak_trapdoor", 3.0, solid, Axe),
    block!("minecraft:oak_pressure_plate", 0.5, solid, Axe),
    block!("minecraft:oak_button", 0.5, non_solid, None),
    block!("minecraft:oak_sign", 1.0, non_solid, Axe),
    block!("minecraft:oak_wall_sign", 1.0, non_solid, Axe),
    block!("minecraft:oak_hanging_sign", 1.0, non_solid, Axe),
    block!("minecraft:oak_leaves", 0.2, solid, Hoe),
    // ===== Wood — Spruce =====
    block!("minecraft:spruce_log", 2.0, solid, Axe),
    block!("minecraft:stripped_spruce_log", 2.0, solid, Axe),
    block!("minecraft:spruce_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_spruce_wood", 2.0, solid, Axe),
    block!("minecraft:spruce_planks", 2.0, solid, Axe),
    block!("minecraft:spruce_slab", 2.0, solid, Axe),
    block!("minecraft:spruce_stairs", 2.0, solid, Axe),
    block!("minecraft:spruce_fence", 2.0, solid, Axe),
    block!("minecraft:spruce_fence_gate", 2.0, solid, Axe),
    block!("minecraft:spruce_door", 3.0, solid, Axe),
    block!("minecraft:spruce_trapdoor", 3.0, solid, Axe),
    block!("minecraft:spruce_leaves", 0.2, solid, Hoe),
    // ===== Wood — Birch =====
    block!("minecraft:birch_log", 2.0, solid, Axe),
    block!("minecraft:stripped_birch_log", 2.0, solid, Axe),
    block!("minecraft:birch_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_birch_wood", 2.0, solid, Axe),
    block!("minecraft:birch_planks", 2.0, solid, Axe),
    block!("minecraft:birch_slab", 2.0, solid, Axe),
    block!("minecraft:birch_stairs", 2.0, solid, Axe),
    block!("minecraft:birch_fence", 2.0, solid, Axe),
    block!("minecraft:birch_fence_gate", 2.0, solid, Axe),
    block!("minecraft:birch_door", 3.0, solid, Axe),
    block!("minecraft:birch_trapdoor", 3.0, solid, Axe),
    block!("minecraft:birch_leaves", 0.2, solid, Hoe),
    // ===== Wood — Jungle =====
    block!("minecraft:jungle_log", 2.0, solid, Axe),
    block!("minecraft:stripped_jungle_log", 2.0, solid, Axe),
    block!("minecraft:jungle_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_jungle_wood", 2.0, solid, Axe),
    block!("minecraft:jungle_planks", 2.0, solid, Axe),
    block!("minecraft:jungle_slab", 2.0, solid, Axe),
    block!("minecraft:jungle_stairs", 2.0, solid, Axe),
    block!("minecraft:jungle_fence", 2.0, solid, Axe),
    block!("minecraft:jungle_fence_gate", 2.0, solid, Axe),
    block!("minecraft:jungle_door", 3.0, solid, Axe),
    block!("minecraft:jungle_trapdoor", 3.0, solid, Axe),
    block!("minecraft:jungle_leaves", 0.2, solid, Hoe),
    // ===== Wood — Acacia =====
    block!("minecraft:acacia_log", 2.0, solid, Axe),
    block!("minecraft:stripped_acacia_log", 2.0, solid, Axe),
    block!("minecraft:acacia_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_acacia_wood", 2.0, solid, Axe),
    block!("minecraft:acacia_planks", 2.0, solid, Axe),
    block!("minecraft:acacia_slab", 2.0, solid, Axe),
    block!("minecraft:acacia_stairs", 2.0, solid, Axe),
    block!("minecraft:acacia_fence", 2.0, solid, Axe),
    block!("minecraft:acacia_fence_gate", 2.0, solid, Axe),
    block!("minecraft:acacia_door", 3.0, solid, Axe),
    block!("minecraft:acacia_trapdoor", 3.0, solid, Axe),
    block!("minecraft:acacia_leaves", 0.2, solid, Hoe),
    // ===== Wood — Dark Oak =====
    block!("minecraft:dark_oak_log", 2.0, solid, Axe),
    block!("minecraft:stripped_dark_oak_log", 2.0, solid, Axe),
    block!("minecraft:dark_oak_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_dark_oak_wood", 2.0, solid, Axe),
    block!("minecraft:dark_oak_planks", 2.0, solid, Axe),
    block!("minecraft:dark_oak_slab", 2.0, solid, Axe),
    block!("minecraft:dark_oak_stairs", 2.0, solid, Axe),
    block!("minecraft:dark_oak_fence", 2.0, solid, Axe),
    block!("minecraft:dark_oak_fence_gate", 2.0, solid, Axe),
    block!("minecraft:dark_oak_door", 3.0, solid, Axe),
    block!("minecraft:dark_oak_trapdoor", 3.0, solid, Axe),
    block!("minecraft:dark_oak_leaves", 0.2, solid, Hoe),
    // ===== Wood — Mangrove =====
    block!("minecraft:mangrove_log", 2.0, solid, Axe),
    block!("minecraft:stripped_mangrove_log", 2.0, solid, Axe),
    block!("minecraft:mangrove_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_mangrove_wood", 2.0, solid, Axe),
    block!("minecraft:mangrove_planks", 2.0, solid, Axe),
    block!("minecraft:mangrove_slab", 2.0, solid, Axe),
    block!("minecraft:mangrove_stairs", 2.0, solid, Axe),
    block!("minecraft:mangrove_fence", 2.0, solid, Axe),
    block!("minecraft:mangrove_fence_gate", 2.0, solid, Axe),
    block!("minecraft:mangrove_door", 3.0, solid, Axe),
    block!("minecraft:mangrove_trapdoor", 3.0, solid, Axe),
    block!("minecraft:mangrove_leaves", 0.2, solid, Hoe),
    block!("minecraft:mangrove_roots", 0.7, solid, Axe),
    block!("minecraft:muddy_mangrove_roots", 0.7, solid, Shovel),
    // ===== Wood — Cherry =====
    block!("minecraft:cherry_log", 2.0, solid, Axe),
    block!("minecraft:stripped_cherry_log", 2.0, solid, Axe),
    block!("minecraft:cherry_wood", 2.0, solid, Axe),
    block!("minecraft:stripped_cherry_wood", 2.0, solid, Axe),
    block!("minecraft:cherry_planks", 2.0, solid, Axe),
    block!("minecraft:cherry_slab", 2.0, solid, Axe),
    block!("minecraft:cherry_stairs", 2.0, solid, Axe),
    block!("minecraft:cherry_fence", 2.0, solid, Axe),
    block!("minecraft:cherry_fence_gate", 2.0, solid, Axe),
    block!("minecraft:cherry_door", 3.0, solid, Axe),
    block!("minecraft:cherry_trapdoor", 3.0, solid, Axe),
    block!("minecraft:cherry_leaves", 0.2, solid, Hoe),
    // ===== Wood — Bamboo =====
    block!("minecraft:bamboo_block", 2.0, solid, Axe),
    block!("minecraft:stripped_bamboo_block", 2.0, solid, Axe),
    block!("minecraft:bamboo_planks", 2.0, solid, Axe),
    block!("minecraft:bamboo_slab", 2.0, solid, Axe),
    block!("minecraft:bamboo_stairs", 2.0, solid, Axe),
    block!("minecraft:bamboo_fence", 2.0, solid, Axe),
    block!("minecraft:bamboo_fence_gate", 2.0, solid, Axe),
    block!("minecraft:bamboo_door", 3.0, solid, Axe),
    block!("minecraft:bamboo_trapdoor", 3.0, solid, Axe),
    block!("minecraft:bamboo_mosaic", 2.0, solid, Axe),
    block!("minecraft:bamboo_mosaic_slab", 2.0, solid, Axe),
    block!("minecraft:bamboo_mosaic_stairs", 2.0, solid, Axe),
    block!("minecraft:bamboo", 1.0, solid, Axe),
    // ===== Wood — Warped / Crimson (Nether) =====
    block!("minecraft:warped_stem", 2.0, solid, Axe),
    block!("minecraft:stripped_warped_stem", 2.0, solid, Axe),
    block!("minecraft:warped_hyphae", 2.0, solid, Axe),
    block!("minecraft:stripped_warped_hyphae", 2.0, solid, Axe),
    block!("minecraft:warped_planks", 2.0, solid, Axe),
    block!("minecraft:warped_slab", 2.0, solid, Axe),
    block!("minecraft:warped_stairs", 2.0, solid, Axe),
    block!("minecraft:warped_fence", 2.0, solid, Axe),
    block!("minecraft:warped_fence_gate", 2.0, solid, Axe),
    block!("minecraft:warped_door", 3.0, solid, Axe),
    block!("minecraft:warped_trapdoor", 3.0, solid, Axe),
    block!("minecraft:crimson_stem", 2.0, solid, Axe),
    block!("minecraft:stripped_crimson_stem", 2.0, solid, Axe),
    block!("minecraft:crimson_hyphae", 2.0, solid, Axe),
    block!("minecraft:stripped_crimson_hyphae", 2.0, solid, Axe),
    block!("minecraft:crimson_planks", 2.0, solid, Axe),
    block!("minecraft:crimson_slab", 2.0, solid, Axe),
    block!("minecraft:crimson_stairs", 2.0, solid, Axe),
    block!("minecraft:crimson_fence", 2.0, solid, Axe),
    block!("minecraft:crimson_fence_gate", 2.0, solid, Axe),
    block!("minecraft:crimson_door", 3.0, solid, Axe),
    block!("minecraft:crimson_trapdoor", 3.0, solid, Axe),
    // ===== Glass =====
    block!("minecraft:glass", 0.3, solid, None),
    block!("minecraft:glass_pane", 0.3, solid, None),
    block!("minecraft:tinted_glass", 0.3, solid, None),
    block!("minecraft:white_stained_glass", 0.3, solid, None),
    block!("minecraft:orange_stained_glass", 0.3, solid, None),
    block!("minecraft:magenta_stained_glass", 0.3, solid, None),
    block!("minecraft:light_blue_stained_glass", 0.3, solid, None),
    block!("minecraft:yellow_stained_glass", 0.3, solid, None),
    block!("minecraft:lime_stained_glass", 0.3, solid, None),
    block!("minecraft:pink_stained_glass", 0.3, solid, None),
    block!("minecraft:gray_stained_glass", 0.3, solid, None),
    block!("minecraft:light_gray_stained_glass", 0.3, solid, None),
    block!("minecraft:cyan_stained_glass", 0.3, solid, None),
    block!("minecraft:purple_stained_glass", 0.3, solid, None),
    block!("minecraft:blue_stained_glass", 0.3, solid, None),
    block!("minecraft:brown_stained_glass", 0.3, solid, None),
    block!("minecraft:green_stained_glass", 0.3, solid, None),
    block!("minecraft:red_stained_glass", 0.3, solid, None),
    block!("minecraft:black_stained_glass", 0.3, solid, None),
    block!("minecraft:white_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:orange_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:magenta_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:light_blue_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:yellow_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:lime_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:pink_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:gray_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:light_gray_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:cyan_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:purple_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:blue_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:brown_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:green_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:red_stained_glass_pane", 0.3, solid, None),
    block!("minecraft:black_stained_glass_pane", 0.3, solid, None),
    // ===== Wool =====
    block!("minecraft:white_wool", 0.8, solid, Shears),
    block!("minecraft:orange_wool", 0.8, solid, Shears),
    block!("minecraft:magenta_wool", 0.8, solid, Shears),
    block!("minecraft:light_blue_wool", 0.8, solid, Shears),
    block!("minecraft:yellow_wool", 0.8, solid, Shears),
    block!("minecraft:lime_wool", 0.8, solid, Shears),
    block!("minecraft:pink_wool", 0.8, solid, Shears),
    block!("minecraft:gray_wool", 0.8, solid, Shears),
    block!("minecraft:light_gray_wool", 0.8, solid, Shears),
    block!("minecraft:cyan_wool", 0.8, solid, Shears),
    block!("minecraft:purple_wool", 0.8, solid, Shears),
    block!("minecraft:blue_wool", 0.8, solid, Shears),
    block!("minecraft:brown_wool", 0.8, solid, Shears),
    block!("minecraft:green_wool", 0.8, solid, Shears),
    block!("minecraft:red_wool", 0.8, solid, Shears),
    block!("minecraft:black_wool", 0.8, solid, Shears),
    // ===== Carpet =====
    block!("minecraft:white_carpet", 0.1, non_solid, None),
    block!("minecraft:orange_carpet", 0.1, non_solid, None),
    block!("minecraft:magenta_carpet", 0.1, non_solid, None),
    block!("minecraft:light_blue_carpet", 0.1, non_solid, None),
    block!("minecraft:yellow_carpet", 0.1, non_solid, None),
    block!("minecraft:lime_carpet", 0.1, non_solid, None),
    block!("minecraft:pink_carpet", 0.1, non_solid, None),
    block!("minecraft:gray_carpet", 0.1, non_solid, None),
    block!("minecraft:light_gray_carpet", 0.1, non_solid, None),
    block!("minecraft:cyan_carpet", 0.1, non_solid, None),
    block!("minecraft:purple_carpet", 0.1, non_solid, None),
    block!("minecraft:blue_carpet", 0.1, non_solid, None),
    block!("minecraft:brown_carpet", 0.1, non_solid, None),
    block!("minecraft:green_carpet", 0.1, non_solid, None),
    block!("minecraft:red_carpet", 0.1, non_solid, None),
    block!("minecraft:black_carpet", 0.1, non_solid, None),
    // ===== Concrete =====
    block!("minecraft:white_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:orange_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:magenta_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:light_blue_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:yellow_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:lime_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:pink_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:gray_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:light_gray_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:cyan_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:purple_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:blue_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:brown_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:green_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:red_concrete", 1.8, solid, Pickaxe),
    block!("minecraft:black_concrete", 1.8, solid, Pickaxe),
    // ===== Concrete powder =====
    block!("minecraft:white_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:orange_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:magenta_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:light_blue_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:yellow_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:lime_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:pink_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:gray_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:light_gray_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:cyan_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:purple_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:blue_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:brown_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:green_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:red_concrete_powder", 0.5, solid, Shovel),
    block!("minecraft:black_concrete_powder", 0.5, solid, Shovel),
    // ===== Terracotta =====
    block!("minecraft:hardened_clay", 1.25, solid, Pickaxe),
    block!("minecraft:white_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:orange_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:magenta_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:light_blue_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:yellow_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:lime_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:pink_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:gray_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:light_gray_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:cyan_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:purple_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:blue_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:brown_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:green_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:red_terracotta", 1.25, solid, Pickaxe),
    block!("minecraft:black_terracotta", 1.25, solid, Pickaxe),
    // ===== Glazed terracotta =====
    block!("minecraft:white_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:orange_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:magenta_glazed_terracotta", 1.4, solid, Pickaxe),
    block!(
        "minecraft:light_blue_glazed_terracotta",
        1.4,
        solid,
        Pickaxe
    ),
    block!("minecraft:yellow_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:lime_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:pink_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:gray_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:silver_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:cyan_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:purple_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:blue_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:brown_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:green_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:red_glazed_terracotta", 1.4, solid, Pickaxe),
    block!("minecraft:black_glazed_terracotta", 1.4, solid, Pickaxe),
    // ===== Bricks =====
    block!("minecraft:brick_block", 2.0, solid, Pickaxe),
    block!("minecraft:nether_brick", 2.0, solid, Pickaxe),
    block!("minecraft:red_nether_brick", 2.0, solid, Pickaxe),
    block!("minecraft:chiseled_nether_bricks", 2.0, solid, Pickaxe),
    block!("minecraft:cracked_nether_bricks", 2.0, solid, Pickaxe),
    block!("minecraft:prismarine", 1.5, solid, Pickaxe),
    block!("minecraft:dark_prismarine", 1.5, solid, Pickaxe),
    block!("minecraft:prismarine_bricks", 1.5, solid, Pickaxe),
    block!("minecraft:sea_lantern", 0.3, solid, None),
    // ===== Nether =====
    block!("minecraft:netherrack", 0.4, solid, Pickaxe),
    block!("minecraft:nether_wart_block", 1.0, solid, Hoe),
    block!("minecraft:warped_wart_block", 1.0, solid, Hoe),
    block!("minecraft:glowstone", 0.3, solid, None),
    block!("minecraft:shroomlight", 1.0, solid, Hoe),
    block!("minecraft:magma", 0.5, solid, Pickaxe),
    block!("minecraft:crying_obsidian", 50.0, solid, Pickaxe),
    block!("minecraft:respawn_anchor", 50.0, solid, Pickaxe),
    // ===== End =====
    block!("minecraft:end_stone", 3.0, solid, Pickaxe),
    block!("minecraft:end_stone_bricks", 3.0, solid, Pickaxe),
    block!("minecraft:end_bricks", 3.0, solid, Pickaxe),
    block!("minecraft:purpur_block", 1.5, solid, Pickaxe),
    block!("minecraft:purpur_pillar", 1.5, solid, Pickaxe),
    block!("minecraft:purpur_stairs", 1.5, solid, Pickaxe),
    block!("minecraft:end_rod", 0.0, non_solid, None),
    block!("minecraft:chorus_plant", 0.4, solid, Axe),
    block!("minecraft:chorus_flower", 0.4, solid, Axe),
    // ===== Quartz =====
    block!("minecraft:quartz_block", 0.8, solid, Pickaxe),
    block!("minecraft:chiseled_quartz_block", 0.8, solid, Pickaxe),
    block!("minecraft:quartz_pillar", 0.8, solid, Pickaxe),
    block!("minecraft:smooth_quartz", 2.0, solid, Pickaxe),
    block!("minecraft:quartz_bricks", 0.8, solid, Pickaxe),
    block!("minecraft:quartz_stairs", 0.8, solid, Pickaxe),
    block!("minecraft:quartz_slab", 0.8, solid, Pickaxe),
    // ===== Sandstone =====
    block!("minecraft:sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:chiseled_sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:cut_sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:smooth_sandstone", 2.0, solid, Pickaxe),
    block!("minecraft:red_sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:chiseled_red_sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:cut_red_sandstone", 0.8, solid, Pickaxe),
    block!("minecraft:smooth_red_sandstone", 2.0, solid, Pickaxe),
    block!("minecraft:sandstone_stairs", 0.8, solid, Pickaxe),
    block!("minecraft:red_sandstone_stairs", 0.8, solid, Pickaxe),
    // ===== Obsidian =====
    block!("minecraft:obsidian", 50.0, solid, Pickaxe),
    // ===== Liquids =====
    block!("minecraft:water", 100.0, non_solid, None),
    block!("minecraft:flowing_water", 100.0, non_solid, None),
    block!("minecraft:lava", 100.0, non_solid, None),
    block!("minecraft:flowing_lava", 100.0, non_solid, None),
    // ===== Plants =====
    block!("minecraft:short_grass", 0.0, non_solid, None),
    block!("minecraft:tallgrass", 0.0, non_solid, None),
    block!("minecraft:tall_grass", 0.0, non_solid, None),
    block!("minecraft:fern", 0.0, non_solid, None),
    block!("minecraft:large_fern", 0.0, non_solid, None),
    block!("minecraft:dead_bush", 0.0, non_solid, None),
    block!("minecraft:seagrass", 0.0, non_solid, None),
    block!("minecraft:tall_seagrass", 0.0, non_solid, None),
    block!("minecraft:dandelion", 0.0, non_solid, None),
    block!("minecraft:poppy", 0.0, non_solid, None),
    block!("minecraft:blue_orchid", 0.0, non_solid, None),
    block!("minecraft:allium", 0.0, non_solid, None),
    block!("minecraft:azure_bluet", 0.0, non_solid, None),
    block!("minecraft:red_tulip", 0.0, non_solid, None),
    block!("minecraft:orange_tulip", 0.0, non_solid, None),
    block!("minecraft:white_tulip", 0.0, non_solid, None),
    block!("minecraft:pink_tulip", 0.0, non_solid, None),
    block!("minecraft:oxeye_daisy", 0.0, non_solid, None),
    block!("minecraft:cornflower", 0.0, non_solid, None),
    block!("minecraft:lily_of_the_valley", 0.0, non_solid, None),
    block!("minecraft:sunflower", 0.0, non_solid, None),
    block!("minecraft:lilac", 0.0, non_solid, None),
    block!("minecraft:rose_bush", 0.0, non_solid, None),
    block!("minecraft:peony", 0.0, non_solid, None),
    block!("minecraft:wither_rose", 0.0, non_solid, None),
    block!("minecraft:torchflower", 0.0, non_solid, None),
    block!("minecraft:pitcher_plant", 0.0, non_solid, None),
    block!("minecraft:pink_petals", 0.0, non_solid, None),
    block!("minecraft:spore_blossom", 0.0, non_solid, None),
    block!("minecraft:sugar_cane", 0.0, non_solid, None),
    block!("minecraft:cactus", 0.4, solid, None),
    block!("minecraft:vine", 0.2, non_solid, Shears),
    block!("minecraft:glow_lichen", 0.2, non_solid, Shears),
    block!("minecraft:lily_pad", 0.0, non_solid, None),
    block!("minecraft:kelp", 0.0, non_solid, None),
    block!("minecraft:hanging_roots", 0.0, non_solid, None),
    block!("minecraft:moss_block", 0.1, solid, Hoe),
    block!("minecraft:moss_carpet", 0.1, non_solid, Hoe),
    block!("minecraft:azalea", 0.0, non_solid, None),
    block!("minecraft:flowering_azalea", 0.0, non_solid, None),
    block!("minecraft:azalea_leaves", 0.2, solid, Hoe),
    block!("minecraft:azalea_leaves_flowered", 0.2, solid, Hoe),
    block!("minecraft:dripleaf", 0.1, solid, None),
    block!("minecraft:small_dripleaf_block", 0.1, non_solid, None),
    // ===== Mushrooms =====
    block!("minecraft:brown_mushroom", 0.0, non_solid, None),
    block!("minecraft:red_mushroom", 0.0, non_solid, None),
    block!("minecraft:brown_mushroom_block", 0.2, solid, Axe),
    block!("minecraft:red_mushroom_block", 0.2, solid, Axe),
    block!("minecraft:mushroom_stem", 0.2, solid, Axe),
    block!("minecraft:crimson_fungus", 0.0, non_solid, None),
    block!("minecraft:warped_fungus", 0.0, non_solid, None),
    block!("minecraft:crimson_roots", 0.0, non_solid, None),
    block!("minecraft:warped_roots", 0.0, non_solid, None),
    block!("minecraft:nether_sprouts", 0.0, non_solid, None),
    block!("minecraft:twisting_vines", 0.0, non_solid, None),
    block!("minecraft:weeping_vines", 0.0, non_solid, None),
    block!("minecraft:sculk_vein", 0.2, non_solid, Hoe),
    // ===== Sculk =====
    block!("minecraft:sculk", 0.2, solid, Hoe),
    block!("minecraft:sculk_catalyst", 3.0, solid, Hoe),
    block!("minecraft:sculk_shrieker", 3.0, solid, Hoe),
    block!("minecraft:sculk_sensor", 1.5, solid, Hoe),
    block!("minecraft:calibrated_sculk_sensor", 1.5, solid, Hoe),
    // ===== Crops =====
    block!("minecraft:wheat", 0.0, non_solid, None),
    block!("minecraft:carrots", 0.0, non_solid, None),
    block!("minecraft:potatoes", 0.0, non_solid, None),
    block!("minecraft:beetroot", 0.0, non_solid, None),
    block!("minecraft:melon_block", 1.0, solid, Axe),
    block!("minecraft:pumpkin", 1.0, solid, Axe),
    block!("minecraft:carved_pumpkin", 1.0, solid, Axe),
    block!("minecraft:lit_pumpkin", 1.0, solid, Axe),
    block!("minecraft:melon_stem", 0.0, non_solid, None),
    block!("minecraft:pumpkin_stem", 0.0, non_solid, None),
    block!("minecraft:sweet_berry_bush", 0.0, non_solid, None),
    block!("minecraft:cocoa", 0.2, solid, Axe),
    block!("minecraft:nether_wart", 0.0, non_solid, None),
    block!("minecraft:torchflower_crop", 0.0, non_solid, None),
    block!("minecraft:pitcher_crop", 0.0, non_solid, None),
    // ===== Utility blocks =====
    block!("minecraft:crafting_table", 2.5, solid, Axe),
    block!("minecraft:furnace", 3.5, solid, Pickaxe),
    block!("minecraft:lit_furnace", 3.5, solid, Pickaxe),
    block!("minecraft:blast_furnace", 3.5, solid, Pickaxe),
    block!("minecraft:lit_blast_furnace", 3.5, solid, Pickaxe),
    block!("minecraft:smoker", 3.5, solid, Pickaxe),
    block!("minecraft:lit_smoker", 3.5, solid, Pickaxe),
    block!("minecraft:chest", 2.5, solid, Axe),
    block!("minecraft:trapped_chest", 2.5, solid, Axe),
    block!("minecraft:ender_chest", 22.5, solid, Pickaxe),
    block!("minecraft:barrel", 2.5, solid, Axe),
    block!("minecraft:anvil", 5.0, solid, Pickaxe),
    block!("minecraft:enchanting_table", 5.0, solid, Pickaxe),
    block!("minecraft:brewing_stand", 0.5, solid, Pickaxe),
    block!("minecraft:cauldron", 2.0, solid, Pickaxe),
    block!("minecraft:loom", 2.5, solid, Axe),
    block!("minecraft:cartography_table", 2.5, solid, Axe),
    block!("minecraft:fletching_table", 2.5, solid, Axe),
    block!("minecraft:smithing_table", 2.5, solid, Axe),
    block!("minecraft:grindstone", 2.0, solid, Pickaxe),
    block!("minecraft:stonecutter_block", 3.5, solid, Pickaxe),
    block!("minecraft:composter", 0.6, solid, Axe),
    block!("minecraft:lectern", 2.5, solid, Axe),
    block!("minecraft:bookshelf", 1.5, solid, Axe),
    block!("minecraft:chiseled_bookshelf", 1.5, solid, Axe),
    block!("minecraft:decorated_pot", 0.0, solid, None),
    block!("minecraft:jukebox", 2.0, solid, Axe),
    block!("minecraft:note_block", 0.8, solid, Axe),
    block!("minecraft:bell", 5.0, solid, Pickaxe),
    block!("minecraft:beacon", 3.0, solid, None),
    block!("minecraft:conduit", 3.0, solid, None),
    // ===== Redstone =====
    block!("minecraft:redstone_wire", 0.0, non_solid, None),
    block!("minecraft:redstone_torch", 0.0, non_solid, None),
    block!("minecraft:unlit_redstone_torch", 0.0, non_solid, None),
    block!("minecraft:unpowered_repeater", 0.0, non_solid, None),
    block!("minecraft:powered_repeater", 0.0, non_solid, None),
    block!("minecraft:unpowered_comparator", 0.0, non_solid, None),
    block!("minecraft:powered_comparator", 0.0, non_solid, None),
    block!("minecraft:lever", 0.5, non_solid, None),
    block!("minecraft:stone_button", 0.5, non_solid, None),
    block!("minecraft:stone_pressure_plate", 0.5, solid, Pickaxe),
    block!(
        "minecraft:heavy_weighted_pressure_plate",
        0.5,
        solid,
        Pickaxe
    ),
    block!(
        "minecraft:light_weighted_pressure_plate",
        0.5,
        solid,
        Pickaxe
    ),
    block!("minecraft:piston", 1.5, solid, Pickaxe),
    block!("minecraft:sticky_piston", 1.5, solid, Pickaxe),
    block!("minecraft:observer", 3.5, solid, Pickaxe),
    block!("minecraft:daylight_detector", 0.2, solid, Axe),
    block!("minecraft:daylight_detector_inverted", 0.2, solid, Axe),
    block!("minecraft:tripwire_hook", 0.0, non_solid, None),
    block!("minecraft:trip_wire", 0.0, non_solid, None),
    block!("minecraft:target", 0.5, solid, Hoe),
    block!("minecraft:dropper", 3.5, solid, Pickaxe),
    block!("minecraft:dispenser", 3.5, solid, Pickaxe),
    block!("minecraft:hopper", 3.0, solid, Pickaxe),
    block!("minecraft:tnt", 0.0, solid, None),
    block!("minecraft:lightning_rod", 3.0, solid, Pickaxe),
    // ===== Torches & lighting =====
    block!("minecraft:torch", 0.0, non_solid, None),
    block!("minecraft:soul_torch", 0.0, non_solid, None),
    block!("minecraft:lantern", 3.5, solid, Pickaxe),
    block!("minecraft:soul_lantern", 3.5, solid, Pickaxe),
    block!("minecraft:candle", 0.1, non_solid, None),
    block!("minecraft:white_candle", 0.1, non_solid, None),
    block!("minecraft:orange_candle", 0.1, non_solid, None),
    block!("minecraft:magenta_candle", 0.1, non_solid, None),
    block!("minecraft:light_blue_candle", 0.1, non_solid, None),
    block!("minecraft:yellow_candle", 0.1, non_solid, None),
    block!("minecraft:lime_candle", 0.1, non_solid, None),
    block!("minecraft:pink_candle", 0.1, non_solid, None),
    block!("minecraft:gray_candle", 0.1, non_solid, None),
    block!("minecraft:light_gray_candle", 0.1, non_solid, None),
    block!("minecraft:cyan_candle", 0.1, non_solid, None),
    block!("minecraft:purple_candle", 0.1, non_solid, None),
    block!("minecraft:blue_candle", 0.1, non_solid, None),
    block!("minecraft:brown_candle", 0.1, non_solid, None),
    block!("minecraft:green_candle", 0.1, non_solid, None),
    block!("minecraft:red_candle", 0.1, non_solid, None),
    block!("minecraft:black_candle", 0.1, non_solid, None),
    // ===== Rails =====
    block!("minecraft:rail", 0.7, non_solid, Pickaxe),
    block!("minecraft:golden_rail", 0.7, non_solid, Pickaxe),
    block!("minecraft:detector_rail", 0.7, non_solid, Pickaxe),
    block!("minecraft:activator_rail", 0.7, non_solid, Pickaxe),
    // ===== Beds =====
    block!("minecraft:bed", 0.2, solid, None),
    // ===== Snow, ice =====
    block!("minecraft:snow", 0.1, non_solid, Shovel),
    block!("minecraft:snow_layer", 0.1, non_solid, Shovel),
    block!("minecraft:ice", 0.5, solid, Pickaxe),
    block!("minecraft:packed_ice", 0.5, solid, Pickaxe),
    block!("minecraft:blue_ice", 2.8, solid, Pickaxe),
    block!("minecraft:frosted_ice", 0.5, solid, Pickaxe),
    // ===== Misc building =====
    block!("minecraft:hay_block", 0.5, solid, Hoe),
    block!("minecraft:bone_block", 2.0, solid, Pickaxe),
    block!("minecraft:dried_kelp_block", 0.5, solid, Hoe),
    block!("minecraft:sponge", 0.6, solid, Hoe),
    block!("minecraft:wet_sponge", 0.6, solid, Hoe),
    block!("minecraft:slime", 0.0, solid, None),
    block!("minecraft:honey_block", 0.0, solid, None),
    block!("minecraft:honeycomb_block", 0.6, solid, None),
    block!("minecraft:web", 4.0, non_solid, Sword),
    block!("minecraft:ladder", 0.4, non_solid, Axe),
    block!("minecraft:scaffolding", 0.0, non_solid, None),
    block!("minecraft:chain", 5.0, solid, Pickaxe),
    block!("minecraft:iron_bars", 5.0, solid, Pickaxe),
    block!("minecraft:iron_door", 5.0, solid, Pickaxe),
    block!("minecraft:iron_trapdoor", 5.0, solid, Pickaxe),
    block!("minecraft:cobblestone_wall", 2.0, solid, Pickaxe),
    block!("minecraft:mossy_cobblestone_wall", 2.0, solid, Pickaxe),
    block!("minecraft:stone_brick_wall", 1.5, solid, Pickaxe),
    block!("minecraft:nether_brick_fence", 2.0, solid, Pickaxe),
    block!("minecraft:brick_stairs", 2.0, solid, Pickaxe),
    block!("minecraft:stone_stairs", 1.5, solid, Pickaxe),
    block!("minecraft:stone_brick_stairs", 1.5, solid, Pickaxe),
    block!("minecraft:cobblestone_stairs", 2.0, solid, Pickaxe),
    block!("minecraft:nether_brick_stairs", 2.0, solid, Pickaxe),
    block!("minecraft:smooth_stone_slab", 2.0, solid, Pickaxe),
    // ===== Spawner, end portal, etc. =====
    block!("minecraft:mob_spawner", 5.0, solid, Pickaxe),
    block!("minecraft:end_portal_frame", -1.0, solid, None),
    block!("minecraft:end_portal", -1.0, non_solid, None),
    block!("minecraft:end_gateway", -1.0, non_solid, None),
    block!("minecraft:nether_portal", -1.0, non_solid, None),
    block!("minecraft:dragon_egg", 3.0, solid, None),
    // ===== Signs, banners =====
    block!("minecraft:standing_banner", 1.0, non_solid, Axe),
    block!("minecraft:wall_banner", 1.0, non_solid, Axe),
    // ===== Skulls =====
    block!("minecraft:skull", 1.0, non_solid, None),
    // ===== Amethyst clusters =====
    block!("minecraft:small_amethyst_bud", 1.5, solid, None),
    block!("minecraft:medium_amethyst_bud", 1.5, solid, None),
    block!("minecraft:large_amethyst_bud", 1.5, solid, None),
    block!("minecraft:amethyst_cluster", 1.5, solid, None),
    // ===== Command blocks =====
    block!("minecraft:command_block", -1.0, solid, None),
    block!("minecraft:chain_command_block", -1.0, solid, None),
    block!("minecraft:repeating_command_block", -1.0, solid, None),
    block!("minecraft:structure_block", -1.0, solid, None),
    block!("minecraft:jigsaw", -1.0, solid, None),
    // ===== Miscellaneous =====
    block!("minecraft:cake", 0.5, solid, None),
    block!("minecraft:flower_pot", 0.0, non_solid, None),
    block!("minecraft:item_frame", 0.0, non_solid, None),
    block!("minecraft:glow_frame", 0.0, non_solid, None),
    block!("minecraft:painting", 0.0, non_solid, None),
    block!("minecraft:fire", 0.0, non_solid, None),
    block!("minecraft:soul_fire", 0.0, non_solid, None),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_hash::hash_block_state;

    #[test]
    fn known_block_hardness() {
        let registry = BlockRegistry::new();
        let stone = hash_block_state("minecraft:stone");
        assert_eq!(registry.hardness(stone), Some(1.5));

        let bedrock = hash_block_state("minecraft:bedrock");
        assert_eq!(registry.hardness(bedrock), Some(-1.0));

        let air = hash_block_state("minecraft:air");
        assert_eq!(registry.hardness(air), Some(0.0));

        let dirt = hash_block_state("minecraft:dirt");
        assert_eq!(registry.hardness(dirt), Some(0.5));
    }

    #[test]
    fn air_is_not_solid() {
        let registry = BlockRegistry::new();
        let air = hash_block_state("minecraft:air");
        assert!(!registry.is_solid(air));
    }

    #[test]
    fn water_is_not_solid() {
        let registry = BlockRegistry::new();
        let water = hash_block_state("minecraft:water");
        assert!(!registry.is_solid(water));
    }

    #[test]
    fn stone_is_solid() {
        let registry = BlockRegistry::new();
        let stone = hash_block_state("minecraft:stone");
        assert!(registry.is_solid(stone));
    }

    #[test]
    fn unknown_block_defaults_solid() {
        let registry = BlockRegistry::new();
        assert!(registry.is_solid(0xDEADBEEF));
    }

    #[test]
    fn unknown_block_no_hardness() {
        let registry = BlockRegistry::new();
        assert_eq!(registry.hardness(0xDEADBEEF), None);
    }

    #[test]
    fn flat_world_blocks_present() {
        let registry = BlockRegistry::new();
        let flat = crate::block_hash::FlatWorldBlocks::compute();
        assert!(registry.get(flat.air).is_some());
        assert!(registry.get(flat.bedrock).is_some());
        assert!(registry.get(flat.dirt).is_some());
        assert!(registry.get(flat.grass_block).is_some());
    }

    #[test]
    fn no_hash_collisions() {
        let mut seen = std::collections::HashMap::new();
        for info in BLOCK_DATA {
            let hash = hash_block_state(info.name);
            if let Some(existing) = seen.insert(hash, info.name) {
                panic!(
                    "Hash collision: {} and {} both hash to {:#010X}",
                    existing, info.name, hash
                );
            }
        }
    }

    #[test]
    fn expected_mining_time() {
        let registry = BlockRegistry::new();
        // Stone: hardness 1.5, Pickaxe → hand mining = 1.5 * 5.0 = 7.5s
        let stone = hash_block_state("minecraft:stone");
        assert_eq!(registry.expected_mining_secs(stone), Some(7.5));

        // Dirt: hardness 0.5, Shovel → hand mining = 0.5 * 5.0 = 2.5s
        let dirt = hash_block_state("minecraft:dirt");
        assert_eq!(registry.expected_mining_secs(dirt), Some(2.5));

        // Glass: hardness 0.3, None → hand mining = 0.3 * 1.5 ≈ 0.45s
        let glass = hash_block_state("minecraft:glass");
        let glass_time = registry.expected_mining_secs(glass).unwrap();
        assert!((glass_time - 0.45).abs() < 0.001);

        // Air: hardness 0.0 → instant
        let air = hash_block_state("minecraft:air");
        assert_eq!(registry.expected_mining_secs(air), Some(0.0));
    }

    #[test]
    fn block_count() {
        // Verify we have a substantial number of blocks registered
        let registry = BlockRegistry::new();
        assert!(
            registry.blocks.len() >= 300,
            "Expected >= 300 blocks, got {}",
            registry.blocks.len()
        );
    }
}
