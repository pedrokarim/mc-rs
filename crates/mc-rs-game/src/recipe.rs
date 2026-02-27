//! Recipe registry for crafting.
//!
//! Stores shaped and shapeless recipes and provides lookup by network ID.

/// Input ingredient for a recipe.
#[derive(Debug, Clone)]
pub struct RecipeInput {
    /// Item name, e.g. "minecraft:oak_planks".
    pub item_name: String,
    /// Required count per slot.
    pub count: u8,
    /// Metadata filter. -1 = any variant, 0+ = specific.
    pub metadata: i16,
}

/// Output result of a recipe.
#[derive(Debug, Clone)]
pub struct RecipeOutput {
    /// Item name.
    pub item_name: String,
    /// Output count.
    pub count: u8,
    /// Output metadata.
    pub metadata: u16,
}

/// A shaped crafting recipe (position-dependent).
#[derive(Debug, Clone)]
pub struct ShapedRecipe {
    /// Unique recipe identifier (UUID-like string).
    pub id: String,
    /// Network ID for protocol (sequential).
    pub network_id: u32,
    /// Grid width (1-3).
    pub width: u8,
    /// Grid height (1-3).
    pub height: u8,
    /// Input grid (width × height). Empty string = air/empty slot.
    pub input: Vec<RecipeInput>,
    /// Output item(s).
    pub output: Vec<RecipeOutput>,
    /// Block tag, e.g. "crafting_table".
    pub tag: String,
}

/// A shapeless crafting recipe (order-independent).
#[derive(Debug, Clone)]
pub struct ShapelessRecipe {
    /// Unique recipe identifier.
    pub id: String,
    /// Network ID for protocol.
    pub network_id: u32,
    /// Input ingredients (any order).
    pub inputs: Vec<RecipeInput>,
    /// Output item(s).
    pub output: Vec<RecipeOutput>,
    /// Block tag.
    pub tag: String,
}

/// Reference to either a shaped or shapeless recipe.
#[derive(Debug, Clone)]
pub enum RecipeRef<'a> {
    Shaped(&'a ShapedRecipe),
    Shapeless(&'a ShapelessRecipe),
}

impl RecipeRef<'_> {
    pub fn output(&self) -> &[RecipeOutput] {
        match self {
            RecipeRef::Shaped(r) => &r.output,
            RecipeRef::Shapeless(r) => &r.output,
        }
    }
}

/// Registry of all crafting recipes.
pub struct RecipeRegistry {
    shaped: Vec<ShapedRecipe>,
    shapeless: Vec<ShapelessRecipe>,
}

impl Default for RecipeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeRegistry {
    /// Build the registry with essential recipes.
    pub fn new() -> Self {
        let mut next_id: u32 = 1;
        let mut shaped = Vec::new();
        let mut shapeless = Vec::new();

        // Helper to create shaped recipe
        macro_rules! shaped {
            ($id:expr, $w:expr, $h:expr, $input:expr, $out_name:expr, $out_count:expr) => {{
                let nid = next_id;
                next_id += 1;
                shaped.push(ShapedRecipe {
                    id: format!("mc-rs:shaped_{}", nid),
                    network_id: nid,
                    width: $w,
                    height: $h,
                    input: $input,
                    output: vec![RecipeOutput {
                        item_name: $out_name.to_string(),
                        count: $out_count,
                        metadata: 0,
                    }],
                    tag: "crafting_table".to_string(),
                });
            }};
        }

        macro_rules! shapeless {
            ($id:expr, $inputs:expr, $out_name:expr, $out_count:expr) => {{
                let nid = next_id;
                next_id += 1;
                shapeless.push(ShapelessRecipe {
                    id: format!("mc-rs:shapeless_{}", nid),
                    network_id: nid,
                    inputs: $inputs,
                    output: vec![RecipeOutput {
                        item_name: $out_name.to_string(),
                        count: $out_count,
                        metadata: 0,
                    }],
                    tag: "crafting_table".to_string(),
                });
            }};
        }

        let air = || inp("", 0);

        // ---- Planks from logs (shapeless, 1×1) ----
        for (log, plank) in &[
            ("minecraft:oak_log", "minecraft:oak_planks"),
            ("minecraft:spruce_log", "minecraft:spruce_planks"),
            ("minecraft:birch_log", "minecraft:birch_planks"),
            ("minecraft:jungle_log", "minecraft:jungle_planks"),
            ("minecraft:acacia_log", "minecraft:acacia_planks"),
            ("minecraft:dark_oak_log", "minecraft:dark_oak_planks"),
        ] {
            shapeless!("planks", vec![inp(log, -1)], *plank, 4);
        }

        // ---- Sticks (shaped 1×2) ----
        shaped!(
            "sticks",
            1,
            2,
            vec![
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1)
            ],
            "minecraft:stick",
            4
        );

