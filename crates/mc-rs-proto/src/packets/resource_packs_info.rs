//! ResourcePacksInfo (0x06) — Server → Client.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::Uuid;

/// Resource pack entry sent in ResourcePacksInfo (protocol 924+).
///
/// In the new format behavior packs and resource packs are merged into a single
/// list, each entry carries its own `cdn_url`, and `pack_id` is a proper UUID.
#[derive(Debug, Clone)]
pub struct ResourcePackInfoEntry {
    pub pack_id: Uuid,
    pub version: String,
    pub size: u64,
    pub encryption_key: String,
    pub sub_pack_name: String,
    pub content_id: String,
    pub has_scripts: bool,
    pub is_addon_pack: bool,
    pub is_rtx_capable: bool,
    pub cdn_url: String,
}

/// Tells the client what resource/behavior packs are available.
#[derive(Debug, Clone)]
pub struct ResourcePacksInfo {
    pub must_accept: bool,
    pub has_addons: bool,
    pub has_scripts: bool,
    pub force_disable_vibrant_visuals: bool,
    pub world_template_id: Uuid,
    pub world_template_version: String,
    pub resource_packs: Vec<ResourcePackInfoEntry>,
}

impl Default for ResourcePacksInfo {
    fn default() -> Self {
        Self {
            must_accept: false,
            has_addons: false,
            has_scripts: false,
            force_disable_vibrant_visuals: false,
            world_template_id: Uuid::ZERO,
            world_template_version: String::new(),
            resource_packs: Vec::new(),
        }
    }
}

impl ProtoEncode for ResourcePacksInfo {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.must_accept as u8);
        buf.put_u8(self.has_addons as u8);
        buf.put_u8(self.has_scripts as u8);
        buf.put_u8(self.force_disable_vibrant_visuals as u8);
        self.world_template_id.proto_encode(buf);
        codec::write_string(buf, &self.world_template_version);

        // NOTE: Pack count uses uint16_le, NOT VarUInt32 (PMMP-confirmed).
        buf.put_u16_le(self.resource_packs.len() as u16);
        for rp in &self.resource_packs {
            rp.pack_id.proto_encode(buf);
            codec::write_string(buf, &rp.version);
            buf.put_u64_le(rp.size);
            codec::write_string(buf, &rp.encryption_key);
            codec::write_string(buf, &rp.sub_pack_name);
            codec::write_string(buf, &rp.content_id);
            buf.put_u8(rp.has_scripts as u8);
            buf.put_u8(rp.is_addon_pack as u8);
            buf.put_u8(rp.is_rtx_capable as u8);
            codec::write_string(buf, &rp.cdn_url);
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
        // 4 bools + 16 UUID + 1 VarUInt32(0) for empty string + 2 u16_le(0) = 4 + 16 + 1 + 2 = 23 bytes
        assert_eq!(buf.len(), 23);
        assert_eq!(buf[0], 0x00); // must_accept
        assert_eq!(buf[1], 0x00); // has_addons
        assert_eq!(buf[2], 0x00); // has_scripts
        assert_eq!(buf[3], 0x00); // force_disable_vibrant_visuals
                                  // bytes 4..20: UUID zero (16 bytes)
        for i in 4..20 {
            assert_eq!(buf[i], 0x00, "UUID byte {i} should be 0");
        }
        assert_eq!(buf[20], 0x00); // VarUInt32(0) for empty world_template_version
        assert_eq!(buf[21], 0x00); // resource_packs count u16_le low byte
        assert_eq!(buf[22], 0x00); // resource_packs count u16_le high byte
    }

    #[test]
    fn pack_count_is_u16_le() {
        let pkt = ResourcePacksInfo::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Last two bytes: resource_packs count as u16_le
        let len = buf.len();
        assert_eq!(buf[len - 2], 0x00);
        assert_eq!(buf[len - 1], 0x00);
    }

    #[test]
    fn encode_with_one_pack() {
        let pkt = ResourcePacksInfo {
            must_accept: true,
            resource_packs: vec![ResourcePackInfoEntry {
                pack_id: Uuid::ZERO,
                version: "1.0.0".into(),
                size: 1024,
                encryption_key: String::new(),
                sub_pack_name: String::new(),
                content_id: String::new(),
                has_scripts: false,
                is_addon_pack: false,
                is_rtx_capable: false,
                cdn_url: String::new(),
            }],
            ..ResourcePacksInfo::default()
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x01); // must_accept = true
                                  // Should be larger than the empty default
        assert!(buf.len() > 23);
    }
}
