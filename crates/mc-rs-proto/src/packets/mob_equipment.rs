//! MobEquipment (0x1F) — Bidirectional.
//!
//! Sent by the client when the player changes hotbar slot, or by the server
//! to show what another player is holding.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::item_stack::ItemStack;
use crate::types::VarUInt64;

/// Equipment change for a mob/player.
pub struct MobEquipment {
    /// Runtime ID of the entity whose equipment changed.
    pub entity_runtime_id: u64,
    /// The item being held.
    pub item: ItemStack,
    /// Inventory slot the item is from.
    pub inventory_slot: u8,
    /// Hotbar slot (0-8).
    pub hotbar_slot: u8,
    /// Container window ID (0 = inventory).
    pub window_id: u8,
}

impl ProtoEncode for MobEquipment {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        self.item.proto_encode(buf);
        buf.put_u8(self.inventory_slot);
        buf.put_u8(self.hotbar_slot);
        buf.put_u8(self.window_id);
    }
}

impl ProtoDecode for MobEquipment {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let entity_runtime_id = VarUInt64::proto_decode(buf)?.0;
        let item = ItemStack::proto_decode(buf)?;
        if buf.remaining() < 3 {
            return Err(ProtoError::BufferTooShort {
                needed: 3,
                remaining: buf.remaining(),
            });
        }
        let inventory_slot = buf.get_u8();
        let hotbar_slot = buf.get_u8();
        let window_id = buf.get_u8();
        Ok(Self {
            entity_runtime_id,
            item,
            inventory_slot,
            hotbar_slot,
            window_id,
        })
    }
}
