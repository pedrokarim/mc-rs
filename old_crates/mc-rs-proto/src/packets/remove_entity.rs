//! RemoveEntity (0x0E) — Server → Client.
//!
//! Despawns an entity from the client's world.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarLong;

/// Remove an entity from the world.
pub struct RemoveEntity {
    /// Unique entity ID to remove.
    pub entity_unique_id: i64,
}

impl ProtoEncode for RemoveEntity {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarLong(self.entity_unique_id).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_remove_entity() {
        let pkt = RemoveEntity {
            entity_unique_id: 42,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarLong(42) zigzag = 84 = 0x54 → 1 byte
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x54);
    }
}
