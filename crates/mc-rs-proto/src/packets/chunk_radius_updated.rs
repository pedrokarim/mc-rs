//! ChunkRadiusUpdated (0x46) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarInt;

/// Tells the client the accepted chunk render distance.
#[derive(Debug, Clone)]
pub struct ChunkRadiusUpdated {
    pub chunk_radius: i32,
}

impl ProtoEncode for ChunkRadiusUpdated {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.chunk_radius).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoDecode;
    use bytes::BytesMut;

    #[test]
    fn encode_radius() {
        let pkt = ChunkRadiusUpdated { chunk_radius: 8 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = VarInt::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(decoded.0, 8);
    }
}
