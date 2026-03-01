//! ResourcePackStack (0x07) — Server → Client.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::VarUInt32;

/// A pack entry in the resource pack stack.
#[derive(Debug, Clone)]
pub struct StackPackEntry {
    pub uuid: String,
    pub version: String,
    pub sub_pack_name: String,
}

/// An experiment toggle.
#[derive(Debug, Clone)]
pub struct StackExperiment {
    pub name: String,
    pub enabled: bool,
}

/// Tells the client the order in which packs should be applied.
///
/// Protocol 924+: single merged `resource_pack_stack` list (no separate
/// behavior/resource lists).
#[derive(Debug, Clone)]
pub struct ResourcePackStack {
    pub must_accept: bool,
    pub resource_pack_stack: Vec<StackPackEntry>,
    pub game_version: String,
    pub experiments: Vec<StackExperiment>,
    pub use_vanilla_editor_packs: bool,
}

impl Default for ResourcePackStack {
    fn default() -> Self {
        Self {
            must_accept: false,
            resource_pack_stack: Vec::new(),
            game_version: super::game_version_for_protocol(super::PROTOCOL_VERSION).into(),
            experiments: Vec::new(),
            use_vanilla_editor_packs: false,
        }
    }
}

impl ProtoEncode for ResourcePackStack {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.must_accept as u8);

        VarUInt32(self.resource_pack_stack.len() as u32).proto_encode(buf);
        for entry in &self.resource_pack_stack {
            codec::write_string(buf, &entry.uuid);
            codec::write_string(buf, &entry.version);
            codec::write_string(buf, &entry.sub_pack_name);
        }

        codec::write_string(buf, &self.game_version);

        // Experiments — count is u32_le (NOT VarUInt32), per PMMP Experiments::write
        buf.put_u32_le(self.experiments.len() as u32);
        for exp in &self.experiments {
            codec::write_string(buf, &exp.name);
            buf.put_u8(exp.enabled as u8);
        }
        buf.put_u8(0); // hasPreviouslyUsedExperiments = false

        buf.put_u8(self.use_vanilla_editor_packs as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_default() {
        let pkt = ResourcePackStack::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // must_accept(1) + stack_count_varuint(1) + game_version(1+6) + experiments_u32le(4) + hasPrevUsed(1) + useVanilla(1) = 15
        assert_eq!(buf.len(), 15);
    }

    #[test]
    fn default_contains_game_version() {
        let pkt = ResourcePackStack::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let encoded = &buf[..];
        // game_version "1.26.2" should appear in the output
        assert!(encoded.windows(6).any(|w| w == b"1.26.2"));
    }
}
