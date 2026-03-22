//! Big-endian NBT variant (Java Edition format, used by structure files).

use bytes::{Buf, BufMut};

use crate::error::NbtError;
use crate::io::NbtVariant;

pub(crate) struct BeVariant;

impl NbtVariant for BeVariant {
    fn write_int(buf: &mut impl BufMut, value: i32) {
        buf.put_i32(value); // big-endian
    }

    fn read_int(buf: &mut impl Buf) -> Result<i32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i32()) // big-endian
    }

    fn write_array_len(buf: &mut impl BufMut, len: i32) {
        buf.put_i32(len);
    }

    fn read_array_len(buf: &mut impl Buf) -> Result<i32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i32())
    }

    fn write_string_len(buf: &mut impl BufMut, len: usize) {
        buf.put_u16(len as u16); // big-endian
    }

    fn read_string_len(buf: &mut impl Buf) -> Result<usize, NbtError> {
        if buf.remaining() < 2 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_u16() as usize) // big-endian
    }

    fn read_short(buf: &mut impl Buf) -> Result<i16, NbtError> {
        if buf.remaining() < 2 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i16()) // big-endian
    }
    fn write_short(buf: &mut impl BufMut, value: i16) {
        buf.put_i16(value);
    }
    fn read_long(buf: &mut impl Buf) -> Result<i64, NbtError> {
        if buf.remaining() < 8 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_i64())
    }
    fn write_long(buf: &mut impl BufMut, value: i64) {
        buf.put_i64(value);
    }
    fn read_float(buf: &mut impl Buf) -> Result<f32, NbtError> {
        if buf.remaining() < 4 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_f32())
    }
    fn write_float(buf: &mut impl BufMut, value: f32) {
        buf.put_f32(value);
    }
    fn read_double(buf: &mut impl Buf) -> Result<f64, NbtError> {
        if buf.remaining() < 8 {
            return Err(NbtError::UnexpectedEof);
        }
        Ok(buf.get_f64())
    }
    fn write_double(buf: &mut impl BufMut, value: f64) {
        buf.put_f64(value);
    }
}
