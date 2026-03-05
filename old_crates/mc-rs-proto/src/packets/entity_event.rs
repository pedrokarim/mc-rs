//! EntityEvent (0x1B) — Server → Client.
//!
//! Notifies clients of entity events: hurt animation, death, etc.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarInt, VarUInt64};

/// Hurt animation event.
pub const EVENT_HURT: u8 = 2;
/// Death event.
pub const EVENT_DEATH: u8 = 3;
/// Love/breeding particles event.
pub const EVENT_LOVE_PARTICLES: u8 = 18;

/// EntityEvent packet.
pub struct EntityEvent {
    pub entity_runtime_id: u64,
    pub event_id: u8,
    pub data: i32,
}

impl EntityEvent {
    /// Create a hurt event for an entity.
    pub fn hurt(entity_runtime_id: u64) -> Self {
        Self {
            entity_runtime_id,
            event_id: EVENT_HURT,
            data: 0,
        }
    }

    /// Create a death event for an entity.
    pub fn death(entity_runtime_id: u64) -> Self {
        Self {
            entity_runtime_id,
            event_id: EVENT_DEATH,
            data: 0,
        }
    }

    /// Create a love/breeding particles event for an entity.
    pub fn love_particles(entity_runtime_id: u64) -> Self {
        Self {
            entity_runtime_id,
            event_id: EVENT_LOVE_PARTICLES,
            data: 0,
        }
    }
}

impl ProtoEncode for EntityEvent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        buf.put_u8(self.event_id);
        VarInt(self.data).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_hurt() {
        let pkt = EntityEvent::hurt(5);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt64(5) + u8(2) + VarInt(0) = at least 3 bytes
        assert!(buf.len() >= 3);
        assert_eq!(buf[1], EVENT_HURT);
    }

    #[test]
    fn encode_death() {
        let pkt = EntityEvent::death(10);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 3);
        assert_eq!(buf[1], EVENT_DEATH);
    }

    #[test]
    fn encode_love_particles() {
        let pkt = EntityEvent::love_particles(7);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 3);
        assert_eq!(buf[1], EVENT_LOVE_PARTICLES);
    }
}
