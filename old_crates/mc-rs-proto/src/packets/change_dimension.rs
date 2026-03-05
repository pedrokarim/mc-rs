//! ChangeDimension (0x3D) — Server → Client.
//!
//! Tells the client to transition to a different dimension.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarInt, Vec3};

/// ChangeDimension packet: initiates a dimension change on the client.
pub struct ChangeDimension {
    /// Target dimension ID: 0=Overworld, 1=Nether, 2=End.
    pub dimension: i32,
    /// Spawn position in the target dimension.
    pub position: Vec3,
    /// Whether the player should respawn (typically false for portal travel).
    pub respawn: bool,
}

impl ProtoEncode for ChangeDimension {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.dimension).proto_encode(buf);
        self.position.proto_encode(buf);
        buf.put_u8(self.respawn as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_change_dimension_nether() {
        let pkt = ChangeDimension {
            dimension: 1,
            position: Vec3::new(10.0, 64.0, 20.0),
            respawn: false,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(1) = 0x02 (zigzag), Vec3(12 bytes), u8(0)
        assert_eq!(buf[0], 0x02); // dimension=1 zigzag-encoded
        assert_eq!(*buf.last().unwrap(), 0x00); // respawn=false
        assert_eq!(buf.len(), 1 + 12 + 1); // VarInt(1) + Vec3 + bool
    }

    #[test]
    fn encode_change_dimension_end() {
        let pkt = ChangeDimension {
            dimension: 2,
            position: Vec3::new(0.5, 50.0, 0.5),
            respawn: true,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x04); // dimension=2 zigzag-encoded
        assert_eq!(*buf.last().unwrap(), 0x01); // respawn=true
    }

    #[test]
    fn encode_change_dimension_overworld() {
        let pkt = ChangeDimension {
            dimension: 0,
            position: Vec3::new(100.0, 65.0, -200.0),
            respawn: false,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x00); // dimension=0 zigzag-encoded
    }
}
