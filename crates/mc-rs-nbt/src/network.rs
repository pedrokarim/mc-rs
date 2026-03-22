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
}
