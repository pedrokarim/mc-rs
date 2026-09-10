//! Crafting — port sélectif de `.reference/PocketMine-MP/src/crafting/*`.
//!
//! Couvre : recipes shaped (3x3 avec pattern), shapeless (liste d'ingrédients
//! quelconque ordre), furnace (smelting 1→1).
//!
//! `RECIPE_DB` static partagé : initialisé une fois au boot (main.rs) après
//! `recipes_vanilla::register_all`. Permet à `InventoryManager` d'accéder
//! aux 1601+ recipes sans avoir à passer la référence par paramètre.

use std::sync::OnceLock;

use mc_rs_proto::io::ProtoWriter;
use mc_rs_proto::packets::player::ItemStack;
use mc_rs_proto::packets::world::{item_user_data, write_item_instance};

use crate::item_registry;

pub static RECIPE_DB: OnceLock<CraftingManager> = OnceLock::new();

/// Payload encodé du paquet `CraftingData` (S→C), construit une seule fois au
/// boot après `recipes_vanilla::register_all`. Envoyé tel quel à chaque
/// PreSpawn (cf. `connection/spawn.rs`).
pub static CRAFTING_DATA_PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();

// ── Constantes wire CraftingDataPacket 2169 ──

/// Meta joker d'un ingrédient (PMMP `TypeConverter::RECIPE_INPUT_WILDCARD_META`).
/// gophertunnel et Cloudburst écrivent aussi 32767 comme aux des descripteurs
/// vide et `item_tag`.
const RECIPE_INPUT_WILDCARD_META: i32 = 0x7fff;

/// Variante du descripteur d'ingrédient : 0 = invalide (slot vide), 1 = typé
/// par nom (gophertunnel `writer.go` `ItemDescriptorCount`, Cloudburst
/// `BedrockCodecHelper_v2168::writeIngredient` : `min(type, 1)`).
const DESCRIPTOR_VARIANT_INVALID: u32 = 0;
const DESCRIPTOR_VARIANT_TYPED: u32 = 1;

/// Noms sérialisés des types de descripteur (Cloudburst `ItemDescriptorType`,
/// gophertunnel `itemDescriptorType`).
const DESCRIPTOR_NAME_ITEM: &str = "name";
const DESCRIPTOR_NAME_TAG: &str = "item_tag";

/// `RecipeUnlockingRequirement` : contexte AlwaysUnlocked (gophertunnel
/// `RecipeUnlockContextAlwaysUnlocked`).
const UNLOCK_CONTEXT_ALWAYS_UNLOCKED: i32 = 1;

/// Priorité de recette (PMMP `CraftingDataCache` hardcode 50 partout).
const RECIPE_PRIORITY: i32 = 50;

/// Premier `recipeNetId` : le client rejette l'id 0 depuis 1.21.100 (PMMP
/// `CraftingDataCache::RECIPE_ID_OFFSET`).
const RECIPE_NET_ID_OFFSET: u32 = 1;

/// Ingrédient de recette. Peut matcher exactement, avec meta wildcard, ou via
/// un tag d'items Bedrock (`minecraft:planks`, `minecraft:logs`, …).
#[derive(Debug, Clone)]
pub enum RecipeIngredient {
    /// Match exact sur (id, meta). PMMP `ExactRecipeIngredient`.
    Exact { item_id: i32, meta: u32, count: u16 },
    /// Match sur id peu importe meta. PMMP `MetaWildcardRecipeIngredient`.
    AnyMeta { item_id: i32, count: u16 },
    /// Match si l'item appartient au tag. PMMP `TagWildcardRecipeIngredient`.
    /// `tag` est le nom Bedrock officiel (envoyé tel quel au client via
    /// `TagItemDescriptor`), `members` la liste des network ids résolus
    /// (utilisée pour le matching serveur).
    Tag {
        tag: String,
        members: Vec<i32>,
        count: u16,
    },
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
            Self::Tag { members, count, .. } => {
                members.contains(&stack.id) && stack.count >= *count
            }
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

