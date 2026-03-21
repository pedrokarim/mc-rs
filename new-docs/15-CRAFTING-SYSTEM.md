# 15 - Crafting System

## PocketMine : Système de crafting

### Types de recettes

| Type | Description | Exemple |
|---|---|---|
| ShapedRecipe | Grille 2D avec pattern | Pioche, épée, armure |
| ShapelessRecipe | Ingrédients sans position | Teinture, livre+plume |
| FurnaceRecipe | Cuisson (input → output) | Minerai → lingot |
| PotionTypeRecipe | Ingrédient + potion → potion | Nether wart + water → awkward |
| PotionContainerChangeRecipe | Changement de conteneur | Splash → Lingering |

### ShapedRecipe

```
Pattern (3x3 max) :
  "AAA"    A = Iron Ingot
  " B "    B = Stick
  " B "

Résultat : Iron Pickaxe x1
```

**Matching :**
1. Trouver la zone non-vide dans la grille de craft
2. Comparer le pattern aux ingrédients
3. Mirror horizontal supporté (essayer les deux)

### ShapelessRecipe

```
Ingrédients (sans ordre) :
  - Red Dye x1
  - White Wool x1

Résultat : Red Wool x1
```

**Matching :**
1. Pour chaque ingrédient de la recette, trouver un item correspondant dans la grille
2. Chaque item de la grille doit correspondre à exactement un ingrédient
3. L'ordre n'a pas d'importance

### FurnaceRecipe

```
Input : Raw Iron
Fuel  : (any fuel)
Output : Iron Ingot
```

**Types de fourneau :**
- `FURNACE` : fourneau classique
- `BLAST_FURNACE` : haut-fourneau (2x plus rapide, minerais seulement)
- `SMOKER` : fumoir (2x plus rapide, nourriture seulement)

### RecipeIngredient (matching d'ingrédients)

| Type | Description |
|---|---|
| ExactRecipeIngredient | Item exact (type + NBT) |
| MetaWildcardRecipeIngredient | Type d'item, n'importe quelle variante |
| TagWildcardRecipeIngredient | N'importe quel item avec ce tag |

### CraftingDataPacket

Envoyé au client avec toutes les recettes :
```
shaped_recipes: Vec<ShapedRecipeData>
shapeless_recipes: Vec<ShapelessRecipeData>
furnace_recipes: Vec<FurnaceRecipeData>
potion_type_recipes: Vec<PotionTypeRecipeData>
potion_container_change_recipes: Vec<PotionContainerChangeRecipeData>
material_reducers: Vec<MaterialReducerData>
```

### CraftingGrid

```
PlayerCraftingGrid : 2x2 (inventaire)
CraftingTableGrid  : 3x3 (table de craft)

Le client envoie un ItemStackRequest avec CraftRecipeAction
Le serveur vérifie :
  1. La recette existe
  2. Les ingrédients sont dans la grille
  3. Le résultat est valide
Puis applique la transaction
```

### Fichiers PocketMine de référence

```
src/crafting/CraftingManager.php
src/crafting/ShapedRecipe.php
src/crafting/ShapelessRecipe.php
src/crafting/FurnaceRecipe.php
src/crafting/FurnaceRecipeManager.php
src/crafting/RecipeIngredient.php
src/crafting/ExactRecipeIngredient.php
src/crafting/CraftingGrid.php
src/crafting/PotionTypeRecipe.php
src/crafting/PotionContainerChangeRecipe.php
src/crafting/json/                       → données JSON
```

---

## Équivalent Rust

### Crate : `mc-rs-crafting`

