//! Network NBT variant (VarInt for ints/lengths, VarUInt32 for string lengths).

use bytes::{Buf, BufMut};

use crate::error::NbtError;
use crate::io::NbtVariant;

pub(crate) struct NetworkVariant;

// Inline VarInt helpers (no dependency on mc-rs-proto)

fn write_var_i32(buf: &mut impl BufMut, value: i32) {
    // ZigZag encode
    let encoded = ((value << 1) ^ (value >> 31)) as u32;
    write_var_u32(buf, encoded);
}

fn read_var_i32(buf: &mut impl Buf) -> Result<i32, NbtError> {
    let encoded = read_var_u32(buf)?;
    // ZigZag decode
    Ok(((encoded >> 1) as i32) ^ -((encoded & 1) as i32))
}

fn write_var_u32(buf: &mut impl BufMut, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.put_u8(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_var_u32(buf: &mut impl Buf) -> Result<u32, NbtError> {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        if !buf.has_remaining() {
            return Err(NbtError::VarInt("unexpected end of buffer".to_string()));
        }
        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 35 {
            return Err(NbtError::VarInt("VarInt too long".to_string()));
        }
    }
}

impl NbtVariant for NetworkVariant {
    fn write_int(buf: &mut impl BufMut, value: i32) {
        write_var_i32(buf, value);
    }

    fn read_int(buf: &mut impl Buf) -> Result<i32, NbtError> {
        read_var_i32(buf)
    }

    fn write_array_len(buf: &mut impl BufMut, len: i32) {
        write_var_i32(buf, len);
    }

    fn read_array_len(buf: &mut impl Buf) -> Result<i32, NbtError> {
        read_var_i32(buf)
    }

    fn write_string_len(buf: &mut impl BufMut, len: usize) {
        write_var_u32(buf, len as u32);
    }

    fn read_string_len(buf: &mut impl Buf) -> Result<usize, NbtError> {
        read_var_u32(buf).map(|v| v as usize)
    }

    // Network uses LE for short/long/float/double (same as LE variant)
    fn read_short(buf: &mut impl Buf) -> Result<i16, NbtError> {
        if buf.remaining() < 2 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i16_le())
    }
    fn write_short(buf: &mut impl BufMut, value: i16) {
        buf.put_i16_le(value);
    }
    fn read_long(buf: &mut impl Buf) -> Result<i64, NbtError> {
        if buf.remaining() < 8 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i64_le())
    }
    fn write_long(buf: &mut impl BufMut, value: i64) {
        buf.put_i64_le(value);
    }
    fn read_float(buf: &mut impl Buf) -> Result<f32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_f32_le())
    }
    fn write_float(buf: &mut impl BufMut, value: f32) {
        buf.put_f32_le(value);
    }
    fn read_double(buf: &mut impl Buf) -> Result<f64, NbtError> {
        if buf.remaining() < 8 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_f64_le())
    }
    fn write_double(buf: &mut impl BufMut, value: f64) {
        buf.put_f64_le(value);
    }
}
