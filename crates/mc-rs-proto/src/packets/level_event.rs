//! LevelEvent (0x19) — Server → Client.
//!
//! Generic world event: particles, sounds, block effects, etc.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarInt, Vec3};

/// Event ID for block-destroy particles.
pub const PARTICLE_DESTROY_BLOCK: i32 = 2001;

/// LevelEvent packet.
#[derive(Debug, Clone)]
pub struct LevelEvent {
    pub event_id: i32,
    pub position: Vec3,
    pub data: i32,
}

impl LevelEvent {
    /// Create a block-destroy particle event at the block center.
    pub fn destroy_block(block_x: i32, block_y: i32, block_z: i32, runtime_id: u32) -> Self {
        Self {
            event_id: PARTICLE_DESTROY_BLOCK,
            position: Vec3::new(
                block_x as f32 + 0.5,
                block_y as f32 + 0.5,
                block_z as f32 + 0.5,
            ),
            data: runtime_id as i32,
        }
    }
}

impl ProtoEncode for LevelEvent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.event_id).proto_encode(buf);
        self.position.proto_encode(buf);
        VarInt(self.data).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_destroy_block() {
        let pkt = LevelEvent::destroy_block(10, 64, -5, 42);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(event_id) + Vec3(12 bytes) + VarInt(data)
        assert!(buf.len() >= 14);
        assert_eq!(pkt.event_id, PARTICLE_DESTROY_BLOCK);
        assert_eq!(pkt.position.x, 10.5);
        assert_eq!(pkt.position.y, 64.5);
        assert_eq!(pkt.position.z, -4.5);
    }
}