    pub fn register_blast(&mut self, r: FurnaceRecipe) {
        self.blast_furnace.push(r);
    }

    pub fn register_smoker(&mut self, r: FurnaceRecipe) {
        self.smoker.push(r);
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

    /// Encode le payload complet du paquet `CraftingData` (S→C), format 2169
    /// (JSON Mojang `CraftingDataPacketPayload`, gophertunnel
    /// `packet/crafting_data.go`, Cloudburst `CraftingDataSerializer_v2168`) :
    /// ```text
    /// shaped              VarU32 n + n × ShapedRecipe      (vecteur 0)
    /// shapeless           VarU32 n + n × ShapelessRecipe   (vecteur 1)
    /// multi               VarU32 0                         (vecteur 2)
    /// user-data shapeless VarU32 0                         (vecteur 3)
    /// shapeless chemistry VarU32 0                         (vecteur 4)
    /// shaped chemistry    VarU32 0                         (vecteur 5)
    /// smithing transform  VarU32 0                         (vecteur 6)
    /// smithing trim       VarU32 0                         (vecteur 7)
    /// potion mixes        VarU32 0
    /// container mixes     VarU32 0
    /// material reducers   VarU32 0
    /// clearRecipes        bool = true
    /// ```
    /// Plus d'id de type par recette : chaque type a son vecteur. Il n'y a
    /// pas de vecteur de four : les recettes de four partent en shapeless
    /// mono-ingrédient avec le tag du four, comme PMMP le faisait déjà
    /// (`CraftingDataCache` itère chaque `FurnaceType`).
    ///
    /// Les `recipeNetId` démarrent à 1 et suivent l'ordre shaped, shapeless,
    /// four, haut fourneau, fumoir. Notre handler `CraftRecipe` matche la
    /// grille directement, donc ces ids ne servent qu'à l'affichage / au
    /// recipe book côté client.
    ///
    /// Les ingrédients 2169 désignent l'item par son identifiant texte : une
    /// recette dont un ingrédient n'a pas de nom dans le registre d'items est
    /// omise (son net id reste consommé, les autres ne bougent pas).
    pub fn encode_crafting_data(&self) -> Vec<u8> {
        let encoder = RecipeEncoder {
            item_name: &item_registry::item_name_by_id,
            shield_id: item_registry::network_id("minecraft:shield"),
        };
        let (payload, skipped) = self.encode_crafting_data_with(&encoder);
        if skipped > 0 {
            tracing::warn!(
                "CraftingData : {} recettes omises (ingrédient absent du registre d'items)",
                skipped
            );
        }
        payload
    }

    /// Corps de [`Self::encode_crafting_data`] ; retourne le payload et le
    /// nombre de recettes omises.
    fn encode_crafting_data_with(&self, encoder: &RecipeEncoder) -> (Vec<u8>, usize) {
        let mut w = ProtoWriter::with_capacity(256 * 1024);
        let mut net_id = RECIPE_NET_ID_OFFSET;
        let mut skipped = 0;

        // Vecteur 0 : shaped.
        let mut shaped = RecipeVector::new();
        for r in &self.shaped {
            match encoder.resolve_all(&r.input) {
                Some(input) => {
                    encoder.encode_shaped_recipe(&mut shaped.body, r, &input, net_id);
                    shaped.count += 1;
                }
                None => skipped += 1,
            }
            net_id += 1;
        }
        shaped.write_to(&mut w);

        // Vecteur 1 : shapeless, puis les recettes de four.
        let mut shapeless = RecipeVector::new();
        for r in &self.shapeless {
            match encoder.resolve_all(&r.ingredients) {
                Some(input) => {
                    encoder.encode_shapeless_recipe(
                        &mut shapeless.body,
                        &input,
                        &r.output,
                        "crafting_table",
                        net_id,
                    );
                    shapeless.count += 1;
                }
                None => skipped += 1,
            }
            net_id += 1;
        }
        for (recipes, block_name) in [
            (&self.furnace, "furnace"),
            (&self.blast_furnace, "blast_furnace"),
            (&self.smoker, "smoker"),
        ] {
            for r in recipes {
                match encoder.resolve_all(std::slice::from_ref(&r.input)) {
                    Some(input) => {
                        encoder.encode_shapeless_recipe(
                            &mut shapeless.body,
                            &input,
                            std::slice::from_ref(&r.output),
                            block_name,
                            net_id,
                        );
                        shapeless.count += 1;
                    }
                    None => skipped += 1,
                }
                net_id += 1;
            }
        }
        shapeless.write_to(&mut w);

        w.write_var_u32(0); // vecteur 2 : multi
        w.write_var_u32(0); // vecteur 3 : user-data shapeless
        w.write_var_u32(0); // vecteur 4 : shapeless chemistry
        w.write_var_u32(0); // vecteur 5 : shaped chemistry
        w.write_var_u32(0); // vecteur 6 : smithing transform
        w.write_var_u32(0); // vecteur 7 : smithing trim
        w.write_var_u32(0); // potion mixes
        w.write_var_u32(0); // container mixes
        w.write_var_u32(0); // material reducers
        w.write_bool(true); // clearRecipes (PMMP passe toujours true)

        (w.into_bytes(), skipped)
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

// ── Helpers d'encodage wire (CraftingDataPacket 2169) ──

/// Un vecteur de recettes typé : compteur `VarUInt32` suivi des entrées.
/// Le corps est accumulé à part car des recettes peuvent être omises.
struct RecipeVector {
    count: u32,
    body: ProtoWriter,
}

impl RecipeVector {
    fn new() -> Self {
        Self {
            count: 0,
            body: ProtoWriter::with_capacity(64 * 1024),
        }
    }

    fn write_to(self, w: &mut ProtoWriter) {
        w.write_var_u32(self.count);
        w.write_raw(&self.body.into_bytes());
    }
}

/// Ingrédient prêt pour le wire : le descripteur 2169 porte l'identifiant
/// texte de l'item, plus son id réseau numérique.
enum WireIngredient<'a> {
    Empty,
    Item { name: &'a str, aux: i32, count: i32 },
    Tag { tag: &'a str, count: i32 },
}

/// Contexte d'encodage : résolution id réseau → identifiant texte (registre
/// d'items en production, table fixe en test) et id réseau du bouclier.
struct RecipeEncoder<'r> {
    item_name: &'r dyn Fn(i32) -> Option<&'static str>,
    shield_id: Option<i32>,
}

impl RecipeEncoder<'_> {
    /// `None` si l'item d'un ingrédient n'a pas d'identifiant connu.
    fn resolve<'a>(&self, ing: &'a RecipeIngredient) -> Option<WireIngredient<'a>> {
        Some(match ing {
            RecipeIngredient::Empty => WireIngredient::Empty,
            RecipeIngredient::Exact {
                item_id,
                meta,
                count,
            } => WireIngredient::Item {
                name: (self.item_name)(*item_id)?,
                aux: *meta as i32,
                count: i32::from(*count),
            },
            RecipeIngredient::AnyMeta { item_id, count } => WireIngredient::Item {
                name: (self.item_name)(*item_id)?,
                aux: RECIPE_INPUT_WILDCARD_META,
                count: i32::from(*count),
            },
            RecipeIngredient::Tag { tag, count, .. } => WireIngredient::Tag {
                tag,
                count: i32::from(*count),
            },
        })
    }

