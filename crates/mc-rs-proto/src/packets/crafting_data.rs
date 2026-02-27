//! CraftingData (0x34) — Server → Client.
//!
//! Sends all crafting recipes the client should know about. Must be sent
//! during login flow (after StartGame) so the client populates its recipe book.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarInt, VarUInt32};

/// An ingredient in a crafting recipe (for the CraftingData wire format).
#[derive(Debug, Clone)]
pub struct RecipeIngredient {
    /// Item network ID (runtime ID). 0 = air/empty.
    pub network_id: i16,
    /// Metadata filter. 0x7FFF = accept any variant.
    pub metadata: i16,
    /// Required count.
    pub count: i32,
}

impl ProtoEncode for RecipeIngredient {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        if self.network_id == 0 {
            buf.put_u8(0); // type = invalid (air)
        } else {
            buf.put_u8(1); // type = default
            buf.put_i16_le(self.network_id);
            buf.put_i16_le(self.metadata);
            VarInt(self.count).proto_encode(buf);
        }
    }
}

/// A shaped recipe entry for the CraftingData packet.
#[derive(Debug, Clone)]
pub struct ShapedRecipeEntry {
    /// Unique recipe identifier string.
    pub recipe_id: String,
    /// Grid width (1-3).
    pub width: i32,
    /// Grid height (1-3).
    pub height: i32,
    /// Input grid (width × height).
    pub input: Vec<RecipeIngredient>,
    /// Output item(s).
    pub output: Vec<CraftingOutputItem>,
    /// UUID for this recipe (16 bytes).
    pub uuid: [u8; 16],
    /// Block tag, e.g. "crafting_table".
    pub tag: String,
    /// Network ID (unique per recipe, referenced by CraftRecipe action).
    pub network_id: u32,
}

/// A shapeless recipe entry for the CraftingData packet.
#[derive(Debug, Clone)]
pub struct ShapelessRecipeEntry {
    /// Unique recipe identifier string.
    pub recipe_id: String,
    /// Input ingredients.
    pub input: Vec<RecipeIngredient>,
    /// Output item(s).
    pub output: Vec<CraftingOutputItem>,
    /// UUID for this recipe.
    pub uuid: [u8; 16],
    /// Block tag.
    pub tag: String,
    /// Network ID.
    pub network_id: u32,
}

/// An output item in a crafting recipe — simplified ItemStack for CraftingData.
#[derive(Debug, Clone)]
pub struct CraftingOutputItem {
    /// Item network ID (runtime ID).
    pub network_id: i32,
    /// Count of items produced.
    pub count: u16,
    /// Metadata / damage value.
    pub metadata: u16,
    /// Block runtime ID (0 if not a block).
    pub block_runtime_id: i32,
}

impl ProtoEncode for CraftingOutputItem {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        // NetworkItemStackDescriptor (simplified, no NBT / no stack ID)
        VarInt(self.network_id).proto_encode(buf);
        if self.network_id == 0 {
            return;
        }
        buf.put_u16_le(self.count);
        VarUInt32(self.metadata as u32).proto_encode(buf);
        // has_stack_id = false
        buf.put_u8(0);
        // block_runtime_id
        VarInt(self.block_runtime_id).proto_encode(buf);
        // user data = none
        VarUInt32(0).proto_encode(buf);
        // can_place_on = 0
        VarInt(0).proto_encode(buf);
        // can_destroy = 0
        VarInt(0).proto_encode(buf);
    }
}

/// A furnace/smelting recipe entry (type 3 = FurnaceDataRecipe).
#[derive(Debug, Clone)]
pub struct FurnaceRecipeEntry {
    /// Input item numeric ID.
    pub input_id: i32,
    /// Input item metadata (0x7FFF = any variant).
    pub input_metadata: i32,
    /// Output item.
    pub output: CraftingOutputItem,
    /// Block tag: "furnace", "blast_furnace", or "smoker".
    pub tag: String,
}

/// The CraftingData packet containing all recipe definitions.
pub struct CraftingData {
    pub shaped: Vec<ShapedRecipeEntry>,
    pub shapeless: Vec<ShapelessRecipeEntry>,
    pub furnace: Vec<FurnaceRecipeEntry>,
    /// Whether to clear existing recipes first.
    pub clear_recipes: bool,
}

impl ProtoEncode for CraftingData {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        let total = self.shaped.len() + self.shapeless.len() + self.furnace.len();
        VarUInt32(total as u32).proto_encode(buf);

        // Shapeless recipes (type = 0)
        for recipe in &self.shapeless {
            VarInt(0).proto_encode(buf); // recipe type: shapeless
            encode_shapeless(buf, recipe);
        }

        // Shaped recipes (type = 1)
        for recipe in &self.shaped {
            VarInt(1).proto_encode(buf); // recipe type: shaped
            encode_shaped(buf, recipe);
        }

        // Furnace data recipes (type = 3)
        for recipe in &self.furnace {
            VarInt(3).proto_encode(buf); // recipe type: furnace_data
            encode_furnace(buf, recipe);
        }

        // Potion mixes count = 0
        VarUInt32(0).proto_encode(buf);
        // Container mixes count = 0
        VarUInt32(0).proto_encode(buf);
        // Material reducers count = 0
        VarUInt32(0).proto_encode(buf);

