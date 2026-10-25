//! Crafting — port sélectif de `.reference/PocketMine-MP/src/crafting/*`.
//!
//! Couvre : recipes shaped (3x3 avec pattern), shapeless (liste d'ingrédients
//! quelconque ordre), furnace (smelting 1→1), et les autres vecteurs du paquet
//! CraftingData (recettes à données, multiples, forge, potions, contenants).
//!
//! Les recettes viennent de BDS (`recipes_bds.rs`, `data/recipes/bds.json`) :
//! chacune garde son identité BDS (`RecipeInfo`), envoyée telle quelle au
//! client. `RECIPE_DB` static partagé : initialisé une fois au boot (main.rs).
//! Permet à `InventoryManager` d'accéder aux recettes sans avoir à passer la
//! référence par paramètre.

use std::sync::OnceLock;

use mc_rs_proto::io::ProtoWriter;
use mc_rs_proto::packets::player::ItemStack;
use mc_rs_proto::packets::world::{item_user_data, write_item_instance};

use crate::item_registry;

pub static RECIPE_DB: OnceLock<CraftingManager> = OnceLock::new();

/// Payload encodé du paquet `CraftingData` (S→C), construit une seule fois au
/// boot après `recipes_bds::register_all`. Envoyé tel quel à chaque PreSpawn
/// (cf. `connection/spawn.rs`).
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

/// Priorité par défaut d'une recette sans origine BDS (PMMP
/// `CraftingDataCache` hardcode 50 partout ; BDS va de -1 à 50).
const RECIPE_PRIORITY: i32 = 50;

/// Bloc des recettes de l'établi et de la grille 2×2 de l'inventaire.
pub const CRAFTING_TABLE: &str = "crafting_table";

/// Identité d'une recette dans CraftingData : identifiant texte, identifiant
/// réseau (renvoyé par le client dans `CraftRecipe`), UUID, bloc et priorité.
/// Repris tels quels de BDS par `recipes_bds`.
#[derive(Debug, Clone)]
pub struct RecipeInfo {
    /// Vide : l'identifiant réseau en décimal est envoyé à la place.
    pub id: String,
    /// 0 : attribué à l'enregistrement (`CraftingManager::register_*`).
    pub net_id: u32,
    /// Octets dans l'ordre RFC 4122.
    pub uuid: [u8; 16],
    pub block: String,
    pub priority: i32,
}

impl Default for RecipeInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            net_id: 0,
            uuid: [0; 16],
            block: CRAFTING_TABLE.to_string(),
            priority: RECIPE_PRIORITY,
        }
    }
}

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
    pub info: RecipeInfo,
    pub width: usize,
    pub height: usize,
    /// Le motif peut être retourné horizontalement (BDS : toujours vrai).
    pub assume_symmetry: bool,
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
            info: RecipeInfo::default(),
            width,
            height,
            assume_symmetry: true,
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
                    || (self.assume_symmetry && self.matches_at(grid, grid_size, dx, dy, true))
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
    pub info: RecipeInfo,
    pub ingredients: Vec<RecipeIngredient>,
    pub output: Vec<ItemStack>,
}

