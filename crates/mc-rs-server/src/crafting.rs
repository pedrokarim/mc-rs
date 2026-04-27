//! Crafting — port sélectif de `.reference/PocketMine-MP/src/crafting/*`.
//!
//! Couvre : recipes shaped (3x3 avec pattern), shapeless (liste d'ingrédients
//! quelconque ordre), furnace (smelting 1→1).
//!
//! `RECIPE_DB` static partagé : initialisé une fois au boot (main.rs) après
//! `recipes_vanilla::register_all`. Permet à `InventoryManager` d'accéder
//! aux 1601+ recipes sans avoir à passer la référence par paramètre.

use std::sync::OnceLock;

use mc_rs_proto::packets::player::ItemStack;

pub static RECIPE_DB: OnceLock<CraftingManager> = OnceLock::new();

/// Ingrédient de recette. Peut matcher exactement ou avec meta wildcard.
#[derive(Debug, Clone)]
pub enum RecipeIngredient {
    /// Match exact sur (id, meta). PMMP `ExactRecipeIngredient`.
    Exact { item_id: i32, meta: u32, count: u16 },
    /// Match sur id peu importe meta. PMMP `MetaWildcardRecipeIngredient`.
    AnyMeta { item_id: i32, count: u16 },
    /// Slot vide (pour shaped recipes).
    Empty,
}

