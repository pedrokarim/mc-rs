//! Smelting recipes, fuel values, and furnace types.
//!
//! Covers all three furnace variants: standard furnace, blast furnace, and smoker.

use std::collections::HashMap;

/// Furnace variant type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceType {
    Furnace,
    BlastFurnace,
    Smoker,
}

impl FurnaceType {
    /// Block entity ID string for NBT serialization.
    pub fn nbt_id(&self) -> &'static str {
        match self {
            FurnaceType::Furnace => "Furnace",
            FurnaceType::BlastFurnace => "BlastFurnace",
            FurnaceType::Smoker => "Smoker",
        }
    }

    /// CraftingData recipe tag string.
    pub fn recipe_tag(&self) -> &'static str {
        match self {
            FurnaceType::Furnace => "furnace",
            FurnaceType::BlastFurnace => "blast_furnace",
            FurnaceType::Smoker => "smoker",
        }
    }

    /// Bedrock container type ID for ContainerOpen.
    pub fn container_type(&self) -> u8 {
        match self {
            FurnaceType::Furnace => 2,
            FurnaceType::BlastFurnace => 27,
            FurnaceType::Smoker => 28,
        }
    }

    /// Cook time in ticks (200 = 10s for furnace, 100 = 5s for blast/smoker).
    pub fn cook_time(&self) -> i16 {
        match self {
            FurnaceType::Furnace => 200,
            FurnaceType::BlastFurnace | FurnaceType::Smoker => 100,
        }
    }

    /// Parse from NBT id string.
    pub fn from_nbt_id(id: &str) -> Option<Self> {
        match id {
            "Furnace" => Some(FurnaceType::Furnace),
            "BlastFurnace" => Some(FurnaceType::BlastFurnace),
            "Smoker" => Some(FurnaceType::Smoker),
            _ => None,
        }
    }
}

/// A smelting recipe (input → output).
#[derive(Debug, Clone)]
pub struct SmeltingRecipe {
    pub input_name: String,
    pub input_metadata: i16,
    pub output_name: String,
    pub output_count: u8,
    pub output_metadata: u16,
    pub xp: f32,
    /// Which furnace types can use this recipe: "furnace", "blast_furnace", "smoker".
    pub tags: Vec<String>,
}

/// Registry of smelting recipes and fuel burn times.
pub struct SmeltingRegistry {
    recipes: Vec<SmeltingRecipe>,
    fuel_map: HashMap<String, u16>,
}

