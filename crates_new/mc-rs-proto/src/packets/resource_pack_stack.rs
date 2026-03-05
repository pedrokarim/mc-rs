use crate::codec::{write_string, write_unsigned_varint32};
use bytes::{BufMut, BytesMut};

/// Encode ResourcePackStackPacket with no packs (PocketMine default).
/// Fields: mustAccept, stackCount(varuint), gameVersion(string),
///         experimentCount(u32_le), hasPreviouslyUsedExperiments(bool),
///         useVanillaEditorPacks(bool)
pub fn encode_empty() -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(0); // mustAccept
    write_unsigned_varint32(&mut buf, 0); // stack pack count
    write_string(&mut buf, "1.26.0"); // baseGameVersion (must match MINECRAFT_VERSION_NETWORK)
    buf.put_u32_le(0); // experiment count (u32_le, NOT varuint)
    buf.put_u8(0); // hasPreviouslyUsedExperiments
    buf.put_u8(0); // useVanillaEditorPacks
    buf
}
