//! AddItemEntity (0x0F) — Server → Client.
//!
//! Spawns a dropped item entity in the world.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::item_stack::ItemStack;
use crate::types::{VarLong, VarUInt32, VarUInt64, Vec3};

/// Spawn a dropped item entity.
pub struct AddItemEntity {
    /// Unique entity ID.
    pub entity_unique_id: i64,
    /// Runtime entity ID.
    pub entity_runtime_id: u64,
    /// The item stack.
    pub item: ItemStack,
    /// World position.
    pub position: Vec3,
    /// Velocity vector.
    pub velocity: Vec3,
    /// Whether this item came from fishing.
    pub is_from_fishing: bool,
}

impl ProtoEncode for AddItemEntity {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarLong(self.entity_unique_id).proto_encode(buf);
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        self.item.proto_encode(buf);
        self.position.proto_encode(buf);
        self.velocity.proto_encode(buf);
        // Entity metadata (empty — VarUInt32 count = 0, then end marker)
        VarUInt32(0).proto_encode(buf);
        buf.put_u8(self.is_from_fishing as u8);
    }
}
