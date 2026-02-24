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
#[derive(Debug, Clone)]
pub struct ResourcePackStack {
    pub must_accept: bool,
    pub behavior_packs: Vec<StackPackEntry>,
    pub resource_packs: Vec<StackPackEntry>,
    pub game_version: String,
    pub experiments: Vec<StackExperiment>,
    pub experiments_previously_used: bool,
    pub use_vanilla_editor_packs: bool,
}

impl Default for ResourcePackStack {
    fn default() -> Self {
        Self {
            must_accept: false,
            behavior_packs: Vec::new(),
            resource_packs: Vec::new(),
            game_version: "1.21.50".into(),
            experiments: Vec::new(),
            experiments_previously_used: false,
            use_vanilla_editor_packs: false,
        }
    }
}

impl ProtoEncode for ResourcePackStack {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.must_accept as u8);

        VarUInt32(self.behavior_packs.len() as u32).proto_encode(buf);
        for bp in &self.behavior_packs {
            codec::write_string(buf, &bp.uuid);
            codec::write_string(buf, &bp.version);
            codec::write_string(buf, &bp.sub_pack_name);
        }

        VarUInt32(self.resource_packs.len() as u32).proto_encode(buf);
        for rp in &self.resource_packs {
            codec::write_string(buf, &rp.uuid);
            codec::write_string(buf, &rp.version);
            codec::write_string(buf, &rp.sub_pack_name);
        }

        codec::write_string(buf, &self.game_version);

        VarUInt32(self.experiments.len() as u32).proto_encode(buf);
        for exp in &self.experiments {
            codec::write_string(buf, &exp.name);
            buf.put_u8(exp.enabled as u8);
        }

        buf.put_u8(self.experiments_previously_used as u8);
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
        // must_accept(1) + behavior(1) + resource(1) + game_version(1+7) + experiments(1) + 2 bools = 14
        assert_eq!(buf.len(), 14);
    }

    #[test]
    fn default_contains_game_version() {
        let pkt = ResourcePackStack::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let encoded = &buf[..];
        // game_version "1.21.50" should appear in the output
        assert!(encoded.windows(7).any(|w| w == b"1.21.50"));
    }
}
