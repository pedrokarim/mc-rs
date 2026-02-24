//! CreativeContent (0x91) — Server → Client.
//!
//! Sends the list of items available in the creative inventory menu.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::item_stack::ItemStack;
use crate::types::VarUInt32;

/// A single item in the creative inventory.
pub struct CreativeContentItem {
    /// Sequential network ID (1-based). Used by CraftCreative action.
    pub network_id: u32,
    /// The item stack (runtime_id, count=1 typically, metadata, etc.).
    pub item: ItemStack,
}

/// Sends the list of items available in creative mode.
#[derive(Default)]
pub struct CreativeContent {
    pub items: Vec<CreativeContentItem>,
}

impl ProtoEncode for CreativeContent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.items.len() as u32).proto_encode(buf);
        for item in &self.items {
            VarUInt32(item.network_id).proto_encode(buf);
            item.item.proto_encode(buf);
        }
    }
}

/// Generate creative content items from a list of item names and their runtime IDs.
///
/// Each item gets a sequential network_id starting from 1.
pub fn build_creative_content(items: &[(i32, u16)]) -> CreativeContent {
    let mut creative_items = Vec::with_capacity(items.len());
    for (idx, &(runtime_id, count)) in items.iter().enumerate() {
        creative_items.push(CreativeContentItem {
            network_id: (idx + 1) as u32,
            item: ItemStack::new(runtime_id, count),
        });
    }
    CreativeContent {
        items: creative_items,
    }
}

/// Common creative items — a curated subset of vanilla items for the creative menu.
///
/// Returns (runtime_id, count) pairs.
pub fn default_creative_items() -> Vec<(i32, u16)> {
    vec![
        // === Building blocks ===
        (1, 1),  // stone
        (4, 1),  // cobblestone
        (3, 1),  // dirt
        (2, 1),  // grass block (id 2 in legacy mapping)
        (12, 1), // sand
        (13, 1), // gravel
        (45, 1), // brick_block
        (98, 1), // stonebrick (smooth stone bricks, legacy)
        (24, 1), // sandstone
        (5, 1),  // planks (oak)
        (17, 1), // log (oak)
        (7, 1),  // bedrock
        (49, 1), // obsidian
        (89, 1), // glowstone
        (20, 1), // glass
        (47, 1), // bookshelf
        // === Ores & minerals ===
        (14, 1),  // gold_ore
        (15, 1),  // iron_ore
        (16, 1),  // coal_ore
        (56, 1),  // diamond_ore
        (21, 1),  // lapis_ore
        (73, 1),  // redstone_ore
        (129, 1), // emerald_ore
        // === Mineral blocks ===
        (42, 1),  // iron_block
        (41, 1),  // gold_block
        (57, 1),  // diamond_block
        (133, 1), // emerald_block
        (22, 1),  // lapis_block
        (152, 1), // redstone_block
        (173, 1), // coal_block
        // === Tools ===
        (347, 1), // diamond_sword
        (349, 1), // diamond_pickaxe
        (348, 1), // diamond_shovel
        (350, 1), // diamond_axe
        (364, 1), // diamond_hoe
        (342, 1), // iron_sword
        (344, 1), // iron_pickaxe
        (343, 1), // iron_shovel
        (345, 1), // iron_axe
        (359, 1), // iron_hoe
        (337, 1), // stone_sword
        (339, 1), // stone_pickaxe
        (338, 1), // stone_shovel
        (340, 1), // stone_axe
        (354, 1), // stone_hoe
        (336, 1), // wooden_sword
        (271, 1), // wooden_pickaxe (uncertain ID)
        // === Armor ===
        (379, 1), // diamond_helmet
        (380, 1), // diamond_chestplate
        (381, 1), // diamond_leggings
        (382, 1), // diamond_boots
        (375, 1), // iron_helmet
        (376, 1), // iron_chestplate
        (377, 1), // iron_leggings
        (378, 1), // iron_boots
        // === Food ===
        (285, 1), // apple
        (290, 1), // bread
        (303, 1), // cooked_beef
        (305, 1), // cooked_chicken
        (297, 1), // cooked_cod
        (310, 1), // baked_potato
        (300, 1), // cookie
        // === Materials ===
        (335, 1), // diamond
        (266, 1), // gold_ingot (approximate)
        (265, 1), // iron_ingot (approximate)
        (333, 1), // coal
        (331, 1), // bow
        (332, 1), // arrow
        (287, 1), // string (approximate)
        // === Redstone ===
        (76, 1), // redstone_torch
        (55, 1), // redstone_wire
        (69, 1), // lever
        (77, 1), // stone_button
        (33, 1), // piston
        // === Misc ===
        (54, 1),  // chest
        (58, 1),  // crafting_table
        (61, 1),  // furnace
        (50, 1),  // torch
        (65, 1),  // ladder
        (323, 1), // sign (oak) (approximate)
        (392, 1), // bucket
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty() {
        let pkt = CreativeContent::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x00);
    }

    #[test]
    fn encode_with_items() {
        let items = default_creative_items();
        let pkt = build_creative_content(&items);
        assert!(!pkt.items.is_empty());
        assert_eq!(pkt.items[0].network_id, 1);

        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 1);
    }

    #[test]
    fn network_ids_sequential() {
        let items = default_creative_items();
        let pkt = build_creative_content(&items);
        for (i, item) in pkt.items.iter().enumerate() {
            assert_eq!(item.network_id, (i + 1) as u32);
        }
    }
}
