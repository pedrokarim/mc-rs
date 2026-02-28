//! RequestNetworkSettings (0xC1) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;

/// The first game packet sent by the client after the RakNet handshake.
/// Contains the client's protocol version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestNetworkSettings {
    /// Protocol version (int32 big-endian).
    pub protocol_version: i32,
}

impl ProtoDecode for RequestNetworkSettings {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        Ok(Self {
            protocol_version: buf.get_i32(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn decode_protocol_924() {
        // 924 in big-endian = 0x00_00_03_9C
        let data = Bytes::from_static(&[0x00, 0x00, 0x03, 0x9C]);
        let pkt = RequestNetworkSettings::proto_decode(&mut data.clone()).unwrap();
        assert_eq!(pkt.protocol_version, 924);
    }

    #[test]
    fn decode_buffer_too_short() {
        let data = Bytes::from_static(&[0x00, 0x00]);
        assert!(RequestNetworkSettings::proto_decode(&mut data.clone()).is_err());
    }
}
