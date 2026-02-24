//! NetworkChunkPublisherUpdate (0x7A) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{BlockPos, VarUInt32};

/// Tells the client the zone of available chunks.
#[derive(Debug, Clone)]
pub struct NetworkChunkPublisherUpdate {
    pub position: BlockPos,
    pub radius: u32,
}

impl ProtoEncode for NetworkChunkPublisherUpdate {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.position.proto_encode(buf);
        VarUInt32(self.radius).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_at_spawn() {
        let pkt = NetworkChunkPublisherUpdate {
            position: BlockPos::new(0, 64, 0),
            radius: 128,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
    }
}
