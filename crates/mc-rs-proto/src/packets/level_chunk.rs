//! LevelChunk (0x3A) — Server → Client.

use bytes::{BufMut, Bytes};

use crate::codec::ProtoEncode;
use crate::types::{VarInt, VarUInt32};

/// Sends a full chunk column to the client.
#[derive(Debug, Clone)]
pub struct LevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub dimension_id: i32,
    pub sub_chunk_count: u32,
    pub cache_enabled: bool,
    /// Pre-serialized payload: SubChunks[] + BiomeData + BorderBlocks.
    pub payload: Bytes,
}

impl ProtoEncode for LevelChunk {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.chunk_x).proto_encode(buf);
        VarInt(self.chunk_z).proto_encode(buf);
        VarInt(self.dimension_id).proto_encode(buf);
        VarUInt32(self.sub_chunk_count).proto_encode(buf);
        buf.put_u8(self.cache_enabled as u8);
        // Payload as string: VarUInt32(length) + bytes (protocol 924+, per PMMP LevelChunkPacket)
        VarUInt32(self.payload.len() as u32).proto_encode(buf);
        buf.put_slice(&self.payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_level_chunk() {
        let pkt = LevelChunk {
            chunk_x: 0,
            chunk_z: 0,
            dimension_id: 0,
            sub_chunk_count: 24,
            cache_enabled: false,
            payload: Bytes::from_static(&[0x09, 0x01]),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(0) = 0x00, VarInt(0) = 0x00, VarInt(0) = 0x00
        // VarUInt32(24) = 24, cache=0x00, VarUInt32(2)=payload_len, payload=[0x09, 0x01]
        assert_eq!(buf[0], 0x00); // chunk_x
        assert_eq!(buf[1], 0x00); // chunk_z
        assert_eq!(buf[2], 0x00); // dimension_id
        assert_eq!(buf[3], 24); // sub_chunk_count
        assert_eq!(buf[4], 0x00); // cache_enabled
        assert_eq!(buf[5], 0x02); // payload length (VarUInt32 = 2)
        assert_eq!(buf[6], 0x09); // payload start
        assert_eq!(buf[7], 0x01);
    }

    #[test]
    fn cache_disabled_is_zero() {
        let pkt = LevelChunk {
            chunk_x: 1,
            chunk_z: -1,
            dimension_id: 0,
            sub_chunk_count: 1,
            cache_enabled: false,
            payload: Bytes::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Find cache byte — after chunk_x, chunk_z, dimension_id, sub_chunk_count
        // VarInt(1) = 0x02 (zigzag), VarInt(-1) = 0x01 (zigzag), VarInt(0) = 0x00
        // VarUInt32(1) = 0x01
        assert_eq!(buf[4], 0x00, "cache_enabled should be 0");
    }
}