    fn resolve_all<'a>(&self, ings: &'a [RecipeIngredient]) -> Option<Vec<WireIngredient<'a>>> {
        ings.iter().map(|ing| self.resolve(ing)).collect()
    }

    /// `ShapedRecipe` 2169 (gophertunnel `recipe.go` `marshalShaped`,
    /// Cloudburst `CraftingDataSerializer_v2168::writeShapedRecipe`).
    fn encode_shaped_recipe(
        &self,
        w: &mut ProtoWriter,
        r: &ShapedRecipe,
        input: &[WireIngredient],
        net_id: u32,
    ) {
        write_recipe_id(w, net_id);
        w.write_var_i32(r.width as i32);
        w.write_var_i32(r.height as i32);
        // Nouveau en 2168 : compteur explicite, égal à width × height ; ordre
        // row-major, exactement comme stocké.
        w.write_var_u32(input.len() as u32);
        for ing in input {
            write_recipe_ingredient(w, ing);
        }
        w.write_var_u32(r.output.len() as u32);
        for out in &r.output {
            self.write_result(w, out);
        }
        write_nil_uuid(w);
        w.write_string("crafting_table");
        w.write_var_i32(RECIPE_PRIORITY);
        w.write_bool(true); // assumeSymmetry
        write_always_unlocked(w);
        w.write_var_u32(net_id); // recipeNetId
    }

    /// `ShapelessRecipe` 2169 (gophertunnel `recipe.go` `marshalShapeless`,
    /// Cloudburst `CraftingDataSerializer_v2168::writeShapelessRecipe`).
    /// Sert aussi pour les recettes de four (tag = "furnace", etc.).
    fn encode_shapeless_recipe(
        &self,
        w: &mut ProtoWriter,
        input: &[WireIngredient],
        output: &[ItemStack],
        block_name: &str,
        net_id: u32,
    ) {
        write_recipe_id(w, net_id);
        w.write_var_u32(input.len() as u32);
        for ing in input {
            write_recipe_ingredient(w, ing);
        }
        w.write_var_u32(output.len() as u32);
        for out in output {
            self.write_result(w, out);
        }
        write_nil_uuid(w);
        w.write_string(block_name);
        w.write_var_i32(RECIPE_PRIORITY);
        write_always_unlocked(w);
        w.write_var_u32(net_id); // recipeNetId
    }

    /// Résultat au format `NetworkItemInstanceDescriptor` 2169. Un item sans
    /// `extra_data` reçoit le blob userData minimal (allongé du
    /// `blockingTick` pour le bouclier).
    fn write_result(&self, w: &mut ProtoWriter, item: &ItemStack) {
        if item.extra_data.is_empty() {
            let user_data = item_user_data(None, self.shield_id == Some(item.id));
            write_item_instance(
                w,
                item.id,
                item.count,
                item.meta,
                item.block_runtime_id,
                &user_data,
            );
        } else {
            write_item_instance(
                w,
                item.id,
                item.count,
                item.meta,
                item.block_runtime_id,
                &item.extra_data,
            );
        }
    }
}

