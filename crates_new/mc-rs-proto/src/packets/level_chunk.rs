use crate::codec::*;
use bytes::{BufMut, BytesMut};

/// LevelChunkPacket
/// chunkX: SignedVarInt, chunkZ: SignedVarInt, dimensionId: SignedVarInt,
/// subChunkCount: UnsignedVarInt, cacheEnabled: bool,
/// payload: UnsignedVarInt(len) + bytes
pub fn encode(
    chunk_x: i32,
    chunk_z: i32,
    dimension_id: i32,
    sub_chunk_count: u32,
    payload: &[u8],
) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, chunk_x);
    write_signed_varint32(&mut buf, chunk_z);
    write_signed_varint32(&mut buf, dimension_id);
    write_unsigned_varint32(&mut buf, sub_chunk_count);
    buf.put_u8(0); // cacheEnabled = false
    write_unsigned_varint32(&mut buf, payload.len() as u32);
    buf.extend_from_slice(payload);
    buf
}
