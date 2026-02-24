//! NetworkSettings (0x8F) — Server → Client.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;

/// Compression and throttle settings sent by the server after receiving
/// `RequestNetworkSettings`. Compression is activated immediately after this
/// packet is sent/received.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkSettings {
    /// Packets smaller than this are sent uncompressed.
    pub compression_threshold: u16,
    /// Compression algorithm (0 = Zlib, 1 = Snappy, 0xFFFF = None).
    pub compression_algorithm: u16,
    /// Whether client-side packet throttling is enabled.
    pub client_throttle_enabled: bool,
    /// Threshold for client throttle.
    pub client_throttle_threshold: u8,
    /// Scalar multiplier for client throttle.
    pub client_throttle_scalar: f32,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            compression_threshold: 256,
            compression_algorithm: 0, // Zlib
            client_throttle_enabled: false,
            client_throttle_threshold: 0,
            client_throttle_scalar: 0.0,
        }
    }
}

impl ProtoEncode for NetworkSettings {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u16_le(self.compression_threshold);
        buf.put_u16_le(self.compression_algorithm);
        buf.put_u8(self.client_throttle_enabled as u8);
        buf.put_u8(self.client_throttle_threshold);
        buf.put_f32_le(self.client_throttle_scalar);
    }
}

impl ProtoDecode for NetworkSettings {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 10 {
            return Err(ProtoError::BufferTooShort {
                needed: 10,
                remaining: buf.remaining(),
            });
        }
        Ok(Self {
            compression_threshold: buf.get_u16_le(),
            compression_algorithm: buf.get_u16_le(),
            client_throttle_enabled: buf.get_u8() != 0,
            client_throttle_threshold: buf.get_u8(),
            client_throttle_scalar: buf.get_f32_le(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_default() {
        let settings = NetworkSettings::default();
        let mut buf = BytesMut::new();
        settings.proto_encode(&mut buf);
        // 2 (threshold) + 2 (algo) + 1 (throttle_enabled) + 1 (throttle_threshold) + 4 (scalar) = 10
        assert_eq!(buf.len(), 10);
        // threshold=256 LE = [0x00, 0x01]
        assert_eq!(&buf[..2], &[0x00, 0x01]);
        // algorithm=0 LE = [0x00, 0x00]
        assert_eq!(&buf[2..4], &[0x00, 0x00]);
    }

    #[test]
    fn roundtrip() {
        let settings = NetworkSettings {
            compression_threshold: 512,
            compression_algorithm: 1, // Snappy
            client_throttle_enabled: true,
            client_throttle_threshold: 10,
            client_throttle_scalar: 1.5,
        };
        let mut buf = BytesMut::new();
        settings.proto_encode(&mut buf);
        let decoded = NetworkSettings::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn decode_buffer_too_short() {
        let data = bytes::Bytes::from_static(&[0x00, 0x01, 0x00]);
        assert!(NetworkSettings::proto_decode(&mut data.clone()).is_err());
    }
}