/// `recipeId` (string). PMMP utilise un int big-endian packé ; on émet juste le
/// net id en décimal : unique, lisible, et le client ne s'en sert pas pour le
/// dispatch (il utilise le `recipeNetId` numérique en fin de structure).
fn write_recipe_id(w: &mut ProtoWriter, net_id: u32) {
    w.write_string(&net_id.to_string());
}

/// Ingrédient de recette CraftingData 2169 (gophertunnel `writer.go`
/// `ItemDescriptorCount`, Cloudburst `BedrockCodecHelper_v2168::writeIngredient`
/// + `writeItemDescriptor`) :
/// ```text
/// VarU32 variante (0 invalide, 1 typé)
/// [0] VarI32 aux = 32767
/// [1] string type ("name" | "item_tag"), puis
///     "name"     : string identifiant + VarI32 aux (32767 = joker)
///     "item_tag" : string tag + VarI32 aux = 32767
/// VarI32 count
/// ```
/// Le JSON Mojang (`RecipeIngredient.json`) décrit un autre format
/// (`u8` type + count `u16`) ; gophertunnel et Cloudburst ne l'emploient que
/// pour les ingrédients d'ItemStackRequest. On suit les deux implémentations.
fn write_recipe_ingredient(w: &mut ProtoWriter, ing: &WireIngredient) {
    match ing {
        WireIngredient::Empty => {
            w.write_var_u32(DESCRIPTOR_VARIANT_INVALID);
            w.write_var_i32(RECIPE_INPUT_WILDCARD_META);
            w.write_var_i32(0);
        }
        WireIngredient::Item { name, aux, count } => {
            w.write_var_u32(DESCRIPTOR_VARIANT_TYPED);
            w.write_string(DESCRIPTOR_NAME_ITEM);
            w.write_string(name);
            w.write_var_i32(*aux);
            w.write_var_i32(*count);
        }
        WireIngredient::Tag { tag, count } => {
            w.write_var_u32(DESCRIPTOR_VARIANT_TYPED);
            w.write_string(DESCRIPTOR_NAME_TAG);
            w.write_string(tag);
            w.write_var_i32(RECIPE_INPUT_WILDCARD_META);
            w.write_var_i32(*count);
        }
    }
}

