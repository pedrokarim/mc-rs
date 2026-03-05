//! ContainerOpen (0x2E) — Server → Client.
//!
//! Opens a container window (chest, furnace, etc.) on the client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{BlockPos, VarLong};

/// Opens a container window for the player.
#[derive(Debug, Clone)]
pub struct ContainerOpen {
    /// Unique window ID for this container session.
    pub window_id: u8,
    /// Container type (0=inventory, 2=chest, etc.).
    pub container_type: u8,
    /// Position of the container block.
    pub position: BlockPos,
    /// Entity unique ID (-1 for block containers).
    pub entity_unique_id: i64,
}

impl ProtoEncode for ContainerOpen {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.window_id);
        buf.put_u8(self.container_type);
        self.position.proto_encode(buf);
        VarLong(self.entity_unique_id).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_container_open() {
        let pkt = ContainerOpen {
            window_id: 1,
            container_type: 2,
            position: BlockPos::new(10, 64, -5),
            entity_unique_id: -1,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // window_id(1) + container_type(1) + BlockPos(3 VarInts) + VarLong(-1)
        assert!(buf.len() >= 5);
        assert_eq!(buf[0], 1); // window_id
        assert_eq!(buf[1], 2); // container_type
    }
}
