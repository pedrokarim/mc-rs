//! SetDifficulty (0x3C) — Server → Client.
//!
//! Synchronizes world difficulty (0=peaceful, 1=easy, 2=normal, 3=hard).

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarUInt32;

/// SetDifficulty packet.
pub struct SetDifficulty {
    pub difficulty: u32,
}

impl ProtoEncode for SetDifficulty {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.difficulty).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_peaceful() {
        let pkt = SetDifficulty { difficulty: 0 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x00]);
    }

    #[test]
    fn encode_hard() {
        let pkt = SetDifficulty { difficulty: 3 };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x03]);
    }
}
