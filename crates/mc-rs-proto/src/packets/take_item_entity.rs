//! TakeItemEntity (0x11) — Server → Client.
//!
//! Sent when a player picks up a dropped item entity.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarUInt64;

/// Animate item pickup.
pub struct TakeItemEntity {
    /// Runtime ID of the item entity being picked up.
    pub item_runtime_id: u64,
    /// Runtime ID of the player picking up the item.
    pub player_runtime_id: u64,
}

impl ProtoEncode for TakeItemEntity {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.item_runtime_id).proto_encode(buf);
        VarUInt64(self.player_runtime_id).proto_encode(buf);
    }
}
