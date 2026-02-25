//! MoveActorAbsolute (0x10) — Server → Client.
//!
//! Updates the absolute position and rotation of a non-player entity.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarUInt64, Vec3};

/// MoveActorAbsolute packet.
pub struct MoveActorAbsolute {
    pub entity_runtime_id: u64,
    /// Flags: bit 0 = on_ground, bit 1 = teleport.
    pub flags: u16,
    pub position: Vec3,
    /// Pitch in degrees (encoded as compressed byte on wire).
    pub pitch: f32,
    /// Yaw in degrees (encoded as compressed byte on wire).
    pub yaw: f32,
    /// Head yaw in degrees (encoded as compressed byte on wire).
    pub head_yaw: f32,
}

impl MoveActorAbsolute {
    /// Compress an angle (0..360) to a single byte.
    fn angle_to_byte(angle: f32) -> u8 {
        ((angle % 360.0 + 360.0) % 360.0 * (256.0 / 360.0)) as u8
    }

    /// Create a normal (non-teleport) move packet.
    pub fn normal(
        runtime_id: u64,
        position: Vec3,
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
        on_ground: bool,
    ) -> Self {
        Self {
            entity_runtime_id: runtime_id,
            flags: if on_ground { 1 } else { 0 },
            position,
            pitch,
            yaw,
            head_yaw,
        }
    }
}

impl ProtoEncode for MoveActorAbsolute {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        buf.put_u16_le(self.flags);
        self.position.proto_encode(buf);
        buf.put_u8(Self::angle_to_byte(self.pitch));
        buf.put_u8(Self::angle_to_byte(self.yaw));
        buf.put_u8(Self::angle_to_byte(self.head_yaw));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_basic() {
        let pkt = MoveActorAbsolute::normal(42, Vec3::new(10.0, 4.0, 10.0), 0.0, 90.0, 90.0, true);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt64(42)=1 byte + u16_le(flags)=2 + Vec3=12 + 3 angle bytes = 18
        assert_eq!(buf.len(), 18);
    }

    #[test]
    fn angle_to_byte_conversions() {
        assert_eq!(MoveActorAbsolute::angle_to_byte(0.0), 0);
        assert_eq!(MoveActorAbsolute::angle_to_byte(90.0), 64);
        assert_eq!(MoveActorAbsolute::angle_to_byte(180.0), 128);
        assert_eq!(MoveActorAbsolute::angle_to_byte(270.0), 192);
        // 360 wraps to 0
        assert_eq!(MoveActorAbsolute::angle_to_byte(360.0), 0);
        // Negative angles wrap correctly
        assert_eq!(MoveActorAbsolute::angle_to_byte(-90.0), 192);
    }

    #[test]
    fn on_ground_flag() {
        let grounded = MoveActorAbsolute::normal(1, Vec3::ZERO, 0.0, 0.0, 0.0, true);
        let airborne = MoveActorAbsolute::normal(1, Vec3::ZERO, 0.0, 0.0, 0.0, false);
        assert_eq!(grounded.flags, 1);
        assert_eq!(airborne.flags, 0);
    }
}
