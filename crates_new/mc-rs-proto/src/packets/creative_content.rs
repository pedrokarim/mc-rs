use crate::codec::write_unsigned_varint32;
use bytes::BytesMut;

/// CreativeContentPacket — empty (no creative items)
pub fn encode_empty() -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varint32(&mut buf, 0); // count
    buf
}
