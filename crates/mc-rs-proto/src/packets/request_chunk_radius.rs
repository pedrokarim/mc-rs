//! RequestChunkRadius (0x45) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::VarInt;

/// The client requests a specific chunk render distance.
#[derive(Debug, Clone)]
pub struct RequestChunkRadius {
    pub chunk_radius: i32,
    pub max_chunk_radius: i32,
}

impl ProtoDecode for RequestChunkRadius {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let chunk_radius = VarInt::proto_decode(buf)?.0;
        let max_chunk_radius = VarInt::proto_decode(buf)?.0;
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
    use bytes::BytesMut;

    #[test]
    fn decode_radius() {
        let mut buf = BytesMut::new();
        VarInt(8).proto_encode(&mut buf);
        VarInt(16).proto_encode(&mut buf);
        let pkt = RequestChunkRadius::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.chunk_radius, 8);
        assert_eq!(pkt.max_chunk_radius, 16);
    }
}
