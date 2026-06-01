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

pub static RECIPE_DB: OnceLock<CraftingManager> = OnceLock::new();

/// Payload encodé du paquet `CraftingData` (S→C), construit une seule fois au
/// boot après `recipes_vanilla::register_all`. Envoyé tel quel à chaque
/// PreSpawn (cf. `connection/spawn.rs`).
pub static CRAFTING_DATA_PAYLOAD: OnceLock<Vec<u8>> = OnceLock::new();

// ── Constantes wire CraftingDataPacket (PMMP `CraftingDataPacket`) ──
/// `ENTRY_SHAPELESS` — utilisé aussi pour les recettes de four (PMMP les envoie
/// en shapeless avec le block name "furnace"/"blast_furnace"/"smoker").
const ENTRY_SHAPELESS: i32 = 0;
const ENTRY_SHAPED: i32 = 1;

// ── ItemDescriptorType (PMMP `ItemDescriptorType`) ──
const DESC_INT_ID_META: u8 = 1;
const DESC_TAG: u8 = 3;

/// Meta wildcard côté recette (PMMP `TypeConverter::RECIPE_INPUT_WILDCARD_META`).
const RECIPE_INPUT_WILDCARD_META: i16 = 0x7fff;

/// Priorité de recette (PMMP `CraftingDataCache` hardcode 50 partout).
const RECIPE_PRIORITY: i32 = 50;

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

    /// Encode le payload complet du paquet `CraftingData` (S→C).
    ///
    /// Port fidèle de PMMP `CraftingDataCache::buildCraftingDataCache` +
    /// `CraftingDataPacket::encodePayload` (BedrockProtocol tag
    /// `57.1.0+bedrock-1.26.20`) :
    /// ```text
    /// recipe_count (VarU32)
    /// for each recipe: type_id (VarI32) + <recipe payload>
    /// potion_type_recipes_count   (VarU32) = 0
    /// potion_container_recipes    (VarU32) = 0
    /// material_reducer_recipes    (VarU32) = 0
    /// clean_recipes (bool) = true
    /// ```
    ///
    /// - shaped → `ENTRY_SHAPED`, block "crafting_table", symmetric=true.
    /// - shapeless → `ENTRY_SHAPELESS`, block "crafting_table".
    /// - furnace → `ENTRY_SHAPELESS` avec block "furnace" (exactement comme
    ///   PMMP, qui n'utilise PAS `ENTRY_FURNACE` mais réencode en shapeless).
    ///
    /// Les `recipeNetId` démarrent à 1 (le client rejette l'id 0 depuis
    /// 1.21.100 — `CraftingDataCache::RECIPE_ID_OFFSET`). Notre handler
    /// `CraftRecipe` matche la grille directement, donc ces ids ne servent
    /// qu'à l'affichage / au recipe book côté client.
    pub fn encode_crafting_data(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(256 * 1024);

        let total = self.shaped.len()
            + self.shapeless.len()
            + self.furnace.len()
            + self.blast_furnace.len()
            + self.smoker.len();
        w.write_var_u32(total as u32);

        let mut net_id: u32 = 1; // RECIPE_ID_OFFSET

        for r in &self.shaped {
            w.write_var_i32(ENTRY_SHAPED);
            encode_shaped_recipe(&mut w, r, net_id);
            net_id += 1;
        }
        for r in &self.shapeless {
            w.write_var_i32(ENTRY_SHAPELESS);
            encode_shapeless_recipe(&mut w, &r.ingredients, &r.output, "crafting_table", net_id);
            net_id += 1;
        }
        // PMMP encode les recettes de four comme shapeless mono-ingrédient avec
        // le block name du four (`CraftingDataCache` itère chaque `FurnaceType`).
        for (recipes, block_name) in [
            (&self.furnace, "furnace"),
            (&self.blast_furnace, "blast_furnace"),
            (&self.smoker, "smoker"),
        ] {
            for r in recipes {
                w.write_var_i32(ENTRY_SHAPELESS);
                let inputs = std::slice::from_ref(&r.input);
                let outputs = std::slice::from_ref(&r.output);
                encode_shapeless_recipe(&mut w, inputs, outputs, block_name, net_id);
                net_id += 1;
            }
        }

        w.write_var_u32(0); // potion type recipes
        w.write_var_u32(0); // potion container recipes
        w.write_var_u32(0); // material reducer recipes
        w.write_bool(true); // clean recipes (PMMP passe toujours true)

        w.into_bytes()
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

// ── Helpers d'encodage wire (CraftingDataPacket) ──

/// Encode une `ProtocolShapedRecipe` (PMMP `ShapedRecipe::encode`).
fn encode_shaped_recipe(w: &mut ProtoWriter, r: &ShapedRecipe, net_id: u32) {
    write_recipe_id(w, net_id);
    w.write_var_i32(r.width as i32);
    w.write_var_i32(r.height as i32);
    // input en ordre row-major (height × width), exactement comme stocké.
    for ing in &r.input {
        write_recipe_ingredient(w, ing);
    }
    w.write_var_u32(r.output.len() as u32);
    for out in &r.output {
        write_item_stack_without_id(w, out);
    }
    write_nil_uuid(w);
    w.write_string("crafting_table");
    w.write_var_i32(RECIPE_PRIORITY);
    w.write_bool(true); // symmetric
    write_no_unlocking_requirement(w);
    w.write_var_u32(net_id); // recipeNetId
}

/// Encode une `ProtocolShapelessRecipe` (PMMP `ShapelessRecipe::encode`).
/// Sert aussi pour les recettes de four (block name = "furnace").
fn encode_shapeless_recipe(
    w: &mut ProtoWriter,
    ingredients: &[RecipeIngredient],
    output: &[ItemStack],
    block_name: &str,
    net_id: u32,
) {
    write_recipe_id(w, net_id);
    w.write_var_u32(ingredients.len() as u32);
    for ing in ingredients {
        write_recipe_ingredient(w, ing);
    }
    w.write_var_u32(output.len() as u32);
    for out in output {
        write_item_stack_without_id(w, out);
    }
    write_nil_uuid(w);
    w.write_string(block_name);
    w.write_var_i32(RECIPE_PRIORITY);
    write_no_unlocking_requirement(w);
    w.write_var_u32(net_id); // recipeNetId
}

/// `recipeId` (string). PMMP utilise un int big-endian packé ; on émet juste le
/// net id en décimal — unique, lisible, et le client ne s'en sert pas pour le
/// dispatch (il utilise le `recipeNetId` numérique en fin de structure).
fn write_recipe_id(w: &mut ProtoWriter, net_id: u32) {
    w.write_string(&net_id.to_string());
}

/// `RecipeIngredient` wire (PMMP `CommonTypes::putRecipeIngredient`) :
/// descriptor_type (u8) + <descriptor> + count (VarI32).
fn write_recipe_ingredient(w: &mut ProtoWriter, ing: &RecipeIngredient) {
    match ing {
        RecipeIngredient::Empty => {
            w.write_u8(0); // null descriptor
            w.write_var_i32(0);
        }
        RecipeIngredient::Exact {
            item_id,
            meta,
            count,
        } => {
            w.write_u8(DESC_INT_ID_META);
            write_int_id_meta_descriptor(w, *item_id, *meta as i16);
            w.write_var_i32(*count as i32);
        }
        RecipeIngredient::AnyMeta { item_id, count } => {
            w.write_u8(DESC_INT_ID_META);
            write_int_id_meta_descriptor(w, *item_id, RECIPE_INPUT_WILDCARD_META);
            w.write_var_i32(*count as i32);
        }
        RecipeIngredient::Tag { tag, count, .. } => {
            w.write_u8(DESC_TAG);
            w.write_string(tag); // TagItemDescriptor
            w.write_var_i32(*count as i32);
        }
    }
}

/// `IntIdMetaItemDescriptor::write` : i16 LE id, puis i16 LE meta SI id != 0.
fn write_int_id_meta_descriptor(w: &mut ProtoWriter, id: i32, meta: i16) {
    w.write_i16_le(id as i16);
    if id != 0 {
        w.write_i16_le(meta);
    }
}

/// `getItemStackWithoutStackId` côté écriture (PMMP `putItemStackWithoutStackId`)
/// : id (VarI32), puis si id != 0 : count (u16 LE), meta (VarU32),
/// block_runtime_id (VarI32), raw_extra_data (byte array).
fn write_item_stack_without_id(w: &mut ProtoWriter, item: &ItemStack) {
    w.write_var_i32(item.id);
    if item.id == 0 {
        return;
    }
    w.write_u16_le(item.count);
    w.write_var_u32(item.meta);
    w.write_var_i32(item.block_runtime_id);
    // extra data minimal : NBT len=0 (i16) + canPlaceOn=0 (i32) + canDestroy=0 (i32).
    // Identique à `minimal_item_extra_data` côté mc-rs-proto.
    let extra = [0u8; 10];
    w.write_byte_array(&extra);
}

/// UUID NIL (16 octets nuls). PMMP `CraftingDataCache` passe `Uuid::NIL`.
fn write_nil_uuid(w: &mut ProtoWriter) {
    w.write_raw(&[0u8; 16]);
}

/// `RecipeUnlockingRequirement::write` avec `unlockingIngredients === null` :
/// un seul bool `true` (pas de liste). PMMP `$noUnlockingRequirement`.
fn write_no_unlocking_requirement(w: &mut ProtoWriter) {
    w.write_bool(true);
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

    #[test]
    fn crafting_data_payload_roundtrips() {
        use mc_rs_proto::io::ProtoReader;

        let mut mgr = CraftingManager::new();
        // 1 shaped (2x2 oak planks → crafting table), avec un slot Empty pour
        // forcer le chemin "null descriptor".
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
                    meta: 0,
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

        let payload = mgr.encode_crafting_data();
        let mut r = ProtoReader::new(&payload);

        let recipe_count = r.read_var_u32().unwrap();
        assert_eq!(recipe_count, 3);

        // Recette 1 : shaped.
        assert_eq!(r.read_var_i32().unwrap(), ENTRY_SHAPED);
        assert_eq!(r.read_string().unwrap(), "1"); // recipeId = net_id
        assert_eq!(r.read_var_i32().unwrap(), 2); // width
        assert_eq!(r.read_var_i32().unwrap(), 2); // height
                                                  // Ingredient[0] : AnyMeta → INT_ID_META, id=10, meta=wildcard.
        assert_eq!(r.read_u8().unwrap(), DESC_INT_ID_META);
        assert_eq!(r.read_i16_le().unwrap(), 10);
        assert_eq!(r.read_i16_le().unwrap(), RECIPE_INPUT_WILDCARD_META);
        assert_eq!(r.read_var_i32().unwrap(), 1); // count
                                                  // Ingredient[1] : Empty → null descriptor + count 0.
        assert_eq!(r.read_u8().unwrap(), 0);
        assert_eq!(r.read_var_i32().unwrap(), 0);
        // Ingredient[2] : Exact → INT_ID_META, id=10, meta=0.
        assert_eq!(r.read_u8().unwrap(), DESC_INT_ID_META);
        assert_eq!(r.read_i16_le().unwrap(), 10);
        assert_eq!(r.read_i16_le().unwrap(), 0);
        assert_eq!(r.read_var_i32().unwrap(), 1);
        // Ingredient[3] : Tag → DESC_TAG + nom.
        assert_eq!(r.read_u8().unwrap(), DESC_TAG);
        assert_eq!(r.read_string().unwrap(), "minecraft:planks");
        assert_eq!(r.read_var_i32().unwrap(), 1);
        // Output : 1 item stack (id=58, count=1).
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(r.read_var_i32().unwrap(), 58); // id
        assert_eq!(r.read_u16_le().unwrap(), 1); // count
        assert_eq!(r.read_var_u32().unwrap(), 0); // meta
        assert_eq!(r.read_var_i32().unwrap(), 0); // block_runtime_id
        assert_eq!(r.read_byte_array().unwrap().len(), 10); // extra data
                                                            // UUID nil (16 octets).
        for _ in 0..16 {
            assert_eq!(r.read_u8().unwrap(), 0);
        }
        assert_eq!(r.read_string().unwrap(), "crafting_table");
        assert_eq!(r.read_var_i32().unwrap(), RECIPE_PRIORITY);
        assert!(r.read_bool().unwrap()); // symmetric
        assert!(r.read_bool().unwrap()); // unlocking requirement = null
        assert_eq!(r.read_var_u32().unwrap(), 1); // recipeNetId

        // Recette 2 : shapeless (on saute les détails, on vérifie juste le type
        // et le net id).
        assert_eq!(r.read_var_i32().unwrap(), ENTRY_SHAPELESS);
        assert_eq!(r.read_string().unwrap(), "2");
        assert_eq!(r.read_var_u32().unwrap(), 1); // 1 ingredient
        assert_eq!(r.read_u8().unwrap(), DESC_INT_ID_META);
        assert_eq!(r.read_i16_le().unwrap(), 5);
        assert_eq!(r.read_i16_le().unwrap(), RECIPE_INPUT_WILDCARD_META);
        assert_eq!(r.read_var_i32().unwrap(), 1);
        assert_eq!(r.read_var_u32().unwrap(), 1); // 1 output
        assert_eq!(r.read_var_i32().unwrap(), 280);
        assert_eq!(r.read_u16_le().unwrap(), 4);
        assert_eq!(r.read_var_u32().unwrap(), 0);
        assert_eq!(r.read_var_i32().unwrap(), 0);
        assert_eq!(r.read_byte_array().unwrap().len(), 10);
        for _ in 0..16 {
            assert_eq!(r.read_u8().unwrap(), 0);
        }
        assert_eq!(r.read_string().unwrap(), "crafting_table");
        assert_eq!(r.read_var_i32().unwrap(), RECIPE_PRIORITY);
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_var_u32().unwrap(), 2);

        // Recette 3 : furnace encodée en shapeless avec block "furnace".
        assert_eq!(r.read_var_i32().unwrap(), ENTRY_SHAPELESS);
        assert_eq!(r.read_string().unwrap(), "3");
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(r.read_u8().unwrap(), DESC_INT_ID_META);
        assert_eq!(r.read_i16_le().unwrap(), 15);
        assert_eq!(r.read_i16_le().unwrap(), RECIPE_INPUT_WILDCARD_META);
        assert_eq!(r.read_var_i32().unwrap(), 1);
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(r.read_var_i32().unwrap(), 265);
        assert_eq!(r.read_u16_le().unwrap(), 1);
        assert_eq!(r.read_var_u32().unwrap(), 0);
        assert_eq!(r.read_var_i32().unwrap(), 0);
        assert_eq!(r.read_byte_array().unwrap().len(), 10);
        for _ in 0..16 {
            assert_eq!(r.read_u8().unwrap(), 0);
        }
        assert_eq!(r.read_string().unwrap(), "furnace");
        assert_eq!(r.read_var_i32().unwrap(), RECIPE_PRIORITY);
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_var_u32().unwrap(), 3);

        // Trailers : potion type / container / material reducer = 0, clean=true.
        assert_eq!(r.read_var_u32().unwrap(), 0);
        assert_eq!(r.read_var_u32().unwrap(), 0);
        assert_eq!(r.read_var_u32().unwrap(), 0);
        assert!(r.read_bool().unwrap());
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