        // ---- Crafting Table (shaped 2×2) ----
        shaped!(
            "crafting_table",
            2,
            2,
            vec![
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
            ],
            "minecraft:crafting_table",
            1
        );

        // ---- Chest (shaped 3×3) ----
        shaped!(
            "chest",
            3,
            3,
            vec![
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                air(),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
                inp("minecraft:oak_planks", -1),
            ],
            "minecraft:chest",
            1
        );

        // ---- Furnace (shaped 3×3) ----
        shaped!(
            "furnace",
            3,
            3,
            vec![
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
                air(),
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
                inp("minecraft:cobblestone", 0),
            ],
            "minecraft:furnace",
            1
        );

        // ---- Torches (shaped 1×2) ----
        shaped!(
            "torch",
            1,
            2,
            vec![inp("minecraft:coal", -1), inp("minecraft:stick", 0)],
            "minecraft:torch",
            4
        );

        // ---- Tools ----
        // Wooden tools
        let s = || inp("minecraft:stick", 0);
        let p = |name: &str, meta: i16| inp(name, meta);

        // Wooden Pickaxe
        shaped!(
            "wooden_pickaxe",
            3,
            3,
            vec![
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:wooden_pickaxe",
            1
        );

        // Wooden Axe
        shaped!(
            "wooden_axe",
            3,
            3,
            vec![
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                air(),
                p("minecraft:oak_planks", -1),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:wooden_axe",
            1
        );

        // Wooden Shovel
        shaped!(
            "wooden_shovel",
            1,
            3,
            vec![p("minecraft:oak_planks", -1), s(), s(),],
            "minecraft:wooden_shovel",
            1
        );

        // Wooden Sword
        shaped!(
            "wooden_sword",
            1,
            3,
            vec![
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                s(),
            ],
            "minecraft:wooden_sword",
            1
        );

        // Wooden Hoe
        shaped!(
            "wooden_hoe",
            3,
            3,
            vec![
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                air(),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:wooden_hoe",
            1
        );

        // Stone tools
        shaped!(
            "stone_pickaxe",
            3,
            3,
            vec![
                p("minecraft:cobblestone", 0),
                p("minecraft:cobblestone", 0),
                p("minecraft:cobblestone", 0),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:stone_pickaxe",
            1
        );

        shaped!(
            "stone_axe",
            3,
            3,
            vec![
                p("minecraft:cobblestone", 0),
                p("minecraft:cobblestone", 0),
                air(),
                p("minecraft:cobblestone", 0),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:stone_axe",
            1
        );

        shaped!(
            "stone_shovel",
            1,
            3,
            vec![p("minecraft:cobblestone", 0), s(), s(),],
            "minecraft:stone_shovel",
            1
        );

        shaped!(
            "stone_sword",
            1,
            3,
            vec![
                p("minecraft:cobblestone", 0),
                p("minecraft:cobblestone", 0),
                s(),
            ],
            "minecraft:stone_sword",
            1
        );

        shaped!(
            "stone_hoe",
            3,
            3,
            vec![
                p("minecraft:cobblestone", 0),
                p("minecraft:cobblestone", 0),
                air(),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:stone_hoe",
            1
        );

        // Iron tools
        shaped!(
            "iron_pickaxe",
            3,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:iron_pickaxe",
            1
        );

        shaped!(
            "iron_axe",
            3,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:iron_axe",
            1
        );

        shaped!(
            "iron_shovel",
            1,
            3,
            vec![p("minecraft:iron_ingot", 0), s(), s(),],
            "minecraft:iron_shovel",
            1
        );

        shaped!(
            "iron_sword",
            1,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                s(),
            ],
            "minecraft:iron_sword",
            1
        );

        shaped!(
            "iron_hoe",
            3,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:iron_hoe",
            1
        );

        // Diamond tools
        shaped!(
            "diamond_pickaxe",
            3,
            3,
            vec![
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:diamond_pickaxe",
            1
        );

        shaped!(
            "diamond_axe",
            3,
            3,
            vec![
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:diamond_axe",
            1
        );

        shaped!(
            "diamond_shovel",
            1,
            3,
            vec![p("minecraft:diamond", 0), s(), s(),],
            "minecraft:diamond_shovel",
            1
        );

        shaped!(
            "diamond_sword",
            1,
            3,
            vec![p("minecraft:diamond", 0), p("minecraft:diamond", 0), s(),],
            "minecraft:diamond_sword",
            1
        );

        shaped!(
            "diamond_hoe",
            3,
            3,
            vec![
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                air(),
                s(),
                air(),
                air(),
                s(),
                air(),
            ],
            "minecraft:diamond_hoe",
            1
        );

        // ---- Armor ----
        // Iron armor
        shaped!(
            "iron_helmet",
            3,
            2,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
            ],
            "minecraft:iron_helmet",
            1
        );

        shaped!(
            "iron_chestplate",
            3,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
            ],
            "minecraft:iron_chestplate",
            1
        );

        shaped!(
            "iron_leggings",
            3,
            3,
            vec![
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
            ],
            "minecraft:iron_leggings",
            1
        );

        shaped!(
            "iron_boots",
            3,
            2,
            vec![
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
            ],
            "minecraft:iron_boots",
            1
        );

        // Diamond armor
        shaped!(
            "diamond_helmet",
            3,
            2,
            vec![
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
            ],
            "minecraft:diamond_helmet",
            1
        );

        shaped!(
            "diamond_chestplate",
            3,
            3,
            vec![
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
            ],
            "minecraft:diamond_chestplate",
            1
        );

        shaped!(
            "diamond_leggings",
            3,
            3,
            vec![
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
            ],
            "minecraft:diamond_leggings",
            1
        );

        shaped!(
            "diamond_boots",
            3,
            2,
            vec![
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
                p("minecraft:diamond", 0),
                air(),
                p("minecraft:diamond", 0),
            ],
            "minecraft:diamond_boots",
            1
        );

        // ---- Misc ----
        // Bucket
        shaped!(
            "bucket",
            3,
            2,
            vec![
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                air(),
                p("minecraft:iron_ingot", 0),
                air(),
            ],
            "minecraft:bucket",
            1
        );

        // Bow
        shaped!(
            "bow",
            3,
            3,
            vec![
                air(),
                s(),
                p("minecraft:string", 0),
                s(),
                air(),
                p("minecraft:string", 0),
                air(),
                s(),
                p("minecraft:string", 0),
            ],
            "minecraft:bow",
            1
        );

        // Arrow
        shaped!(
            "arrow",
            1,
            3,
            vec![p("minecraft:flint", 0), s(), p("minecraft:feather", 0),],
            "minecraft:arrow",
            4
        );

        // Ladder
        shaped!(
            "ladder",
            3,
            3,
            vec![s(), air(), s(), s(), s(), s(), s(), air(), s(),],
            "minecraft:ladder",
            3
        );

        // Fence (oak)
        shaped!(
            "fence",
            3,
            2,
            vec![
                p("minecraft:oak_planks", -1),
                s(),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                s(),
                p("minecraft:oak_planks", -1),
            ],
            "minecraft:oak_fence",
            3
        );

        // Door (oak)
        shaped!(
            "door",
            2,
            3,
            vec![
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
            ],
            "minecraft:wooden_door",
            3
        );

        // Bed (white wool)
        shaped!(
            "bed",
            3,
            2,
            vec![
                p("minecraft:white_wool", 0),
                p("minecraft:white_wool", 0),
                p("minecraft:white_wool", 0),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
                p("minecraft:oak_planks", -1),
            ],
            "minecraft:bed",
            1
        );

        let _ = next_id;
        Self { shaped, shapeless }
    }

    /// Look up any recipe by network ID.
    pub fn get_by_network_id(&self, id: u32) -> Option<RecipeRef<'_>> {
        for r in &self.shaped {
            if r.network_id == id {
                return Some(RecipeRef::Shaped(r));
            }
        }
        for r in &self.shapeless {
            if r.network_id == id {
                return Some(RecipeRef::Shapeless(r));
            }
        }
        None
    }

    /// Get all shaped recipes.
    pub fn shaped_recipes(&self) -> &[ShapedRecipe] {
        &self.shaped
    }

    /// Get all shapeless recipes.
    pub fn shapeless_recipes(&self) -> &[ShapelessRecipe] {
        &self.shapeless
    }

    /// Total number of recipes.
    pub fn len(&self) -> usize {
        self.shaped.len() + self.shapeless.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.shaped.is_empty() && self.shapeless.is_empty()
    }

    /// Next available network ID.
    fn next_network_id(&self) -> u32 {
        let max_shaped = self.shaped.iter().map(|r| r.network_id).max().unwrap_or(0);
        let max_shapeless = self
            .shapeless
            .iter()
            .map(|r| r.network_id)
            .max()
            .unwrap_or(0);
        max_shaped.max(max_shapeless) + 1
    }

    /// Register a custom shaped recipe. Returns the assigned network_id.
    pub fn register_shaped(&mut self, mut recipe: ShapedRecipe) -> u32 {
        let id = self.next_network_id();
        recipe.network_id = id;
        self.shaped.push(recipe);
        id
    }

    /// Register a custom shapeless recipe. Returns the assigned network_id.
    pub fn register_shapeless(&mut self, mut recipe: ShapelessRecipe) -> u32 {
        let id = self.next_network_id();
        recipe.network_id = id;
        self.shapeless.push(recipe);
        id
    }
}

/// Helper to create a recipe input.
fn inp(name: &str, metadata: i16) -> RecipeInput {
    RecipeInput {
        item_name: name.to_string(),
        count: 1,
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_recipes() {
        let reg = RecipeRegistry::new();
        assert!(reg.len() > 30);
        assert!(!reg.shaped_recipes().is_empty());
        assert!(!reg.shapeless_recipes().is_empty());
    }

    #[test]
    fn network_id_lookup() {
        let reg = RecipeRegistry::new();
        // First recipe should have network_id = 1
        let recipe = reg.get_by_network_id(1);
        assert!(recipe.is_some());
        // Non-existent ID
        assert!(reg.get_by_network_id(9999).is_none());
    }

    #[test]
    fn shaped_recipe_structure() {
        let reg = RecipeRegistry::new();
        // Find crafting table recipe
        let ct = reg.shaped_recipes().iter().find(|r| {
            r.output
                .iter()
                .any(|o| o.item_name == "minecraft:crafting_table")
        });
        assert!(ct.is_some());
        let ct = ct.unwrap();
        assert_eq!(ct.width, 2);
        assert_eq!(ct.height, 2);
        assert_eq!(ct.input.len(), 4);
    }

    #[test]
    fn shapeless_recipe_structure() {
        let reg = RecipeRegistry::new();
        // Planks recipes should be shapeless
        let planks = reg.shapeless_recipes().iter().find(|r| {
            r.output
                .iter()
                .any(|o| o.item_name == "minecraft:oak_planks")
        });
        assert!(planks.is_some());
        let planks = planks.unwrap();
        assert_eq!(planks.inputs.len(), 1);
        assert_eq!(planks.output[0].count, 4);
    }

    #[test]
    fn all_network_ids_unique() {
        let reg = RecipeRegistry::new();
        let mut ids: Vec<u32> = reg.shaped_recipes().iter().map(|r| r.network_id).collect();
        ids.extend(reg.shapeless_recipes().iter().map(|r| r.network_id));
        let len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len, "Duplicate network IDs found");
    }

    #[test]
    fn register_custom_recipe() {
        let mut reg = RecipeRegistry::new();
        let old_len = reg.len();
        let id = reg.register_shapeless(ShapelessRecipe {
            id: "custom:rubies".to_string(),
            network_id: 0,
            inputs: vec![RecipeInput {
                item_name: "custom:ruby_block".to_string(),
                count: 1,
                metadata: -1,
            }],
            output: vec![RecipeOutput {
                item_name: "custom:ruby".to_string(),
                count: 9,
                metadata: 0,
            }],
            tag: "crafting_table".to_string(),
        });
        assert_eq!(reg.len(), old_len + 1);
        assert!(reg.get_by_network_id(id).is_some());
    }

    #[test]
    fn tool_recipes_exist() {
        let reg = RecipeRegistry::new();
        for tool in &[
            "minecraft:wooden_pickaxe",
            "minecraft:stone_sword",
            "minecraft:iron_axe",
            "minecraft:diamond_shovel",
        ] {
            let found = reg
                .shaped_recipes()
                .iter()
                .any(|r| r.output.iter().any(|o| o.item_name == *tool));
            assert!(found, "Missing recipe for {tool}");
        }
    }
}
