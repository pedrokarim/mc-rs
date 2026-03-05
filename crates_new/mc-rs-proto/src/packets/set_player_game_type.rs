use crate::codec::write_signed_varint32;
use bytes::BytesMut;

/// SetPlayerGameTypePacket
/// Payload: SignedVarInt gamemode
pub fn encode(gamemode: i32) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, gamemode);
    buf
}
