use crate::codec::*;
use bytes::{BufMut, BytesMut};

/// NetworkChunkPublisherUpdatePacket
/// position: SignedBlockPos (3× SignedVarInt)
/// radius: UnsignedVarInt (in blocks, = viewDistance * 16)
/// savedChunks: u32_le count + entries
pub fn encode(x: i32, y: i32, z: i32, radius_blocks: u32) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, x);
    write_signed_varint32(&mut buf, y);
    write_signed_varint32(&mut buf, z);
    write_unsigned_varint32(&mut buf, radius_blocks);
    buf.put_u32_le(0); // savedChunks count = 0
    buf
}
