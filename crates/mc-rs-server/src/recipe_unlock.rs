//! Recipe unlock tracking per player.

use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct UnlockedRecipes {
    pub recipes: HashSet<String>,
}

impl UnlockedRecipes {
    pub fn unlock(&mut self, recipe_id: impl Into<String>) -> bool {
        self.recipes.insert(recipe_id.into())
    }

    pub fn has(&self, recipe_id: &str) -> bool {
        self.recipes.contains(recipe_id)
    }

    pub fn lock(&mut self, recipe_id: &str) -> bool {
        self.recipes.remove(recipe_id)
    }

    /// Starter recipes unlocked by default.
    pub fn apply_starter(&mut self) {
        for r in &[
            "minecraft:crafting_table",
            "minecraft:furnace",
            "minecraft:oak_planks",
            "minecraft:stick",
        ] {
            self.unlock(r.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlock_new_recipe() {
        let mut u = UnlockedRecipes::default();
        assert!(u.unlock("minecraft:iron_pickaxe"));
        assert!(u.has("minecraft:iron_pickaxe"));
    }

    #[test]
    fn starter_has_crafting_table() {
        let mut u = UnlockedRecipes::default();
        u.apply_starter();
        assert!(u.has("minecraft:crafting_table"));
    }
}
