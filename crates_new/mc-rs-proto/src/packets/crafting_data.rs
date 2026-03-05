use crate::codec::write_unsigned_varint32;
use bytes::{BufMut, BytesMut};

/// CraftingDataPacket — empty (no recipes)
pub fn encode_empty() -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varint32(&mut buf, 0); // recipe count
    write_unsigned_varint32(&mut buf, 0); // potion type recipe count
    write_unsigned_varint32(&mut buf, 0); // potion container recipe count
    write_unsigned_varint32(&mut buf, 0); // material reducer recipe count
    buf.put_u8(1); // isClean = true
    buf
}