impl ShapelessRecipe {
    pub fn new(ingredients: Vec<RecipeIngredient>, output: Vec<ItemStack>) -> Self {
        Self {
            info: RecipeInfo::default(),
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

/// Recette de forge : amélioration (avec résultat) ou garniture (sans).
#[derive(Debug, Clone)]
pub struct SmithingRecipe {
    pub id: String,
    pub net_id: u32,
    pub block: String,
    pub template: RecipeIngredient,
    pub base: RecipeIngredient,
    pub addition: RecipeIngredient,
    /// `None` pour une garniture (le résultat est l'armure garnie).
    pub output: Option<ItemStack>,
}

/// Mélange d'alambic : `(id réseau, meta)` de l'entrée, du réactif, du résultat.
#[derive(Debug, Clone, Copy)]
pub struct PotionMix {
    pub input: (i32, i32),
    pub reagent: (i32, i32),
    pub output: (i32, i32),
}

/// Changement de contenant d'alambic (potion → splash → lingering).
#[derive(Debug, Clone, Copy)]
pub struct ContainerMix {
    pub input: i32,
    pub reagent: i32,
    pub output: i32,
}

/// Recette « multiple » (cartes, feux d'artifice…) : résolue par le client.
#[derive(Debug, Clone, Copy)]
pub struct MultiRecipe {
    pub uuid: [u8; 16],
    pub net_id: u32,
}

/// Recette désignée par son identifiant réseau (`CraftRecipe` du client).
pub enum CraftingRecipeRef<'a> {
    Shaped(&'a ShapedRecipe),
    Shapeless(&'a ShapelessRecipe),
    /// Recette à données utilisateur : le résultat reprend le NBT de l'entrée.
    UserData(&'a ShapelessRecipe),
}

/// Manager global qui garde toutes les recettes. Port PMMP `CraftingManager`.
#[derive(Debug, Default, Clone)]
pub struct CraftingManager {
    pub shaped: Vec<ShapedRecipe>,
    /// Toutes les recettes sans forme, quel que soit leur bloc (établi, four,
    /// tailleur de pierre…), dans l'ordre d'envoi.
    pub shapeless: Vec<ShapelessRecipe>,
    pub user_data_shapeless: Vec<ShapelessRecipe>,
    pub multi: Vec<MultiRecipe>,
    pub smithing_transform: Vec<SmithingRecipe>,
    pub smithing_trim: Vec<SmithingRecipe>,
    pub potion_mixes: Vec<PotionMix>,
    pub container_mixes: Vec<ContainerMix>,
    /// Recettes de cuisson utilisées par les fours (`furnace.rs`) ; envoyées au
    /// client via `shapeless` (bloc `furnace`, `blast_furnace`, `smoker`).
    pub furnace: Vec<FurnaceRecipe>,
    pub blast_furnace: Vec<FurnaceRecipe>,
    pub smoker: Vec<FurnaceRecipe>,
    /// Prochain identifiant réseau libre pour une recette sans origine BDS.
    next_net_id: u32,
}

impl CraftingManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attribue un identifiant réseau à une recette qui n'en a pas, et réserve
    /// celui d'une recette qui en a un.
    fn assign_net_id(&mut self, info: &mut RecipeInfo) {
        if self.next_net_id < RECIPE_NET_ID_OFFSET {
            self.next_net_id = RECIPE_NET_ID_OFFSET;
        }
        if info.net_id == 0 {
            info.net_id = self.next_net_id;
        }
        self.next_net_id = self.next_net_id.max(info.net_id + 1);
    }

    pub fn register_shaped(&mut self, mut r: ShapedRecipe) {
        self.assign_net_id(&mut r.info);
        self.shaped.push(r);
    }

    pub fn register_shapeless(&mut self, mut r: ShapelessRecipe) {
        self.assign_net_id(&mut r.info);
        self.shapeless.push(r);
    }

    pub fn register_user_data_shapeless(&mut self, mut r: ShapelessRecipe) {
        self.assign_net_id(&mut r.info);
        self.user_data_shapeless.push(r);
    }

    /// Recette d'établi ou de forge désignée par l'identifiant réseau que le
    /// client renvoie dans `CraftRecipe` (PMMP `CraftingManager::getCraftingRecipeIndex`).
    pub fn recipe_by_net_id(&self, net_id: u32) -> Option<CraftingRecipeRef<'_>> {
        if let Some(r) = self.shaped.iter().find(|r| r.info.net_id == net_id) {
            return Some(CraftingRecipeRef::Shaped(r));
        }
        if let Some(r) = self.shapeless.iter().find(|r| r.info.net_id == net_id) {
            return Some(CraftingRecipeRef::Shapeless(r));
        }
        self.user_data_shapeless
            .iter()
            .find(|r| r.info.net_id == net_id)
            .map(CraftingRecipeRef::UserData)
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

    /// Trouve la première recette d'établi qui match la grille (shaped ou
    /// shapeless). Les recettes des autres blocs (four, tailleur de pierre…)
    /// et les recettes à données sont exclues.
    pub fn match_crafting(&self, grid: &[ItemStack], grid_size: usize) -> Option<&[ItemStack]> {
        for r in &self.shaped {
            if r.info.block == CRAFTING_TABLE && r.matches(grid, grid_size) {
                return Some(&r.output);
            }
        }
        for r in &self.shapeless {
            if r.info.block == CRAFTING_TABLE && r.matches(grid) {
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

    /// Encode le payload complet du paquet `CraftingData` (S→C), format 2193
    /// (JSON Mojang `CraftingDataPacketPayload`, gophertunnel
    /// `packet/crafting_data.go`, Cloudburst `CraftingDataSerializer_v2168`) :
    /// ```text
    /// shaped              VarU32 n + n × ShapedRecipe      (vecteur 0)
    /// shapeless           VarU32 n + n × ShapelessRecipe   (vecteur 1)
    /// multi               VarU32 n + n × (UUID, net id)    (vecteur 2)
    /// user-data shapeless VarU32 n + n × ShapelessRecipe   (vecteur 3)
    /// shapeless chemistry VarU32 0                         (vecteur 4)
    /// shaped chemistry    VarU32 0                         (vecteur 5)
    /// smithing transform  VarU32 n                         (vecteur 6)
    /// smithing trim       VarU32 n                         (vecteur 7)
    /// potion mixes        VarU32 n + n × 6 VarI32
    /// container mixes     VarU32 n + n × 3 VarI32
    /// material reducers   VarU32 0
    /// clearRecipes        bool = true
    /// ```
    /// Chaque type a son vecteur. Il n'y a pas de vecteur de four : comme chez
    /// BDS, les recettes de cuisson sont des shapeless au bloc `furnace`,
    /// `blast_furnace`, `smoker`, `campfire`… Les listes `furnace`,
    /// `blast_furnace` et `smoker` du manager ne sont pas envoyées.
    ///
    /// Chaque recette part avec son `RecipeInfo` (identifiant, UUID, bloc,
    /// priorité, identifiant réseau), repris de BDS. La condition de déblocage
    /// reste « toujours débloquée » : BDS en décrit une par recette (stockée
    /// dans `bds.json`) mais débloque ensuite par le paquet UnlockedRecipes,
    /// que mc-rs n'envoie pas.
    ///
    /// Les ingrédients désignent l'item par son identifiant texte : une recette
    /// dont un ingrédient n'a pas de nom dans le registre d'items est omise.
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
        let mut w = ProtoWriter::with_capacity(1024 * 1024);
        let mut skipped = 0;

        // Vecteur 0 : shaped.
        let mut shaped = RecipeVector::new();
        for r in &self.shaped {
            match encoder.resolve_all(&r.input) {
                Some(input) => {
                    encoder.encode_shaped_recipe(&mut shaped.body, r, &input);
                    shaped.count += 1;
                }
                None => skipped += 1,
            }
        }
        shaped.write_to(&mut w);

        // Vecteur 1 : shapeless (tous blocs, cuisson comprise).
        let (shapeless, n) = encoder.shapeless_vector(&self.shapeless);
        skipped += n;
        shapeless.write_to(&mut w);

        // Vecteur 2 : multi.
        w.write_var_u32(self.multi.len() as u32);
        for m in &self.multi {
            w.write_uuid(&m.uuid);
            w.write_var_u32(m.net_id);
        }

        // Vecteur 3 : user-data shapeless.
        let (user_data, n) = encoder.shapeless_vector(&self.user_data_shapeless);
        skipped += n;
        user_data.write_to(&mut w);

        w.write_var_u32(0); // vecteur 4 : shapeless chemistry
        w.write_var_u32(0); // vecteur 5 : shaped chemistry

        // Vecteurs 6 et 7 : forge (amélioration, garniture).
        for recipes in [&self.smithing_transform, &self.smithing_trim] {
            let mut v = RecipeVector::new();
            for r in recipes {
                if encoder.encode_smithing_recipe(&mut v.body, r) {
                    v.count += 1;
                } else {
                    skipped += 1;
                }
            }
            v.write_to(&mut w);
        }

        // Mélanges d'alambic (gophertunnel `PotionRecipe`,
        // `PotionContainerChangeRecipe`).
        w.write_var_u32(self.potion_mixes.len() as u32);
        for m in &self.potion_mixes {
            for (id, meta) in [m.input, m.reagent, m.output] {
                w.write_var_i32(id);
                w.write_var_i32(meta);
            }
        }
        w.write_var_u32(self.container_mixes.len() as u32);
        for m in &self.container_mixes {
            w.write_var_i32(m.input);
            w.write_var_i32(m.reagent);
            w.write_var_i32(m.output);
        }

        w.write_var_u32(0); // material reducers
        w.write_bool(true); // clearRecipes (PMMP et BDS passent true)

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

    /// Un vecteur de recettes sans forme ; retourne aussi le nombre d'omises.
    fn shapeless_vector(&self, recipes: &[ShapelessRecipe]) -> (RecipeVector, usize) {
        let mut v = RecipeVector::new();
        let mut skipped = 0;
        for r in recipes {
            match self.resolve_all(&r.ingredients) {
                Some(input) => {
                    self.encode_shapeless_recipe(&mut v.body, r, &input);
                    v.count += 1;
                }
                None => skipped += 1,
            }
        }
        (v, skipped)
    }

    /// `ShapedRecipe` 2169 (gophertunnel `recipe.go` `marshalShaped`,
    /// Cloudburst `CraftingDataSerializer_v2168::writeShapedRecipe`).
    fn encode_shaped_recipe(
        &self,
        w: &mut ProtoWriter,
        r: &ShapedRecipe,
        input: &[WireIngredient],
    ) {
        write_recipe_id(w, &r.info);
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
        w.write_uuid(&r.info.uuid);
        w.write_string(&r.info.block);
        w.write_var_i32(r.info.priority);
        w.write_bool(r.assume_symmetry);
        write_always_unlocked(w);
        w.write_var_u32(r.info.net_id); // recipeNetId
    }

    /// `ShapelessRecipe` 2169 (gophertunnel `recipe.go` `marshalShapeless`,
    /// Cloudburst `CraftingDataSerializer_v2168::writeShapelessRecipe`).
    /// Sert aussi pour les recettes de cuisson (bloc `furnace`, etc.) et les
    /// recettes à données (même structure).
    fn encode_shapeless_recipe(
        &self,
        w: &mut ProtoWriter,
        r: &ShapelessRecipe,
        input: &[WireIngredient],
    ) {
        write_recipe_id(w, &r.info);
        w.write_var_u32(input.len() as u32);
        for ing in input {
            write_recipe_ingredient(w, ing);
        }
        w.write_var_u32(r.output.len() as u32);
        for out in &r.output {
            self.write_result(w, out);
        }
        w.write_uuid(&r.info.uuid);
        w.write_string(&r.info.block);
        w.write_var_i32(r.info.priority);
        write_always_unlocked(w);
        w.write_var_u32(r.info.net_id); // recipeNetId
    }

    /// `SmithingTransformRecipe` / `SmithingTrimRecipe` (gophertunnel
    /// `recipe.go`) : identifiant, gabarit, base, ajout, [résultat], bloc, net
    /// id. Retourne `false` si un ingrédient n'a pas de nom connu.
    fn encode_smithing_recipe(&self, w: &mut ProtoWriter, r: &SmithingRecipe) -> bool {
        let (Some(template), Some(base), Some(addition)) = (
            self.resolve(&r.template),
            self.resolve(&r.base),
            self.resolve(&r.addition),
        ) else {
            return false;
        };
        let parts = [template, base, addition];
        w.write_string(&r.id);
        for ing in &parts {
            write_recipe_ingredient(w, ing);
        }
        if let Some(out) = &r.output {
            self.write_result(w, out);
        }
        w.write_string(&r.block);
        w.write_var_u32(r.net_id);
        true
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

/// `recipeId` (string) : identifiant BDS (`minecraft:barrel`, `bed_dye_0_1`…),
/// ou l'identifiant réseau en décimal pour une recette sans origine BDS. Le
/// client dispatche sur le `recipeNetId` numérique en fin de structure.
fn write_recipe_id(w: &mut ProtoWriter, info: &RecipeInfo) {
    if info.id.is_empty() {
        w.write_string(&info.net_id.to_string());
    } else {
        w.write_string(&info.id);
    }
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
        // 1 recette de cuisson : shapeless au bloc du four, comme chez BDS.
        let mut smelting = ShapelessRecipe::new(
            vec![RecipeIngredient::AnyMeta {
                item_id: 15,
                count: 1,
            }],
            vec![stack(265, 1)],
        );
        smelting.info.block = "furnace".to_string();
        mgr.register_shapeless(smelting);

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

        // Vecteur 1 : shapeless, cuisson comprise.
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