impl RecipeIngredient {
    pub fn matches(&self, stack: &ItemStack) -> bool {
        match self {
            Self::Exact {
                item_id,
                meta,
                count,
            } => stack.id == *item_id && stack.meta == *meta && stack.count >= *count,
            Self::AnyMeta { item_id, count } => stack.id == *item_id && stack.count >= *count,
            Self::Empty => stack.is_air(),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// Recette shaped 3x3 ou plus petite. PMMP `ShapedRecipe.php`.
/// `input` est un Vec<Vec<RecipeIngredient>> de dimensions `height × width`
/// avec width ≤ 3 et height ≤ 3.
#[derive(Debug, Clone)]
pub struct ShapedRecipe {
    pub width: usize,
    pub height: usize,
    pub input: Vec<RecipeIngredient>,
    pub output: Vec<ItemStack>,
}

impl ShapedRecipe {
    pub fn new(
        width: usize,
        height: usize,
        input: Vec<RecipeIngredient>,
        output: Vec<ItemStack>,
    ) -> Self {
        assert_eq!(input.len(), width * height);
        assert!(width <= 3 && height <= 3, "max crafting grid is 3x3");
        Self {
            width,
            height,
            input,
            output,
        }
    }

    /// Essaye de match la grille avec cette recette.
    /// La grille est un slice de (crafting_size × crafting_size) items.
    /// Retourne true si match (y compris via offset / mirror).
    pub fn matches(&self, grid: &[ItemStack], grid_size: usize) -> bool {
        if self.width > grid_size || self.height > grid_size {
            return false;
        }
        // Essayer chaque position possible dans la grille, pour direct + mirror.
        let max_x = grid_size - self.width;
        let max_y = grid_size - self.height;
        for dy in 0..=max_y {
            for dx in 0..=max_x {
                if self.matches_at(grid, grid_size, dx, dy, false)
                    || self.matches_at(grid, grid_size, dx, dy, true)
                {
                    return true;
                }
            }
        }
        false
    }

    fn matches_at(
        &self,
        grid: &[ItemStack],
        grid_size: usize,
        ox: usize,
        oy: usize,
        mirror: bool,
    ) -> bool {
        // Vérif que les slots HORS de la zone recipe sont air.
        for gy in 0..grid_size {
            for gx in 0..grid_size {
                let in_recipe =
                    gx >= ox && gx < ox + self.width && gy >= oy && gy < oy + self.height;
                let cell = &grid[gy * grid_size + gx];
                if !in_recipe {
                    if !cell.is_air() {
                        return false;
                    }
                    continue;
                }
                let rx = if mirror {
                    self.width - 1 - (gx - ox)
                } else {
                    gx - ox
                };
                let ry = gy - oy;
                let ing = &self.input[ry * self.width + rx];
                if !ing.matches(cell) {
                    return false;
                }
            }
        }
        true
    }
}

/// Recette shapeless — ordre n'importe quel dans la grille.
/// PMMP `ShapelessRecipe.php`.
#[derive(Debug, Clone)]
pub struct ShapelessRecipe {
    pub ingredients: Vec<RecipeIngredient>,
    pub output: Vec<ItemStack>,
}

impl ShapelessRecipe {
    pub fn new(ingredients: Vec<RecipeIngredient>, output: Vec<ItemStack>) -> Self {
        Self {
            ingredients,
            output,
        }
    }

    pub fn matches(&self, grid: &[ItemStack]) -> bool {
        let non_air_grid: Vec<&ItemStack> = grid.iter().filter(|s| !s.is_air()).collect();
        if non_air_grid.len() != self.ingredients.len() {
            return false;
        }
        // Bipartite matching greedy : chaque ingredient doit matcher un slot unique.
        let mut used = vec![false; non_air_grid.len()];
        for ing in &self.ingredients {
            let mut found = false;
            for (i, stack) in non_air_grid.iter().enumerate() {
                if !used[i] && ing.matches(stack) {
                    used[i] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    }
}

/// Recette furnace = 1 ingredient → 1 output + temps de cuisson.
#[derive(Debug, Clone)]
pub struct FurnaceRecipe {
    pub input: RecipeIngredient,
    pub output: ItemStack,
    pub cook_time_ticks: u32,
    pub xp: f32,
}

impl FurnaceRecipe {
    pub fn matches(&self, stack: &ItemStack) -> bool {
        self.input.matches(stack)
    }
}

/// Manager global qui garde toutes les recettes. Port PMMP `CraftingManager`.
#[derive(Debug, Default, Clone)]
pub struct CraftingManager {
    pub shaped: Vec<ShapedRecipe>,
    pub shapeless: Vec<ShapelessRecipe>,
    pub furnace: Vec<FurnaceRecipe>,
    pub blast_furnace: Vec<FurnaceRecipe>,
    pub smoker: Vec<FurnaceRecipe>,
}

impl CraftingManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_shaped(&mut self, r: ShapedRecipe) {
        self.shaped.push(r);
    }

    pub fn register_shapeless(&mut self, r: ShapelessRecipe) {
        self.shapeless.push(r);
    }

    pub fn register_furnace(&mut self, r: FurnaceRecipe) {
        self.furnace.push(r);
    }

    /// Trouve la première recette qui match la grille (shaped ou shapeless).
    pub fn match_crafting(&self, grid: &[ItemStack], grid_size: usize) -> Option<&[ItemStack]> {
        for r in &self.shaped {
            if r.matches(grid, grid_size) {
                return Some(&r.output);
            }
        }
        for r in &self.shapeless {
            if r.matches(grid) {
                return Some(&r.output);
            }
        }
        None
    }

    pub fn match_furnace(&self, stack: &ItemStack) -> Option<&FurnaceRecipe> {
        self.furnace.iter().find(|r| r.matches(stack))
    }

    pub fn match_blast(&self, stack: &ItemStack) -> Option<&FurnaceRecipe> {
        self.blast_furnace.iter().find(|r| r.matches(stack))
    }

    pub fn match_smoker(&self, stack: &ItemStack) -> Option<&FurnaceRecipe> {
        self.smoker.iter().find(|r| r.matches(stack))
    }

    /// Précharge des recettes vanilla basiques (planks, sticks, torches, etc.).
    /// Port minimal de `crafting/json/*`.
    pub fn register_vanilla_basics(&mut self) {
        use crate::item_registry::required_item_id;
        // Planks (shapeless): 1 log → 4 planks
        for (log, planks) in &[
            ("minecraft:oak_log", "minecraft:oak_planks"),
            ("minecraft:birch_log", "minecraft:birch_planks"),
            ("minecraft:spruce_log", "minecraft:spruce_planks"),
            ("minecraft:jungle_log", "minecraft:jungle_planks"),
            ("minecraft:acacia_log", "minecraft:acacia_planks"),
            ("minecraft:dark_oak_log", "minecraft:dark_oak_planks"),
        ] {
            self.register_shapeless(ShapelessRecipe::new(
                vec![RecipeIngredient::AnyMeta {
                    item_id: required_item_id(log),
                    count: 1,
                }],
                vec![ItemStack::new(required_item_id(planks), 4, 0)],
            ));
        }
        // Sticks : 2 planks vertically → 4 sticks
        let oak_planks = RecipeIngredient::AnyMeta {
            item_id: required_item_id("minecraft:oak_planks"),
            count: 1,
        };
        self.register_shaped(ShapedRecipe::new(
            1,
            2,
            vec![oak_planks.clone(), oak_planks.clone()],
            vec![ItemStack::new(required_item_id("minecraft:stick"), 4, 0)],
        ));
        // Crafting table : 2x2 planks
        let p = RecipeIngredient::AnyMeta {
            item_id: required_item_id("minecraft:oak_planks"),
            count: 1,
        };
        self.register_shaped(ShapedRecipe::new(
            2,
            2,
            vec![p.clone(), p.clone(), p.clone(), p.clone()],
            vec![ItemStack::new(
                required_item_id("minecraft:crafting_table"),
                1,
                0,
            )],
        ));
        // Furnace : 8 cobblestone
        let c = RecipeIngredient::AnyMeta {
            item_id: required_item_id("minecraft:cobblestone"),
            count: 1,
        };
        self.register_shaped(ShapedRecipe::new(
            3,
            3,
            vec![
                c.clone(),
                c.clone(),
                c.clone(),
                c.clone(),
                RecipeIngredient::Empty,
                c.clone(),
                c.clone(),
                c.clone(),
                c.clone(),
            ],
            vec![ItemStack::new(required_item_id("minecraft:furnace"), 1, 0)],
        ));
        // Chest : 8 planks
        let pp = RecipeIngredient::AnyMeta {
            item_id: required_item_id("minecraft:oak_planks"),
            count: 1,
        };
        self.register_shaped(ShapedRecipe::new(
            3,
            3,
            vec![
                pp.clone(),
                pp.clone(),
                pp.clone(),
                pp.clone(),
                RecipeIngredient::Empty,
                pp.clone(),
                pp.clone(),
                pp.clone(),
                pp.clone(),
            ],
            vec![ItemStack::new(required_item_id("minecraft:chest"), 1, 0)],
        ));

        // Furnace smelting basics.
        self.register_furnace(FurnaceRecipe {
            input: RecipeIngredient::AnyMeta {
                item_id: required_item_id("minecraft:iron_ore"),
                count: 1,
            },
            output: ItemStack::new(required_item_id("minecraft:iron_ingot"), 1, 0),
            cook_time_ticks: 200,
            xp: 0.7,
        });
        self.register_furnace(FurnaceRecipe {
            input: RecipeIngredient::AnyMeta {
                item_id: required_item_id("minecraft:gold_ore"),
                count: 1,
            },
            output: ItemStack::new(required_item_id("minecraft:gold_ingot"), 1, 0),
            cook_time_ticks: 200,
            xp: 1.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air() -> ItemStack {
        ItemStack::AIR
    }

    fn stack(id: i32, count: u16) -> ItemStack {
        ItemStack::new(id, count, 0)
    }

    #[test]
    fn shapeless_matches_regardless_of_position() {
        let r = ShapelessRecipe::new(
            vec![
                RecipeIngredient::AnyMeta {
                    item_id: 1,
                    count: 1,
                },
                RecipeIngredient::AnyMeta {
                    item_id: 2,
                    count: 1,
                },
            ],
            vec![stack(3, 1)],
        );
        // 2x2 grid with items at (0,0) and (1,1).
        let grid = vec![stack(1, 1), air(), air(), stack(2, 1)];
        assert!(r.matches(&grid));
        // Swap: grid with items at (0,0) and (0,1)
        let grid2 = vec![stack(2, 1), air(), stack(1, 1), air()];
        assert!(r.matches(&grid2));
    }

    #[test]
    fn shaped_matches_with_offset() {
        // 2-high 1-wide pattern (sticks: 2 planks on top of each other).
        let r = ShapedRecipe::new(
            1,
            2,
            vec![
                RecipeIngredient::AnyMeta {
                    item_id: 1,
                    count: 1,
                },
                RecipeIngredient::AnyMeta {
                    item_id: 1,
                    count: 1,
                },
            ],
            vec![stack(5, 4)],
        );
        // 2x2 grid with items at (0,0) and (0,1) col 0 rows 0-1.
        let grid = vec![stack(1, 1), air(), stack(1, 1), air()];
        assert!(r.matches(&grid, 2));
    }

    #[test]
    fn shaped_rejects_non_matching() {
        let r = ShapedRecipe::new(
            1,
            2,
            vec![
                RecipeIngredient::AnyMeta {
                    item_id: 1,
                    count: 1,
                },
                RecipeIngredient::AnyMeta {
                    item_id: 1,
                    count: 1,
                },
            ],
            vec![stack(5, 4)],
        );
        let grid = vec![stack(2, 1), air(), stack(1, 1), air()];
        assert!(!r.matches(&grid, 2));
    }

    #[test]
    fn furnace_matches_ingredient() {
        let r = FurnaceRecipe {
            input: RecipeIngredient::AnyMeta {
                item_id: 10,
                count: 1,
            },
            output: stack(20, 1),
            cook_time_ticks: 200,
            xp: 0.7,
        };
        assert!(r.matches(&stack(10, 1)));
        assert!(!r.matches(&stack(11, 1)));
    }
}
