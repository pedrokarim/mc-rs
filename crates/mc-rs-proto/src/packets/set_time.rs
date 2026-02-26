//! SetTime (0x0A) — Server → Client.
//!
//! Synchronizes the world time (day/night cycle) to all clients.

use bytes::BufMut;

use crate::codec::ProtoEncode;

/// SetTime packet.
pub struct SetTime {
    /// Current world time in ticks (wraps at 24000 for day/night cycle).
    pub time: i32,
}

impl ProtoEncode for SetTime {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_i32_le(self.time);
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
        assert_eq!(buf.len(), 4);
        // i32_le for 6000
        assert_eq!(&buf[..], &6000_i32.to_le_bytes());
    }

    #[test]
    fn encode_set_time_midnight() {
        let pkt = SetTime { time: 18000 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(&buf[..], &18000_i32.to_le_bytes());
    }
}
