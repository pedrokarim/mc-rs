//! Large subset of crafting recipes (shaped + shapeless).

#[derive(Debug, Clone)]
pub struct ShapedRecipe {
    pub width: u8,
    pub height: u8,
    pub pattern: Vec<Option<&'static str>>,
    pub output: (&'static str, u32),
}

#[derive(Debug, Clone)]
pub struct ShapelessRecipe {
    pub ingredients: Vec<&'static str>,
    pub output: (&'static str, u32),
}

/// Some basic recipes.
pub fn basic_shaped_recipes() -> Vec<ShapedRecipe> {
    vec![
        // Crafting table: 4 planks
        ShapedRecipe {
            width: 2, height: 2,
            pattern: vec![
                Some("minecraft:oak_planks"), Some("minecraft:oak_planks"),
                Some("minecraft:oak_planks"), Some("minecraft:oak_planks"),
            ],
            output: ("minecraft:crafting_table", 1),
        },
        // Furnace: 8 cobblestones
        ShapedRecipe {
            width: 3, height: 3,
            pattern: vec![
                Some("minecraft:cobblestone"), Some("minecraft:cobblestone"), Some("minecraft:cobblestone"),
                Some("minecraft:cobblestone"), None, Some("minecraft:cobblestone"),
                Some("minecraft:cobblestone"), Some("minecraft:cobblestone"), Some("minecraft:cobblestone"),
            ],
            output: ("minecraft:furnace", 1),
        },
        // Stick: 2 planks vertical
        ShapedRecipe {
            width: 1, height: 2,
            pattern: vec![
                Some("minecraft:oak_planks"),
                Some("minecraft:oak_planks"),
            ],
            output: ("minecraft:stick", 4),
        },
        // Iron pickaxe
        ShapedRecipe {
            width: 3, height: 3,
            pattern: vec![
                Some("minecraft:iron_ingot"), Some("minecraft:iron_ingot"), Some("minecraft:iron_ingot"),
                None, Some("minecraft:stick"), None,
                None, Some("minecraft:stick"), None,
            ],
            output: ("minecraft:iron_pickaxe", 1),
        },
        // Iron sword
        ShapedRecipe {
            width: 1, height: 3,
            pattern: vec![
                Some("minecraft:iron_ingot"),
                Some("minecraft:iron_ingot"),
                Some("minecraft:stick"),
            ],
            output: ("minecraft:iron_sword", 1),
        },
    ]
}

pub fn basic_shapeless_recipes() -> Vec<ShapelessRecipe> {
    vec![
        ShapelessRecipe {
            ingredients: vec!["minecraft:oak_log"],
            output: ("minecraft:oak_planks", 4),
        },
        ShapelessRecipe {
            ingredients: vec!["minecraft:wheat", "minecraft:wheat", "minecraft:wheat"],
            output: ("minecraft:bread", 1),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_crafting_table_recipe() {
        assert!(basic_shaped_recipes().iter().any(|r| r.output.0 == "minecraft:crafting_table"));
    }
}
