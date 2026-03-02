//! RequestChunkRadius (0x45) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::VarInt;

/// The client requests a specific chunk render distance.
#[derive(Debug, Clone)]
pub struct RequestChunkRadius {
    pub chunk_radius: i32,
    pub max_chunk_radius: u8,
}

impl ProtoDecode for RequestChunkRadius {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let chunk_radius = VarInt::proto_decode(buf)?.0;
        if !buf.has_remaining() {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: 0,
            });
        }
        let max_chunk_radius = buf.get_u8();
        Ok(Self {
            chunk_radius,
            max_chunk_radius,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoEncode;
    use bytes::{Buf, BytesMut};

    #[test]
    fn decode_radius() {
        let mut buf = BytesMut::new();
        VarInt(8).proto_encode(&mut buf);
        buf.extend_from_slice(&[16]);
        let pkt = RequestChunkRadius::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.chunk_radius, 8);
        assert_eq!(pkt.max_chunk_radius, 16);
    }

    #[test]
    fn decode_does_not_consume_extra_bytes() {
        let mut buf = BytesMut::new();
        VarInt(6).proto_encode(&mut buf);
        buf.extend_from_slice(&[20, 0xAB]);

        let mut frozen = buf.freeze();
        let pkt = RequestChunkRadius::proto_decode(&mut frozen).unwrap();
        assert_eq!(pkt.chunk_radius, 6);
        assert_eq!(pkt.max_chunk_radius, 20);
        assert_eq!(frozen.remaining(), 1);
        assert_eq!(frozen.get_u8(), 0xAB);
    }
}
