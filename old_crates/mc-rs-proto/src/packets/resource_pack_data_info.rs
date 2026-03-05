//! ResourcePackDataInfo (0x52) — Server → Client.
//!
//! Sent after the client requests packs (SendPacks status).
//! Describes pack metadata so the client can request chunks.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};

/// Metadata about a resource/behavior pack the server is about to transfer.
#[derive(Debug, Clone)]
pub struct ResourcePackDataInfo {
    /// Pack UUID + version, e.g. `"uuid_version"`.
    pub pack_id: String,
    /// Maximum chunk size in bytes (typically 1 MB = 1_048_576).
    pub max_chunk_size: u32,
    /// Total number of chunks.
    pub chunk_count: u32,
    /// Total compressed pack size in bytes.
    pub pack_size: u64,
    /// SHA-256 hash of the pack file.
    pub pack_hash: String,
    /// Whether this is a premium pack.
    pub is_premium: bool,
    /// Pack type: 1 = resource, 2 = behavior.
    pub pack_type: u8,
}

impl ProtoEncode for ResourcePackDataInfo {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        codec::write_string(buf, &self.pack_id);
        buf.put_u32_le(self.max_chunk_size);
        buf.put_u32_le(self.chunk_count);
        buf.put_u64_le(self.pack_size);
        codec::write_string(buf, &self.pack_hash);
        buf.put_u8(self.is_premium as u8);
        buf.put_u8(self.pack_type);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_data_info() {
        let pkt = ResourcePackDataInfo {
            pack_id: "abc".into(),
            max_chunk_size: 1_048_576,
            chunk_count: 2,
            pack_size: 2_000_000,
            pack_hash: "h".into(),
            is_premium: false,
            pack_type: 2,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // pack_id: 1 (varuint) + 3 bytes = 4
        // max_chunk_size: 4
        // chunk_count: 4
        // pack_size: 8
        // pack_hash: 1 + 1 = 2
        // is_premium: 1
        // pack_type: 1
        // Total = 4 + 4 + 4 + 8 + 2 + 1 + 1 = 24
        assert_eq!(buf.len(), 24);
    }
}
