//! InventorySlot (0x32) — Server → Client.
//!
//! Updates a single slot in a container.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::item_stack::ItemStack;
use crate::types::VarUInt32;

/// Update a single container slot.
pub struct InventorySlot {
    /// Container window ID.
    pub window_id: u32,
    /// Slot index within the container.
    pub slot: u32,
    /// The item to set in the slot.
    pub item: ItemStack,
}

impl ProtoEncode for InventorySlot {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.window_id).proto_encode(buf);
        VarUInt32(self.slot).proto_encode(buf);
        // FullContainerName
        buf.put_u8(0); // container_id
        VarUInt32(0).proto_encode(buf); // dynamic_container_id
        self.item.proto_encode(buf);
    }
}
