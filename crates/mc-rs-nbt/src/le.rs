//! Standard little-endian NBT variant (used for disk storage and chunk data).

use bytes::{Buf, BufMut};

use crate::error::NbtError;
use crate::io::NbtVariant;

pub(crate) struct LeVariant;

impl NbtVariant for LeVariant {
    fn write_int(buf: &mut impl BufMut, value: i32) {
        buf.put_i32_le(value);
    }

    fn read_int(buf: &mut impl Buf) -> Result<i32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i32_le())
    }

    fn write_array_len(buf: &mut impl BufMut, len: i32) {
        buf.put_i32_le(len);
    }

    fn read_array_len(buf: &mut impl Buf) -> Result<i32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i32_le())
    }

    fn write_string_len(buf: &mut impl BufMut, len: usize) {
        buf.put_u16_le(len as u16);
    }

    fn read_string_len(buf: &mut impl Buf) -> Result<usize, NbtError> {
        if buf.remaining() < 2 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_u16_le() as usize)
    }
}
