//! Animate (0x2C) — Bidirectional.
//!
//! Client sends when player swings arm; server broadcasts to other players.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::{VarInt, VarUInt64};

/// Arm swing action.
pub const ACTION_SWING_ARM: i32 = 1;

/// Animate packet.
pub struct Animate {
    pub action_type: i32,
    pub entity_runtime_id: u64,
}

impl ProtoEncode for Animate {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.action_type).proto_encode(buf);
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
    }
}

impl ProtoDecode for Animate {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let action_type = VarInt::proto_decode(buf)?.0;
        let entity_runtime_id = VarUInt64::proto_decode(buf)?.0;
        Ok(Self {
            action_type,
            entity_runtime_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn roundtrip_swing_arm() {
        let pkt = Animate {
            action_type: ACTION_SWING_ARM,
            entity_runtime_id: 42,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);

        let decoded = Animate::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.action_type, ACTION_SWING_ARM);
        assert_eq!(decoded.entity_runtime_id, 42);
    }

    #[test]
    fn encode_critical_hit() {
        let pkt = Animate {
            action_type: 4, // critical hit
            entity_runtime_id: 1,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 2);
    }
}
