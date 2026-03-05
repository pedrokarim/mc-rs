//! InventoryContent (0x31) — Server → Client.
//!
//! Sends the full contents of a container (inventory window) to the client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::item_stack::ItemStack;
use crate::types::VarUInt32;

/// Full contents of a container window.
pub struct InventoryContent {
    /// Container window ID: 0 = inventory, 119 = armor, 120 = creative output, 124 = offhand.
    pub window_id: u32,
    /// All item slots in the container.
    pub items: Vec<ItemStack>,
}

impl ProtoEncode for InventoryContent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.window_id).proto_encode(buf);
        VarUInt32(self.items.len() as u32).proto_encode(buf);
        for item in &self.items {
            item.proto_encode(buf);
            // FullContainerName: container_id (u8) + dynamic_container_id (VarUInt32 = 0)
            buf.put_u8(0); // container_id = 0
            VarUInt32(0).proto_encode(buf); // dynamic_container_id = 0
        }
    }
}
