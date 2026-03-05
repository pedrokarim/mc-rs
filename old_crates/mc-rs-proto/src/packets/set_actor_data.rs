//! SetActorData (0x27) — Server → Client.
//!
//! Synchronizes entity metadata for an existing actor.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::packets::add_player::{encode_entity_metadata, EntityMetadataEntry};
use crate::types::{VarUInt32, VarUInt64};

/// SetActorData packet.
pub struct SetActorData {
    pub actor_runtime_id: u64,
    pub metadata: Vec<EntityMetadataEntry>,
    pub tick: u64,
}

impl ProtoEncode for SetActorData {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.actor_runtime_id).proto_encode(buf);
        encode_entity_metadata(buf, &self.metadata);
        // PropertySyncData: int_properties_count + float_properties_count.
        VarUInt32(0).proto_encode(buf);
        VarUInt32(0).proto_encode(buf);
        VarUInt64(self.tick).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::add_player::default_player_metadata;
    use bytes::BytesMut;

    #[test]
    fn encode_set_actor_data_minimal() {
        let pkt = SetActorData {
            actor_runtime_id: 1,
            metadata: default_player_metadata("Steve"),
            tick: 0,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
    }
}
