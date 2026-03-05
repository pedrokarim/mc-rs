//! UpdateBlock (0x15) — Server → Client.
//!
//! Sent when a single block changes in the world.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{BlockPos, VarUInt32};

/// UpdateBlock packet.
#[derive(Debug, Clone)]
pub struct UpdateBlock {
    pub position: BlockPos,
    pub runtime_id: u32,
    pub flags: u32,
    pub layer: u32,
}

/// Flags: Neighbours (0x01) + Network (0x02).
pub const UPDATE_BLOCK_FLAGS_DEFAULT: u32 = 0x03;

impl UpdateBlock {
    /// Create an UpdateBlock for the default layer with standard flags.
    pub fn new(position: BlockPos, runtime_id: u32) -> Self {
        Self {
            position,
            runtime_id,
            flags: UPDATE_BLOCK_FLAGS_DEFAULT,
            layer: 0,
        }
    }
}

impl ProtoEncode for UpdateBlock {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.position.proto_encode(buf);
        VarUInt32(self.runtime_id).proto_encode(buf);
        VarUInt32(self.flags).proto_encode(buf);
        VarUInt32(self.layer).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoDecode;
    use bytes::BytesMut;

    #[test]
    fn encode_update_block() {
        let pkt = UpdateBlock::new(BlockPos::new(10, 64, -5), 42);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Position (VarInt x + VarUInt32 y + VarInt z) + runtime_id + flags + layer
        // Should produce non-trivial data
        assert!(buf.len() > 4);
        // Verify we can read back the BlockPos at the start
        let decoded_pos = BlockPos::proto_decode(&mut buf.clone().freeze()).unwrap();
        assert_eq!(decoded_pos, BlockPos::new(10, 64, -5));
    }
}
