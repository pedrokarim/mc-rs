use crate::codec::*;
use bytes::{Buf, BufMut};

/// Signed VarInt (zigzag encoded i32)
pub struct VarInt(pub i32);

impl ProtoEncode for VarInt {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        write_signed_varint32(buf, self.0);
    }
}

impl ProtoDecode for VarInt {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, CodecError> {
        read_signed_varint32(buf).map(VarInt)
    }
}

/// Unsigned VarInt (u32)
pub struct VarUInt32(pub u32);

impl ProtoEncode for VarUInt32 {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        write_unsigned_varint32(buf, self.0);
    }
}

impl ProtoDecode for VarUInt32 {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, CodecError> {
        read_unsigned_varint32(buf).map(VarUInt32)
    }
}
