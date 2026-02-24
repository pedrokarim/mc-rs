//! ResourcePacksInfo (0x06) — Server → Client.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::VarUInt32;

/// Behavior pack entry sent in ResourcePacksInfo.
#[derive(Debug, Clone)]
pub struct BehaviorPackEntry {
    pub uuid: String,
    pub version: String,
    pub size: u64,
    pub content_key: String,
    pub sub_pack_name: String,
    pub content_identity: String,
    pub has_scripts: bool,
}

/// Resource pack entry sent in ResourcePacksInfo.
#[derive(Debug, Clone)]
pub struct ResourcePackEntry {
    pub uuid: String,
    pub version: String,
    pub size: u64,
    pub content_key: String,
    pub sub_pack_name: String,
    pub content_identity: String,
    pub has_scripts: bool,
    pub rtx_enabled: bool,
}

/// CDN URL entry for remote pack downloads.
#[derive(Debug, Clone)]
pub struct CdnUrlEntry {
    pub pack_id: String,
    pub remote_url: String,
}

/// Tells the client what resource/behavior packs are available.
#[derive(Debug, Clone, Default)]
pub struct ResourcePacksInfo {
    pub must_accept_packs_for_skins: bool,
    pub scripting_enabled: bool,
    pub forcing_server_packs: bool,
    pub behavior_packs: Vec<BehaviorPackEntry>,
    pub resource_packs: Vec<ResourcePackEntry>,
    pub cdn_urls: Vec<CdnUrlEntry>,
}

impl ProtoEncode for ResourcePacksInfo {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.must_accept_packs_for_skins as u8);
        buf.put_u8(self.scripting_enabled as u8);
        buf.put_u8(self.forcing_server_packs as u8);

        // NOTE: Pack counts use uint16_le, NOT VarUInt32 (PMMP-confirmed).
        buf.put_u16_le(self.behavior_packs.len() as u16);
        for bp in &self.behavior_packs {
            codec::write_string(buf, &bp.uuid);
            codec::write_string(buf, &bp.version);
            buf.put_u64_le(bp.size);
            codec::write_string(buf, &bp.content_key);
            codec::write_string(buf, &bp.sub_pack_name);
            codec::write_string(buf, &bp.content_identity);
            buf.put_u8(bp.has_scripts as u8);
        }

        buf.put_u16_le(self.resource_packs.len() as u16);
        for rp in &self.resource_packs {
            codec::write_string(buf, &rp.uuid);
            codec::write_string(buf, &rp.version);
            buf.put_u64_le(rp.size);
            codec::write_string(buf, &rp.content_key);
            codec::write_string(buf, &rp.sub_pack_name);
            codec::write_string(buf, &rp.content_identity);
            buf.put_u8(rp.has_scripts as u8);
            buf.put_u8(rp.rtx_enabled as u8);
        }

        // CDN URLs use VarUInt32 for count (different from packs).
        VarUInt32(self.cdn_urls.len() as u32).proto_encode(buf);
        for cdn in &self.cdn_urls {
            codec::write_string(buf, &cdn.pack_id);
            codec::write_string(buf, &cdn.remote_url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty_default() {
        let pkt = ResourcePacksInfo::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // 3 bools + 2 uint16_le(0) + 1 VarUInt32(0) = 3 + 4 + 1 = 8 bytes
        assert_eq!(buf.len(), 8);
        assert_eq!(
            &buf[..],
            &[
                0x00, 0x00, 0x00, // bools
                0x00, 0x00, // behavior_packs count (u16_le)
                0x00, 0x00, // resource_packs count (u16_le)
                0x00, // cdn_urls count (VarUInt32)
            ]
        );
    }

    #[test]
    fn pack_counts_are_u16_le() {
        // Verify that pack array counts are encoded as u16_le, not VarUInt32
        let pkt = ResourcePacksInfo::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Bytes 3-4: behavior_packs count as u16_le
        assert_eq!(buf[3], 0x00);
        assert_eq!(buf[4], 0x00);
        // Bytes 5-6: resource_packs count as u16_le
        assert_eq!(buf[5], 0x00);
        assert_eq!(buf[6], 0x00);
    }
}