impl Default for SmeltingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SmeltingRegistry {
    /// Build the registry with vanilla recipes and fuel values.
    pub fn new() -> Self {
        let mut recipes = Vec::new();

        // Helper macros
        macro_rules! smelt {
            ($input:expr, $output:expr, $xp:expr, $tags:expr) => {
                recipes.push(SmeltingRecipe {
                    input_name: $input.to_string(),
                    input_metadata: 0,
                    output_name: $output.to_string(),
                    output_count: 1,
                    output_metadata: 0,
                    xp: $xp,
                    tags: $tags.iter().map(|s: &&str| s.to_string()).collect(),
                });
            };
        }

        // --- Ores (furnace + blast_furnace) ---
        let fb = &["furnace", "blast_furnace"];
        smelt!("minecraft:iron_ore", "minecraft:iron_ingot", 0.7, fb);
        smelt!("minecraft:gold_ore", "minecraft:gold_ingot", 1.0, fb);
        smelt!("minecraft:copper_ore", "minecraft:copper_ingot", 0.7, fb);
        smelt!(
            "minecraft:deepslate_iron_ore",
            "minecraft:iron_ingot",
            0.7,
            fb
        );
        smelt!(
            "minecraft:deepslate_gold_ore",
            "minecraft:gold_ingot",
            1.0,
            fb
        );
        smelt!(
            "minecraft:deepslate_copper_ore",
            "minecraft:copper_ingot",
            0.7,
            fb
        );
        smelt!(
            "minecraft:lapis_lazuli_ore",
            "minecraft:lapis_lazuli",
            0.2,
            fb
        );
        smelt!(
            "minecraft:deepslate_lapis_lazuli_ore",
            "minecraft:lapis_lazuli",
            0.2,
            fb
        );
        smelt!("minecraft:redstone_ore", "minecraft:redstone", 0.3, fb);
        smelt!(
            "minecraft:deepslate_redstone_ore",
            "minecraft:redstone",
            0.3,
            fb
        );
        smelt!("minecraft:diamond_ore", "minecraft:diamond", 1.0, fb);
        smelt!(
            "minecraft:deepslate_diamond_ore",
            "minecraft:diamond",
            1.0,
            fb
        );
        smelt!("minecraft:emerald_ore", "minecraft:emerald", 1.0, fb);
        smelt!(
            "minecraft:deepslate_emerald_ore",
            "minecraft:emerald",
            1.0,
            fb
        );
        smelt!("minecraft:coal_ore", "minecraft:coal", 0.1, fb);
        smelt!("minecraft:deepslate_coal_ore", "minecraft:coal", 0.1, fb);
        smelt!("minecraft:nether_gold_ore", "minecraft:gold_ingot", 1.0, fb);
        smelt!(
            "minecraft:ancient_debris",
            "minecraft:netherite_scrap",
            2.0,
            fb
        );

        // Raw metals (furnace + blast_furnace)
        smelt!("minecraft:raw_iron", "minecraft:iron_ingot", 0.7, fb);
        smelt!("minecraft:raw_gold", "minecraft:gold_ingot", 1.0, fb);
        smelt!("minecraft:raw_copper", "minecraft:copper_ingot", 0.7, fb);

        // --- Food (furnace + smoker) ---
        let fs = &["furnace", "smoker"];
        smelt!("minecraft:beef", "minecraft:cooked_beef", 0.35, fs);
        smelt!("minecraft:porkchop", "minecraft:cooked_porkchop", 0.35, fs);
        smelt!("minecraft:chicken", "minecraft:cooked_chicken", 0.35, fs);
        smelt!("minecraft:mutton", "minecraft:cooked_mutton", 0.35, fs);
        smelt!("minecraft:rabbit", "minecraft:cooked_rabbit", 0.35, fs);
        smelt!("minecraft:cod", "minecraft:cooked_cod", 0.35, fs);
        smelt!("minecraft:salmon", "minecraft:cooked_salmon", 0.35, fs);
        smelt!("minecraft:potato", "minecraft:baked_potato", 0.35, fs);
        smelt!("minecraft:kelp", "minecraft:dried_kelp", 0.1, fs);

        // --- Misc (furnace only) ---
        let f = &["furnace"];
        smelt!("minecraft:sand", "minecraft:glass", 0.1, f);
        smelt!("minecraft:cobblestone", "minecraft:stone", 0.1, f);
        smelt!("minecraft:stone", "minecraft:smooth_stone", 0.1, f);
        smelt!("minecraft:clay_ball", "minecraft:brick", 0.3, f);
        smelt!("minecraft:clay", "minecraft:hardened_clay", 0.35, f);
        smelt!("minecraft:netherrack", "minecraft:netherbrick", 0.1, f);
        smelt!("minecraft:cactus", "minecraft:green_dye", 1.0, f);
        smelt!("minecraft:wet_sponge", "minecraft:sponge", 0.15, f);

        // Logs → charcoal (furnace only, any log)
        for log in &[
            "minecraft:oak_log",
            "minecraft:spruce_log",
            "minecraft:birch_log",
            "minecraft:jungle_log",
            "minecraft:acacia_log",
            "minecraft:dark_oak_log",
        ] {
            smelt!(log, "minecraft:charcoal", 0.15, f);
        }

        // --- Fuel map ---
        let mut fuel_map = HashMap::new();
        fuel_map.insert("minecraft:coal".to_string(), 1600);
        fuel_map.insert("minecraft:charcoal".to_string(), 1600);
        fuel_map.insert("minecraft:coal_block".to_string(), 16000);
        fuel_map.insert("minecraft:oak_planks".to_string(), 300);
        fuel_map.insert("minecraft:spruce_planks".to_string(), 300);
        fuel_map.insert("minecraft:birch_planks".to_string(), 300);
        fuel_map.insert("minecraft:jungle_planks".to_string(), 300);
        fuel_map.insert("minecraft:acacia_planks".to_string(), 300);
        fuel_map.insert("minecraft:dark_oak_planks".to_string(), 300);
        fuel_map.insert("minecraft:oak_log".to_string(), 300);
        fuel_map.insert("minecraft:spruce_log".to_string(), 300);
        fuel_map.insert("minecraft:birch_log".to_string(), 300);
        fuel_map.insert("minecraft:jungle_log".to_string(), 300);
        fuel_map.insert("minecraft:acacia_log".to_string(), 300);
        fuel_map.insert("minecraft:dark_oak_log".to_string(), 300);
        fuel_map.insert("minecraft:stick".to_string(), 100);
        fuel_map.insert("minecraft:wooden_pickaxe".to_string(), 200);
        fuel_map.insert("minecraft:wooden_axe".to_string(), 200);
        fuel_map.insert("minecraft:wooden_shovel".to_string(), 200);
        fuel_map.insert("minecraft:wooden_sword".to_string(), 200);
        fuel_map.insert("minecraft:wooden_hoe".to_string(), 200);
        fuel_map.insert("minecraft:blaze_rod".to_string(), 2400);
        fuel_map.insert("minecraft:lava_bucket".to_string(), 20000);
        fuel_map.insert("minecraft:dried_kelp_block".to_string(), 4000);
        fuel_map.insert("minecraft:bamboo".to_string(), 50);
        fuel_map.insert("minecraft:scaffolding".to_string(), 50);
        fuel_map.insert("minecraft:carpet".to_string(), 67);
        fuel_map.insert("minecraft:wool".to_string(), 100);

        SmeltingRegistry { recipes, fuel_map }
    }

