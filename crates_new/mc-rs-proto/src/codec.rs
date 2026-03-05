use bytes::{Buf, BufMut};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("varint too large")]
    VarIntTooLarge,
    #[error("invalid string: {0}")]
    InvalidString(String),
}

pub trait ProtoEncode {
    fn proto_encode(&self, buf: &mut impl BufMut);
}

pub trait ProtoDecode: Sized {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, CodecError>;
}

// --- VarInt helpers (used by both proto types and external crates like NBT) ---

pub fn write_unsigned_varint32(buf: &mut impl BufMut, mut value: u32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            buf.put_u8(byte);
            break;
        }
        buf.put_u8(byte | 0x80);
    }
}

pub fn read_unsigned_varint32(buf: &mut impl Buf) -> Result<u32, CodecError> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        if !buf.has_remaining() {
            return Err(CodecError::UnexpectedEof);
        }
        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 35 {
            return Err(CodecError::VarIntTooLarge);
        }
    }
}

pub fn write_signed_varint32(buf: &mut impl BufMut, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    write_unsigned_varint32(buf, zigzag);
}

pub fn read_signed_varint32(buf: &mut impl Buf) -> Result<i32, CodecError> {
    let raw = read_unsigned_varint32(buf)?;
    Ok(((raw >> 1) as i32) ^ -((raw & 1) as i32))
}

pub fn write_unsigned_varlong(buf: &mut impl BufMut, mut value: u64) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            buf.put_u8(byte);
            break;
        }
        buf.put_u8(byte | 0x80);
    }
}

pub fn read_unsigned_varlong(buf: &mut impl Buf) -> Result<u64, CodecError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if !buf.has_remaining() {
            return Err(CodecError::UnexpectedEof);
        }
        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 70 {
            return Err(CodecError::VarIntTooLarge);
        }
    }
}

pub fn write_signed_varlong(buf: &mut impl BufMut, value: i64) {
    let zigzag = ((value << 1) ^ (value >> 63)) as u64;
    write_unsigned_varlong(buf, zigzag);
}

pub fn read_signed_varlong(buf: &mut impl Buf) -> Result<i64, CodecError> {
    let raw = read_unsigned_varlong(buf)?;
    Ok(((raw >> 1) as i64) ^ -((raw & 1) as i64))
}

pub fn write_string(buf: &mut impl BufMut, s: &str) {
    write_unsigned_varint32(buf, s.len() as u32);
    buf.put_slice(s.as_bytes());
}

pub fn read_string(buf: &mut impl Buf) -> Result<String, CodecError> {
    let len = read_unsigned_varint32(buf)? as usize;
    if buf.remaining() < len {
        return Err(CodecError::UnexpectedEof);
    }
    let mut data = vec![0u8; len];
    buf.copy_to_slice(&mut data);
    String::from_utf8(data).map_err(|e| CodecError::InvalidString(e.to_string()))
}

pub fn write_block_pos(buf: &mut impl BufMut, x: i32, y: u32, z: i32) {
    write_signed_varint32(buf, x);
    write_unsigned_varint32(buf, y);
    write_signed_varint32(buf, z);
}

pub fn write_vec3f(buf: &mut impl BufMut, x: f32, y: f32, z: f32) {
    buf.put_f32_le(x);
    buf.put_f32_le(y);
    buf.put_f32_le(z);
}

pub fn write_uuid(buf: &mut impl BufMut, uuid: &[u8; 16]) {
    buf.put_slice(uuid);
}
