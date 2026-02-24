use bytes::{Buf, BufMut};

use crate::constants::RAKNET_MAGIC;
use crate::error::RakNetError;

/// Read a 24-bit unsigned integer in little-endian byte order.
pub fn read_u24_le(buf: &mut impl Buf) -> u32 {
    let b0 = buf.get_u8() as u32;
    let b1 = buf.get_u8() as u32;
    let b2 = buf.get_u8() as u32;
    b0 | (b1 << 8) | (b2 << 16)
}

/// Write a 24-bit unsigned integer in little-endian byte order.
pub fn write_u24_le(buf: &mut impl BufMut, val: u32) {
    buf.put_u8((val & 0xFF) as u8);
    buf.put_u8(((val >> 8) & 0xFF) as u8);
    buf.put_u8(((val >> 16) & 0xFF) as u8);
}

/// Read the 16-byte RakNet magic and validate it.
pub fn read_magic(buf: &mut impl Buf) -> Result<(), RakNetError> {
    if buf.remaining() < 16 {
        return Err(RakNetError::PacketTooShort {
            expected: 16,
            actual: buf.remaining(),
        });
    }
    let mut magic = [0u8; 16];
    buf.copy_to_slice(&mut magic);
    if magic != RAKNET_MAGIC {
        return Err(RakNetError::InvalidMagic);
    }
    Ok(())
}

/// Write the 16-byte RakNet magic.
pub fn write_magic(buf: &mut impl BufMut) {
    buf.put_slice(&RAKNET_MAGIC);
}

/// Read a UTF-8 string prefixed by a u16 BE length.
pub fn read_string(buf: &mut impl Buf) -> Result<String, RakNetError> {
    if buf.remaining() < 2 {
        return Err(RakNetError::PacketTooShort {
            expected: 2,
            actual: buf.remaining(),
        });
    }
    let len = buf.get_u16() as usize;
    if buf.remaining() < len {
        return Err(RakNetError::PacketTooShort {
            expected: len,
            actual: buf.remaining(),
        });
    }
    let data = buf.copy_to_bytes(len);
    String::from_utf8(data.to_vec()).map_err(|_| RakNetError::InvalidUtf8)
}

/// Write a UTF-8 string prefixed by a u16 BE length.
pub fn write_string(buf: &mut impl BufMut, s: &str) {
    buf.put_u16(s.len() as u16);
    buf.put_slice(s.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use std::io::Cursor;

    #[test]
    fn u24_le_roundtrip() {
        for val in [0u32, 1, 255, 256, 0xFF_FFFF, 12345] {
            let mut buf = BytesMut::new();
            write_u24_le(&mut buf, val);
            assert_eq!(buf.len(), 3);
            let mut cursor = Cursor::new(&buf[..]);
            assert_eq!(read_u24_le(&mut cursor), val);
        }
    }

    #[test]
    fn magic_roundtrip() {
        let mut buf = BytesMut::new();
        write_magic(&mut buf);
        assert_eq!(buf.len(), 16);
        let mut cursor = Cursor::new(&buf[..]);
        assert!(read_magic(&mut cursor).is_ok());
    }

    #[test]
    fn magic_invalid() {
        let mut bad = RAKNET_MAGIC;
        bad[0] = 0xFF;
        let mut cursor = Cursor::new(&bad[..]);
        assert!(read_magic(&mut cursor).is_err());
    }

    #[test]
    fn string_roundtrip() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "Hello MC-RS!");
        let mut cursor = Cursor::new(&buf[..]);
        assert_eq!(read_string(&mut cursor).unwrap(), "Hello MC-RS!");
    }

    #[test]
    fn string_empty() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "");
        let mut cursor = Cursor::new(&buf[..]);
        assert_eq!(read_string(&mut cursor).unwrap(), "");
    }
}
