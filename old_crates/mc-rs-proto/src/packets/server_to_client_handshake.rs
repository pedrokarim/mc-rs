//! ServerToClientHandshake (0x03) — Server → Client.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};

/// Sent by the server to initiate encryption.
///
/// Contains a JWT signed with ES384 that includes the server's public key
/// and a random salt for key derivation.
#[derive(Debug, Clone)]
pub struct ServerToClientHandshake {
    /// JWT string: `header.payload.signature`
    pub jwt: String,
}

impl ProtoEncode for ServerToClientHandshake {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        codec::write_string(buf, &self.jwt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    use crate::codec::ProtoDecode;
    use crate::types::VarUInt32;

    #[test]
    fn encode_handshake() {
        let pkt = ServerToClientHandshake {
            jwt: "header.payload.signature".into(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);

        // Verify it's a VarString: VarUInt32 length + UTF-8 data
        let frozen = buf.freeze();
        let mut cursor = &frozen[..];
        let len = VarUInt32::proto_decode(&mut cursor).unwrap().0 as usize;
        assert_eq!(len, "header.payload.signature".len());
    }
}
