use bytes::BytesMut;

use crate::codec::write_unsigned_varint32;

/// SetDifficulty (0x3C): UnsignedVarInt difficulty.
pub fn encode(difficulty: u32) -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varint32(&mut buf, difficulty);
    buf
}
