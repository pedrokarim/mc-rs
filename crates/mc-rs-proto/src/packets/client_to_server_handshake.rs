//! ClientToServerHandshake (0x04) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;

/// Sent by the client to confirm the encryption handshake.
///
/// This packet has no fields — its presence confirms that the client
/// has derived the encryption key and is ready for encrypted communication.
#[derive(Debug, Clone)]
pub struct ClientToServerHandshake;

impl ProtoDecode for ClientToServerHandshake {
    fn proto_decode(_buf: &mut impl Buf) -> Result<Self, ProtoError> {
        // Empty packet — no fields to decode
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn decode_empty_handshake() {
        let data = Bytes::new();
        let _pkt = ClientToServerHandshake::proto_decode(&mut data.as_ref()).unwrap();
    }

    #[test]
    fn decode_with_trailing_data() {
        // Should succeed even with extra bytes (packet length already handled by batch layer)
        let data = Bytes::from_static(&[0x00, 0x01, 0x02]);
        let _pkt = ClientToServerHandshake::proto_decode(&mut data.as_ref()).unwrap();
    }
}