```rust
/// Recette shaped (grille 2D)
pub struct ShapedRecipe {
    pub width: u8,        // 1-3
    pub height: u8,       // 1-3
    pub pattern: Vec<Vec<Option<RecipeIngredient>>>,  // [row][col]
    pub results: Vec<ItemStack>,
}

impl ShapedRecipe {
    pub fn matches(&self, grid: &CraftingGrid) -> bool {
        // Essayer normal et miroir horizontal
        self.matches_at(grid, false) || self.matches_at(grid, true)
    }

    fn matches_at(&self, grid: &CraftingGrid, mirrored: bool) -> bool {
        let (gw, gh) = grid.recipe_bounds();
        if gw != self.width || gh != self.height {
            return false;
        }
        for row in 0..self.height {
            for col in 0..self.width {
                let pattern_col = if mirrored { self.width - 1 - col } else { col };
                let ingredient = &self.pattern[row as usize][pattern_col as usize];
                let grid_item = grid.get(col, row);
                match ingredient {
                    Some(ing) => if !ing.matches(&grid_item) { return false; }
                    None => if !grid_item.is_empty() { return false; }
                }
            }
        }
        true
    }
}

/// Recette shapeless (sans ordre)
pub struct ShapelessRecipe {
    pub ingredients: Vec<RecipeIngredient>,
    pub results: Vec<ItemStack>,
}

impl ShapelessRecipe {
    pub fn matches(&self, grid: &CraftingGrid) -> bool {
        let items = grid.non_empty_items();
        if items.len() != self.ingredients.len() {
            return false;
        }
        let mut used = vec![false; self.ingredients.len()];
        for item in &items {
            let found = self.ingredients.iter().enumerate()
                .find(|(i, ing)| !used[*i] && ing.matches(item));
            match found {
                Some((i, _)) => used[i] = true,
                None => return false,
            }
        }
        used.iter().all(|&u| u)
    }
}

/// Recette de fourneau
pub struct FurnaceRecipe {
    pub input: RecipeIngredient,
    pub result: ItemStack,
}

/// Ingrédient de recette
pub enum RecipeIngredient {
    Exact(ItemStack),
    TypeWildcard(ItemTypeId),
    TagWildcard(String),
}

impl RecipeIngredient {
    pub fn matches(&self, item: &ItemStack) -> bool {
        match self {
            Self::Exact(expected) => item.type_id == expected.type_id && item.damage == expected.damage,
            Self::TypeWildcard(type_id) => item.type_id == *type_id,
            Self::TagWildcard(tag) => ItemRegistry::global().has_tag(item.type_id, tag),
        }
    }
}

/// Grille de craft
pub struct CraftingGrid {
    size: u8,  // 2 ou 3
    slots: Vec<ItemStack>,
}

impl CraftingGrid {
    pub fn get(&self, x: u8, y: u8) -> &ItemStack {
        &self.slots[(y as usize) * self.size as usize + x as usize]
    }

    pub fn recipe_bounds(&self) -> (u8, u8) {
        // Calculer la plus petite zone non-vide
        todo!()
    }

    pub fn non_empty_items(&self) -> Vec<&ItemStack> {
        self.slots.iter().filter(|s| !s.is_empty()).collect()
    }
}

/// Type de fourneau
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FurnaceType {
    Furnace,
    BlastFurnace,
    Smoker,
}

/// Registre de recettes
pub struct CraftingManager {
    shaped: Vec<ShapedRecipe>,
    shapeless: Vec<ShapelessRecipe>,
    furnace: HashMap<FurnaceType, Vec<FurnaceRecipe>>,
}

impl CraftingManager {
    pub fn match_recipe(&self, grid: &CraftingGrid) -> Option<&dyn CraftingRecipe> {
        // D'abord essayer shaped, puis shapeless
        for recipe in &self.shaped {
            if recipe.matches(grid) {
                return Some(recipe);
            }
        }
        for recipe in &self.shapeless {
            if recipe.matches(grid) {
                return Some(recipe);
            }
        }
        None
    }

    pub fn match_furnace(&self, furnace_type: FurnaceType, input: &ItemStack) -> Option<&FurnaceRecipe> {
        self.furnace.get(&furnace_type)?
            .iter()
            .find(|r| r.input.matches(input))
    }

    pub fn load_from_json(&mut self, path: &Path) -> Result<()> {
        // Charger les recettes depuis les fichiers JSON de PocketMine
        todo!()
    }
}
```
