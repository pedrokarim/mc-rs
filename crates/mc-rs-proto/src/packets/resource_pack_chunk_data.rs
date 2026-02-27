//! ResourcePackChunkData (0x53) — Server → Client.
//!
//! Sends a single chunk of pack data in response to a chunk request.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::VarUInt32;

/// One chunk of resource/behavior pack binary data.
#[derive(Debug, Clone)]
pub struct ResourcePackChunkData {
    /// Pack UUID + version.
    pub pack_id: String,
    /// Zero-based chunk index.
    pub chunk_index: u32,
    /// Byte offset of this chunk in the full pack (chunk_index * max_chunk_size).
    pub progress: u64,
    /// Raw chunk bytes.
    pub data: Vec<u8>,
}

impl ProtoEncode for ResourcePackChunkData {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        codec::write_string(buf, &self.pack_id);
        buf.put_u32_le(self.chunk_index);
        buf.put_u64_le(self.progress);
        VarUInt32(self.data.len() as u32).proto_encode(buf);
        buf.put_slice(&self.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_chunk_data() {
        let pkt = ResourcePackChunkData {
            pack_id: "id".into(),
            chunk_index: 0,
            progress: 0,
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // pack_id: 1 + 2 = 3
        // chunk_index: 4
        // progress: 8
        // data len (VarUInt32): 1
        // data: 4
        // Total = 3 + 4 + 8 + 1 + 4 = 20
        assert_eq!(buf.len(), 20);
        // Verify data bytes at the end
        assert_eq!(&buf[buf.len() - 4..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
