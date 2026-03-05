use crate::codec::write_string;
use bytes::{BufMut, BytesMut};

/// Encode ResourcePacksInfoPacket with no packs (PocketMine default).
/// Fields: mustAccept, hasAddons, hasScripts, forceDisableVibrantVisuals,
///         worldTemplateUUID(16B), worldTemplateVersion(string), packs(u16_le count)
pub fn encode_empty() -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(0); // mustAccept
    buf.put_u8(0); // hasAddons
    buf.put_u8(0); // hasScripts
    buf.put_u8(0); // forceDisableVibrantVisuals
    buf.put_slice(&[0u8; 16]); // worldTemplateUUID = NIL
    write_string(&mut buf, ""); // worldTemplateVersion
    buf.put_u16_le(0); // pack count (u16_le, NOT varuint)
    buf
}
