//! ItemStackResponse (0x94) — Server → Client.
//!
//! Sent by the server in response to ItemStackRequest to confirm or reject
//! inventory operations.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarInt;

/// Response for a single slot after an inventory operation.
pub struct StackResponseSlot {
    /// Slot index.
    pub slot: u8,
    /// Hotbar slot (same as slot for hotbar items).
    pub hotbar_slot: u8,
    /// Resulting item count.
    pub count: u8,
    /// New stack network ID assigned by the server.
    pub stack_network_id: i32,
    /// Custom name (empty string if none).
    pub custom_name: String,
    /// Durability correction (0 if no change).
    pub durability_correction: i32,
}

impl ProtoEncode for StackResponseSlot {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.slot);
        buf.put_u8(self.hotbar_slot);
        buf.put_u8(self.count);
        VarInt(self.stack_network_id).proto_encode(buf);
        write_string(buf, &self.custom_name);
        VarInt(self.durability_correction).proto_encode(buf);
    }
}

/// Container slot updates grouped by container.
pub struct StackResponseContainer {
    /// Container ID (0 = inventory, 119 = armor, etc.).
    pub container_id: u8,
    /// Updated slots in this container.
    pub slots: Vec<StackResponseSlot>,
}

impl ProtoEncode for StackResponseContainer {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        // FullContainerName
        buf.put_u8(self.container_id);
        crate::types::VarUInt32(0).proto_encode(buf); // dynamic_container_id = 0
                                                      // Slots
        crate::types::VarUInt32(self.slots.len() as u32).proto_encode(buf);
        for slot in &self.slots {
            slot.proto_encode(buf);
        }
    }
}

/// Response for a single request.
pub struct StackResponseEntry {
    /// 0 = success, non-zero = error.
    pub status: u8,
    /// Must match the request_id from the ItemStackRequest.
    pub request_id: i32,
    /// Container slot updates (empty if status != 0).
    pub containers: Vec<StackResponseContainer>,
}

impl ProtoEncode for StackResponseEntry {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.status);
        VarInt(self.request_id).proto_encode(buf);
        if self.status == 0 {
            crate::types::VarUInt32(self.containers.len() as u32).proto_encode(buf);
            for container in &self.containers {
                container.proto_encode(buf);
            }
        }
    }
}

/// The complete ItemStackResponse packet.
pub struct ItemStackResponse {
    pub responses: Vec<StackResponseEntry>,
}

impl ProtoEncode for ItemStackResponse {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        crate::types::VarUInt32(self.responses.len() as u32).proto_encode(buf);
        for response in &self.responses {
            response.proto_encode(buf);
        }
    }
}