/// UUID NIL (16 octets nuls). PMMP `CraftingDataCache` passe `Uuid::NIL`.
fn write_nil_uuid(w: &mut ProtoWriter) {
    w.write_raw(&[0u8; 16]);
}

/// Pas de condition de déblocage : `bool hasUnlock = true`, contexte
/// AlwaysUnlocked en VarInt32 zigzag (`02`), puis `bool false` (la liste
/// d'ingrédients n'est présente que pour le contexte None). Octets `01 02 00`
/// (gophertunnel `RecipeUnlockRequirement::Marshal`, Cloudburst
/// `CraftingDataSerializer_v2168::writeRequirement`) ; même sémantique que le
/// `bool true` de PMMP 1001.
fn write_always_unlocked(w: &mut ProtoWriter) {
    w.write_bool(true);
    w.write_var_i32(UNLOCK_CONTEXT_ALWAYS_UNLOCKED);
    w.write_bool(false);
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
    fn tag_ingredient_matches_any_member() {
        let ing = RecipeIngredient::Tag {
            tag: "minecraft:planks".to_string(),
            members: vec![10, 11, 12],
            count: 1,
        };
        assert!(ing.matches(&stack(11, 1)));
        assert!(!ing.matches(&stack(99, 1)));
        // count insuffisant → pas de match.
        let ing2 = RecipeIngredient::Tag {
            tag: "minecraft:planks".to_string(),
            members: vec![10],
            count: 2,
        };
        assert!(!ing2.matches(&stack(10, 1)));
        assert!(ing2.matches(&stack(10, 2)));
    }

    use mc_rs_proto::io::ProtoReader;
    use mc_rs_proto::packets::world::CraftingData;

    const SHIELD_ID: i32 = 900;

    /// Registre d'items factice : les ingrédients 2169 sont nommés.
    fn test_item_name(id: i32) -> Option<&'static str> {
        match id {
            5 => Some("minecraft:oak_log"),
            10 => Some("minecraft:oak_planks"),
            15 => Some("minecraft:iron_ore"),
            _ => None,
        }
    }

    fn encode_for_test(mgr: &CraftingManager) -> (Vec<u8>, usize) {
        let encoder = RecipeEncoder {
            item_name: &test_item_name,
            shield_id: Some(SHIELD_ID),
        };
        mgr.encode_crafting_data_with(&encoder)
    }

    fn assert_item_ingredient(r: &mut ProtoReader, name: &str, aux: i32, count: i32) {
        assert_eq!(r.read_var_u32().unwrap(), DESCRIPTOR_VARIANT_TYPED);
        assert_eq!(r.read_string().unwrap(), "name");
        assert_eq!(r.read_string().unwrap(), name);
        assert_eq!(r.read_var_i32().unwrap(), aux);
        assert_eq!(r.read_var_i32().unwrap(), count);
    }

    /// Lit un `NetworkItemInstanceDescriptor` 2169 ; retourne (id, count, blob).
    fn read_result(r: &mut ProtoReader) -> (i32, u16, Vec<u8>) {
        let id = r.read_var_i32().unwrap();
        let count = r.read_u16_le().unwrap();
        assert_eq!(r.read_var_u32().unwrap(), 0); // meta
        assert_eq!(r.read_var_i32().unwrap(), 0); // block runtime id
        (id, count, r.read_byte_array().unwrap())
    }

    /// UUID nil, tag, priorité.
    fn assert_recipe_trailer_start(r: &mut ProtoReader, block_name: &str) {
        assert_eq!(r.read_raw(16).unwrap(), vec![0u8; 16]);
        assert_eq!(r.read_string().unwrap(), block_name);
        assert_eq!(r.read_var_i32().unwrap(), RECIPE_PRIORITY);
    }

    /// hasUnlock + contexte AlwaysUnlocked (zigzag 1 = 0x02) + pas de liste.
    fn assert_always_unlocked(r: &mut ProtoReader) {
        assert_eq!(r.read_raw(3).unwrap(), vec![0x01, 0x02, 0x00]);
    }

    #[test]
    fn crafting_data_payload_roundtrips() {
        let mut mgr = CraftingManager::new();
        // 1 shaped avec les quatre sortes d'ingrédients (dont un slot vide).
        mgr.register_shaped(ShapedRecipe::new(
            2,
            2,
            vec![
                RecipeIngredient::AnyMeta {
                    item_id: 10,
                    count: 1,
                },
                RecipeIngredient::Empty,
                RecipeIngredient::Exact {
                    item_id: 10,
                    meta: 3,
                    count: 1,
                },
                RecipeIngredient::Tag {
                    tag: "minecraft:planks".to_string(),
                    members: vec![10, 11],
                    count: 1,
                },
            ],
            vec![stack(58, 1)],
        ));
        // 1 shapeless.
        mgr.register_shapeless(ShapelessRecipe::new(
            vec![RecipeIngredient::AnyMeta {
                item_id: 5,
                count: 1,
            }],
            vec![stack(280, 4)],
        ));
        // 1 furnace.
        mgr.register_furnace(FurnaceRecipe {
            input: RecipeIngredient::AnyMeta {
                item_id: 15,
                count: 1,
            },
            output: stack(265, 1),
            cook_time_ticks: 200,
            xp: 0.7,
        });

        let (payload, skipped) = encode_for_test(&mgr);
        assert_eq!(skipped, 0);
        let mut r = ProtoReader::new(&payload);

        // Vecteur 0 : shaped.
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(r.read_string().unwrap(), "1"); // recipeId = net_id
        assert_eq!(r.read_var_i32().unwrap(), 2); // width
        assert_eq!(r.read_var_i32().unwrap(), 2); // height
        assert_eq!(r.read_var_u32().unwrap(), 4); // nombre d'ingrédients
        assert_item_ingredient(&mut r, "minecraft:oak_planks", 32767, 1);
        // Slot vide : variante 0, aux 32767, count 0.
        assert_eq!(r.read_var_u32().unwrap(), DESCRIPTOR_VARIANT_INVALID);
        assert_eq!(r.read_var_i32().unwrap(), 32767);
        assert_eq!(r.read_var_i32().unwrap(), 0);
        assert_item_ingredient(&mut r, "minecraft:oak_planks", 3, 1);
        assert_eq!(r.read_var_u32().unwrap(), DESCRIPTOR_VARIANT_TYPED);
        assert_eq!(r.read_string().unwrap(), "item_tag");
        assert_eq!(r.read_string().unwrap(), "minecraft:planks");
        assert_eq!(r.read_var_i32().unwrap(), 32767);
        assert_eq!(r.read_var_i32().unwrap(), 1);
        assert_eq!(r.read_var_u32().unwrap(), 1); // 1 résultat
        assert_eq!(read_result(&mut r), (58, 1, vec![0u8; 10]));
        assert_recipe_trailer_start(&mut r, "crafting_table");
        assert!(r.read_bool().unwrap()); // assumeSymmetry
        assert_always_unlocked(&mut r);
        assert_eq!(r.read_var_u32().unwrap(), 1); // recipeNetId

        // Vecteur 1 : shapeless puis four.
        assert_eq!(r.read_var_u32().unwrap(), 2);
        assert_eq!(r.read_string().unwrap(), "2");
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_item_ingredient(&mut r, "minecraft:oak_log", 32767, 1);
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(read_result(&mut r), (280, 4, vec![0u8; 10]));
        assert_recipe_trailer_start(&mut r, "crafting_table");
        assert_always_unlocked(&mut r);
        assert_eq!(r.read_var_u32().unwrap(), 2);

        assert_eq!(r.read_string().unwrap(), "3");
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_item_ingredient(&mut r, "minecraft:iron_ore", 32767, 1);
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(read_result(&mut r), (265, 1, vec![0u8; 10]));
        assert_recipe_trailer_start(&mut r, "furnace");
        assert_always_unlocked(&mut r);
        assert_eq!(r.read_var_u32().unwrap(), 3);

        // Vecteurs 2 à 7, potions, conteneurs, reducers : vides ; clearRecipes.
        for _ in 2..CraftingData::VECTOR_COUNT {
            assert_eq!(r.read_var_u32().unwrap(), 0);
        }
        assert!(r.read_bool().unwrap());
        assert_eq!(r.remaining(), 0);
    }

    /// Un ingrédient sans nom connu fait omettre la recette, mais son net id
    /// reste consommé : les suivantes gardent le leur.
    #[test]
    fn recipe_with_unnamed_ingredient_is_skipped() {
        let mut mgr = CraftingManager::new();
        mgr.register_shapeless(ShapelessRecipe::new(
            vec![RecipeIngredient::AnyMeta {
                item_id: 4242,
                count: 1,
            }],
            vec![stack(280, 1)],
        ));
        mgr.register_shapeless(ShapelessRecipe::new(
            vec![RecipeIngredient::AnyMeta {
                item_id: 5,
                count: 1,
            }],
            vec![stack(280, 4)],
        ));

        let (payload, skipped) = encode_for_test(&mgr);
        assert_eq!(skipped, 1);
        let mut r = ProtoReader::new(&payload);
        assert_eq!(r.read_var_u32().unwrap(), 0); // aucun shaped
        assert_eq!(r.read_var_u32().unwrap(), 1); // un seul shapeless
        assert_eq!(r.read_string().unwrap(), "2");
    }

    /// Le blob userData d'un bouclier porte le `blockingTick` (i64) en plus.
    #[test]
    fn shield_result_carries_blocking_tick() {
        let mut mgr = CraftingManager::new();
        mgr.register_shapeless(ShapelessRecipe::new(
            vec![RecipeIngredient::AnyMeta {
                item_id: 5,
                count: 1,
            }],
            vec![stack(SHIELD_ID, 1)],
        ));
        let (payload, _) = encode_for_test(&mgr);
        let mut r = ProtoReader::new(&payload);
        r.read_var_u32().unwrap(); // shaped
        r.read_var_u32().unwrap(); // shapeless
        r.read_string().unwrap();
        r.read_var_u32().unwrap();
        assert_item_ingredient(&mut r, "minecraft:oak_log", 32767, 1);
        r.read_var_u32().unwrap();
        let (id, _, user_data) = read_result(&mut r);
        assert_eq!(id, SHIELD_ID);
        assert_eq!(user_data, vec![0u8; 18]);
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
