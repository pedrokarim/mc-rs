//! SetPlayerGameType (0x3E) — Server → Client.
//!
//! Changes the client's local gamemode display.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarInt;

/// Notify the client that its gamemode has changed.
pub struct SetPlayerGameType {
    /// 0 = survival, 1 = creative, 2 = adventure, 3 = spectator.
    pub gamemode: i32,
}

impl ProtoEncode for SetPlayerGameType {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.gamemode).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_survival() {
        let pkt = SetPlayerGameType { gamemode: 0 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(0) zigzag = 0
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x00);
    }

    #[test]
    fn encode_creative() {
        let pkt = SetPlayerGameType { gamemode: 1 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarInt(1) zigzag = 2
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x02);
    }
}
