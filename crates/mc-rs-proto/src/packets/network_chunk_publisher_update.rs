//! NetworkChunkPublisherUpdate (0x79) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{ChunkPos, SignedBlockPos, VarUInt32};

/// Tells the client the zone of available chunks.
#[derive(Debug, Clone)]
pub struct NetworkChunkPublisherUpdate {
    pub position: SignedBlockPos,
    pub radius: u32,
    pub saved_chunks: Vec<ChunkPos>,
}

impl ProtoEncode for NetworkChunkPublisherUpdate {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.position.proto_encode(buf);
        VarUInt32(self.radius).proto_encode(buf);
        buf.put_u32_le(self.saved_chunks.len() as u32);
        for chunk in &self.saved_chunks {
            chunk.proto_encode(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoDecode;
    use bytes::BytesMut;

    #[test]
    fn encode_includes_empty_saved_chunk_count() {
        let pkt = NetworkChunkPublisherUpdate {
            position: SignedBlockPos::new(0, 64, 0),
            radius: 128,
            saved_chunks: Vec::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 9);
        assert_eq!(&buf[buf.len() - 4..], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn encode_signed_y_position() {
        let pkt = NetworkChunkPublisherUpdate {
            position: SignedBlockPos::new(0, -64, 0),
            radius: 128,
            saved_chunks: Vec::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);

        let mut cursor = buf.freeze();
        let decoded_pos = SignedBlockPos::proto_decode(&mut cursor).unwrap();
        assert_eq!(decoded_pos.y, -64);
    }
}
