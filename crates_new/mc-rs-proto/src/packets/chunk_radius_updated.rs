use crate::codec::write_signed_varint32;
use bytes::BytesMut;

/// ChunkRadiusUpdatedPacket — SignedVarInt(chunkRadius)
pub fn encode(radius: i32) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, radius);
    buf
}
