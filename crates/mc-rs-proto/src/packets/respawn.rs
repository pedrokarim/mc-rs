//! Respawn (0x2D) — Bidirectional.
//!
//! Server → Client: triggers death screen (state=0/1).
//! Client → Server: player clicked respawn (state=2).

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::{VarUInt64, Vec3};

/// Respawn packet.
pub struct Respawn {
    pub position: Vec3,
    /// 0 = searching, 1 = server_ready, 2 = client_ready.
    pub state: u8,
    pub runtime_entity_id: u64,
}

impl ProtoEncode for Respawn {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.position.proto_encode(buf);
        buf.put_u8(self.state);
        VarUInt64(self.runtime_entity_id).proto_encode(buf);
    }
}

impl ProtoDecode for Respawn {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let position = Vec3::proto_decode(buf)?;
        let state = if buf.remaining() > 0 {
            buf.get_u8()
        } else {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: 0,
            });
        };
        let runtime_entity_id = VarUInt64::proto_decode(buf)?.0;
        Ok(Self {
            position,
            state,
            runtime_entity_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn roundtrip_client_ready() {
        let pkt = Respawn {
            position: Vec3::new(0.5, 5.62, 0.5),
            state: 2, // client_ready
            runtime_entity_id: 7,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = Respawn::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.state, 2);
        assert_eq!(decoded.runtime_entity_id, 7);
    }

    #[test]
    fn encode_respawn_server_ready() {
        let pkt = Respawn {
            position: Vec3::new(0.5, 5.62, 0.5),
            state: 1,
            runtime_entity_id: 1,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Vec3(12) + u8(1) + VarUInt64(1)
        assert!(buf.len() >= 14);
        assert_eq!(buf[12], 1); // state = server_ready
    }
}
