//! SetTime (0x0A) — Server → Client.
//!
//! Synchronizes the world time (day/night cycle) to all clients.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarInt;

/// SetTime packet.
pub struct SetTime {
    /// Current world time in ticks (wraps at 24000 for day/night cycle).
    pub time: i32,
}

impl ProtoEncode for SetTime {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.time).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_set_time() {
        let pkt = SetTime { time: 6000 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(6000) zigzag = 12000 => 0xE0 0x5D
        assert_eq!(&buf[..], &[0xE0, 0x5D]);
    }

    #[test]
    fn encode_set_time_midnight() {
        let pkt = SetTime { time: 18000 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(18000) zigzag = 36000 => 0xA0 0x99 0x02
        assert_eq!(&buf[..], &[0xA0, 0x99, 0x02]);
    }
}