        // clear_recipes flag
        buf.put_u8(if self.clear_recipes { 1 } else { 0 });
    }
}

fn write_string_raw(buf: &mut impl BufMut, s: &str) {
    VarUInt32(s.len() as u32).proto_encode(buf);
    buf.put_slice(s.as_bytes());
}

fn encode_shapeless(buf: &mut impl BufMut, recipe: &ShapelessRecipeEntry) {
    write_string_raw(buf, &recipe.recipe_id);
    // input count
    VarUInt32(recipe.input.len() as u32).proto_encode(buf);
    for ing in &recipe.input {
        ing.proto_encode(buf);
    }
    // output count
    VarUInt32(recipe.output.len() as u32).proto_encode(buf);
    for out in &recipe.output {
        out.proto_encode(buf);
    }
    // UUID (16 bytes)
    buf.put_slice(&recipe.uuid);
    // block tag
    write_string_raw(buf, &recipe.tag);
    // priority
    VarInt(0).proto_encode(buf);
    // assume_symmetry
    buf.put_u8(0);
    // network_id
    VarUInt32(recipe.network_id).proto_encode(buf);
}

fn encode_shaped(buf: &mut impl BufMut, recipe: &ShapedRecipeEntry) {
    write_string_raw(buf, &recipe.recipe_id);
    // width, height
    VarInt(recipe.width).proto_encode(buf);
    VarInt(recipe.height).proto_encode(buf);
    // input grid (width × height ingredients — no count prefix, it's implicit)
    for ing in &recipe.input {
        ing.proto_encode(buf);
    }
    // output count
    VarUInt32(recipe.output.len() as u32).proto_encode(buf);
    for out in &recipe.output {
        out.proto_encode(buf);
    }
    // UUID (16 bytes)
    buf.put_slice(&recipe.uuid);
    // block tag
    write_string_raw(buf, &recipe.tag);
    // priority
    VarInt(0).proto_encode(buf);
    // assume_symmetry
    buf.put_u8(0);
    // unlock requirement (VarUInt32 = 0 means context)
    VarUInt32(0).proto_encode(buf);
    // network_id
    VarUInt32(recipe.network_id).proto_encode(buf);
}

fn encode_furnace(buf: &mut impl BufMut, recipe: &FurnaceRecipeEntry) {
    VarInt(recipe.input_id).proto_encode(buf);
    VarInt(recipe.input_metadata).proto_encode(buf);
    recipe.output.proto_encode(buf);
    write_string_raw(buf, &recipe.tag);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty_crafting_data() {
        let pkt = CraftingData {
            shaped: Vec::new(),
            shapeless: Vec::new(),
            furnace: Vec::new(),
            clear_recipes: true,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt32(0) + 3 × VarUInt32(0) + u8(1)
        assert!(buf.len() >= 5);
    }

    #[test]
    fn encode_air_ingredient() {
        let ing = RecipeIngredient {
            network_id: 0,
            metadata: 0,
            count: 0,
        };
        let mut buf = BytesMut::new();
        ing.proto_encode(&mut buf);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn encode_valid_ingredient() {
        let ing = RecipeIngredient {
            network_id: 1,
            metadata: 0,
            count: 1,
        };
        let mut buf = BytesMut::new();
        ing.proto_encode(&mut buf);
        // u8(1) + i16_le(1) + i16_le(0) + VarInt(1)
        assert_eq!(buf.len(), 6);
        assert_eq!(buf[0], 1); // type = default
    }

    #[test]
    fn encode_output_item() {
        let out = CraftingOutputItem {
            network_id: 5,
            count: 4,
            metadata: 0,
            block_runtime_id: 0,
        };
        let mut buf = BytesMut::new();
        out.proto_encode(&mut buf);
        assert!(buf.len() > 5);
    }

    #[test]
    fn encode_shaped_entry() {
        let entry = ShapedRecipeEntry {
            recipe_id: "test:shaped".to_string(),
            width: 2,
            height: 1,
            input: vec![
                RecipeIngredient {
                    network_id: 1,
                    metadata: 0,
                    count: 1,
                },
                RecipeIngredient {
                    network_id: 1,
                    metadata: 0,
                    count: 1,
                },
            ],
            output: vec![CraftingOutputItem {
                network_id: 5,
                count: 4,
                metadata: 0,
                block_runtime_id: 0,
            }],
            uuid: [0u8; 16],
            tag: "crafting_table".to_string(),
            network_id: 1,
        };
        let pkt = CraftingData {
            shaped: vec![entry],
            shapeless: Vec::new(),
            furnace: Vec::new(),
            clear_recipes: true,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 30);
    }

    #[test]
    fn encode_furnace_recipe() {
        let entry = FurnaceRecipeEntry {
            input_id: 10,
            input_metadata: 0,
            output: CraftingOutputItem {
                network_id: 265,
                count: 1,
                metadata: 0,
                block_runtime_id: 0,
            },
            tag: "furnace".to_string(),
        };
        let pkt = CraftingData {
            shaped: Vec::new(),
            shapeless: Vec::new(),
            furnace: vec![entry],
            clear_recipes: true,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should have: VarUInt32(1) + VarInt(3) + furnace data + trailing counts + flag
        assert!(buf.len() > 10);
    }
}
