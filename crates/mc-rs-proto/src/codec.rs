//! Protocol encoding/decoding traits and helpers.

use bytes::{Buf, BufMut};

use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Encode a value onto a buffer.
pub trait ProtoEncode {
    fn proto_encode(&self, buf: &mut impl BufMut);
}

/// Decode a value from a buffer.
pub trait ProtoDecode: Sized {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError>;
}

/// Write a Bedrock protocol string (VarUInt32 length + UTF-8).
pub fn write_string(buf: &mut impl BufMut, s: &str) {
    VarUInt32(s.len() as u32).proto_encode(buf);
    buf.put_slice(s.as_bytes());
}

/// Read a Bedrock protocol string (VarUInt32 length + UTF-8).
pub fn read_string(buf: &mut impl Buf) -> Result<String, ProtoError> {
    let len = VarUInt32::proto_decode(buf)?.0 as usize;
    if buf.remaining() < len {
        return Err(ProtoError::BufferTooShort {
            needed: len,
            remaining: buf.remaining(),
        });
    }
    let data = buf.copy_to_bytes(len);
    String::from_utf8(data.to_vec()).map_err(|_| ProtoError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn string_roundtrip() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "Hello, Bedrock!");
        let result = read_string(&mut buf.freeze()).unwrap();
        assert_eq!(result, "Hello, Bedrock!");
    }

    #[test]
    fn string_empty() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "");
        let result = read_string(&mut buf.freeze()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn string_unicode() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "日本語テスト");
        let result = read_string(&mut buf.freeze()).unwrap();
        assert_eq!(result, "日本語テスト");
    }

    #[test]
    fn string_buffer_too_short() {
        // Write a string but truncate the buffer
        let mut buf = BytesMut::new();
        write_string(&mut buf, "Hello");
        let truncated = buf.freeze().slice(..3); // Only length prefix, not full data
        assert!(read_string(&mut truncated.clone()).is_err());
    }
}
