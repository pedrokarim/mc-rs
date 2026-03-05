//! Network NBT variant (VarInt for ints/lengths, VarUInt32 for string lengths).

use bytes::{Buf, BufMut};
use mc_rs_proto::codec::{ProtoDecode, ProtoEncode};
use mc_rs_proto::types::{VarInt, VarUInt32};

use crate::error::NbtError;
use crate::io::NbtVariant;

pub(crate) struct NetworkVariant;

impl NbtVariant for NetworkVariant {
    fn write_int(buf: &mut impl BufMut, value: i32) {
        VarInt(value).proto_encode(buf);
    }

    fn read_int(buf: &mut impl Buf) -> Result<i32, NbtError> {
        VarInt::proto_decode(buf)
            .map(|v| v.0)
            .map_err(|e| NbtError::VarInt(e.to_string()))
    }

    fn write_array_len(buf: &mut impl BufMut, len: i32) {
        VarInt(len).proto_encode(buf);
    }

    fn read_array_len(buf: &mut impl Buf) -> Result<i32, NbtError> {
        VarInt::proto_decode(buf)
            .map(|v| v.0)
            .map_err(|e| NbtError::VarInt(e.to_string()))
    }

    fn write_string_len(buf: &mut impl BufMut, len: usize) {
        VarUInt32(len as u32).proto_encode(buf);
    }

    fn read_string_len(buf: &mut impl Buf) -> Result<usize, NbtError> {
        VarUInt32::proto_decode(buf)
            .map(|v| v.0 as usize)
            .map_err(|e| NbtError::VarInt(e.to_string()))
    }
}
