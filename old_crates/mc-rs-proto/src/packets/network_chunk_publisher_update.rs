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
        // PocketMine writes this as LE u32, not VarUInt.
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
    use bytes::{Buf, BytesMut};

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
        assert_eq!(
            &buf[buf.len() - 4..],
            &[0x00, 0x00, 0x00, 0x00],
            "empty saved_chunks count = LE u32(0)"
        );
    }

    #[test]
    fn encode_saved_chunk_count_is_le_u32() {
        let pkt = NetworkChunkPublisherUpdate {
            position: SignedBlockPos::new(0, 64, 0),
            radius: 128,
            saved_chunks: vec![ChunkPos::new(1, -1)],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);

        let mut cursor = buf.freeze();
        let _ = SignedBlockPos::proto_decode(&mut cursor).unwrap();
        let _ = VarUInt32::proto_decode(&mut cursor).unwrap();
        let count = cursor.get_u32_le();
        assert_eq!(count, 1);
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
