use crate::codec::*;
use bytes::{BufMut, BytesMut};

// Metadata key IDs (Bedrock protocol)
const DATA_FLAGS: u32 = 0;
const DATA_AIR_SUPPLY: u32 = 7;
const DATA_MAX_AIR_SUPPLY: u32 = 44;
const DATA_PLAYER_FLAG_REGISTER: u32 = 26;
const DATA_BED_POSITION: u32 = 29;
const DATA_LEAD_HOLDER_EID: u32 = 38;

// Metadata types
const TYPE_LONG: u32 = 7;
const TYPE_SHORT: u32 = 1;
const TYPE_VEC3I: u32 = 8;

// Actor flags (bit positions in DATA_FLAGS and PLAYER_FLAG_REGISTER)
const FLAG_BREATHING: u64 = 1 << 35;
const FLAG_HAS_GRAVITY: u64 = 1 << 34;

/// SetActorDataPacket — minimal version for player spawn
pub fn encode_player_default(actor_runtime_id: u64) -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varlong(&mut buf, actor_runtime_id);

    // Metadata entries
    let entry_count = 5u32;
    write_unsigned_varint32(&mut buf, entry_count);

    // DATA_FLAGS (key=0, type=Long)
    write_unsigned_varint32(&mut buf, DATA_FLAGS);
    write_unsigned_varint32(&mut buf, TYPE_LONG);
    write_signed_varlong(&mut buf, (FLAG_BREATHING | FLAG_HAS_GRAVITY) as i64);

    // DATA_AIR_SUPPLY (key=7, type=Short)
    write_unsigned_varint32(&mut buf, DATA_AIR_SUPPLY);
    write_unsigned_varint32(&mut buf, TYPE_SHORT);
    buf.put_i16_le(400);

    // DATA_MAX_AIR_SUPPLY (key=44, type=Short)
    write_unsigned_varint32(&mut buf, DATA_MAX_AIR_SUPPLY);
    write_unsigned_varint32(&mut buf, TYPE_SHORT);
    buf.put_i16_le(400);

    // DATA_PLAYER_FLAG_REGISTER (key=26, type=Long)
    write_unsigned_varint32(&mut buf, DATA_PLAYER_FLAG_REGISTER);
    write_unsigned_varint32(&mut buf, TYPE_LONG);
    write_signed_varlong(&mut buf, 0);

    // DATA_LEAD_HOLDER_EID (key=38, type=Long)
    write_unsigned_varint32(&mut buf, DATA_LEAD_HOLDER_EID);
    write_unsigned_varint32(&mut buf, TYPE_LONG);
    write_signed_varlong(&mut buf, -1);

    // Properties (u32 count × 2 = int + float properties)
    write_unsigned_varint32(&mut buf, 0); // int properties count
    write_unsigned_varint32(&mut buf, 0); // float properties count

    // Tick
    write_unsigned_varlong(&mut buf, 0);

    buf
}
