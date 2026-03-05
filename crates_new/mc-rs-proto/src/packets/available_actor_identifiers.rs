use bytes::BytesMut;

const CANONICAL_DATA: &[u8] = include_bytes!("../../data/entity_identifiers.nbt");

/// Encode AvailableActorIdentifiersPacket — raw network NBT blob
pub fn encode() -> BytesMut {
    BytesMut::from(CANONICAL_DATA)
}