    /// Find a matching smelting recipe for the given input and furnace type.
    pub fn find_recipe(
        &self,
        input_name: &str,
        _input_meta: i16,
        furnace_type: FurnaceType,
    ) -> Option<&SmeltingRecipe> {
        let tag = furnace_type.recipe_tag();
        self.recipes.iter().find(|r| {
            r.input_name == input_name
                && r.tags.iter().any(|t| t == tag)
                && (r.input_metadata == -1 || r.input_metadata == _input_meta)
        })
    }

    /// Get the fuel burn time for an item (in ticks). Returns `None` if not a fuel.
    pub fn fuel_burn_time(&self, item_name: &str) -> Option<u16> {
        self.fuel_map.get(item_name).copied()
    }

    /// All smelting recipes (for CraftingData encoding).
    pub fn recipes(&self) -> &[SmeltingRecipe] {
        &self.recipes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_furnace_recipe() {
        let reg = SmeltingRegistry::new();
        let r = reg
            .find_recipe("minecraft:iron_ore", 0, FurnaceType::Furnace)
            .unwrap();
        assert_eq!(r.output_name, "minecraft:iron_ingot");
        assert!((r.xp - 0.7).abs() < 0.01);
    }

    #[test]
    fn find_blast_furnace_recipe() {
        let reg = SmeltingRegistry::new();
        let r = reg
            .find_recipe("minecraft:iron_ore", 0, FurnaceType::BlastFurnace)
            .unwrap();
        assert_eq!(r.output_name, "minecraft:iron_ingot");
    }

    #[test]
    fn smoker_cannot_smelt_ore() {
        let reg = SmeltingRegistry::new();
        assert!(reg
            .find_recipe("minecraft:iron_ore", 0, FurnaceType::Smoker)
            .is_none());
    }

    #[test]
    fn smoker_can_cook_food() {
        let reg = SmeltingRegistry::new();
        let r = reg
            .find_recipe("minecraft:beef", 0, FurnaceType::Smoker)
            .unwrap();
        assert_eq!(r.output_name, "minecraft:cooked_beef");
    }

    #[test]
    fn fuel_values() {
        let reg = SmeltingRegistry::new();
        assert_eq!(reg.fuel_burn_time("minecraft:coal"), Some(1600));
        assert_eq!(reg.fuel_burn_time("minecraft:stick"), Some(100));
        assert_eq!(reg.fuel_burn_time("minecraft:lava_bucket"), Some(20000));
        assert!(reg.fuel_burn_time("minecraft:stone").is_none());
    }

    #[test]
    fn cook_times() {
        assert_eq!(FurnaceType::Furnace.cook_time(), 200);
        assert_eq!(FurnaceType::BlastFurnace.cook_time(), 100);
        assert_eq!(FurnaceType::Smoker.cook_time(), 100);
    }
}
