//! ResourcePackChunkRequest (0x54) — Client → Server.
//!
//! The client requests a specific chunk of a pack after receiving DataInfo.

use bytes::Buf;

use crate::codec::{self, ProtoDecode};
use crate::error::ProtoError;

/// Client request for a specific pack data chunk.
#[derive(Debug, Clone)]
pub struct ResourcePackChunkRequest {
    /// Pack UUID + version.
    pub pack_id: String,
    /// Zero-based chunk index to request.
    pub chunk_index: u32,
}

impl ProtoDecode for ResourcePackChunkRequest {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let pack_id = codec::read_string(buf)?;
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let chunk_index = buf.get_u32_le();
        Ok(Self {
            pack_id,
            chunk_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn decode_chunk_request() {
        let mut buf = BytesMut::new();
        // pack_id string: VarUInt32(4) + "test"
        buf.put_u8(4);
        buf.put_slice(b"test");
        // chunk_index: u32_le = 3
        buf.put_u32_le(3);
        let pkt = ResourcePackChunkRequest::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.pack_id, "test");
        assert_eq!(pkt.chunk_index, 3);
    }

    #[test]
    fn decode_truncated_fails() {
        let mut buf = BytesMut::new();
        buf.put_u8(2);
        buf.put_slice(b"ab");
        // Missing chunk_index bytes
        assert!(ResourcePackChunkRequest::proto_decode(&mut buf.freeze().as_ref()).is_err());
    }
}
