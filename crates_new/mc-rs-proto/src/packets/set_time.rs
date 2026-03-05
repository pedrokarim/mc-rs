use crate::codec::write_signed_varint32;
use bytes::BytesMut;

/// SetTimePacket — SignedVarInt(time)
pub fn encode(time: i32) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, time);
    buf
}
