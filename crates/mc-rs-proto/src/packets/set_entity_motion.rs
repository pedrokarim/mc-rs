//! SetEntityMotion (0x12) — Server → Client.
//!
//! Applies a velocity to an entity. Used for knockback.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarUInt64, Vec3};

/// SetEntityMotion packet.
pub struct SetEntityMotion {
    pub entity_runtime_id: u64,
    pub motion: Vec3,
}

impl ProtoEncode for SetEntityMotion {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        self.motion.proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_knockback() {
        let pkt = SetEntityMotion {
            entity_runtime_id: 3,
            motion: Vec3::new(0.4, 0.4, 0.0),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt64(3) = 1 byte + Vec3(12 bytes) = 13
        assert_eq!(buf.len(), 13);
    }
}
