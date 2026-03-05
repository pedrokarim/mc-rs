use bytes::{BufMut, BytesMut};

/// UpdateAdventureSettingsPacket
/// All flags are booleans
pub fn encode_default() -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(0); // noPvM (no player-vs-mobs)
    buf.put_u8(0); // noMvP (no mobs-vs-player)
    buf.put_u8(0); // immutableWorld
    buf.put_u8(1); // showNameTags (PMMP default)
    buf.put_u8(1); // autoJump
    buf
}
