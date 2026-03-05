use crate::codec::write_unsigned_varint32;
use bytes::{BufMut, BytesMut};

/// AvailableCommandsPacket — minimal empty version
pub fn encode_empty() -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varint32(&mut buf, 0); // enumValuesCount
    write_unsigned_varint32(&mut buf, 0); // chainedSubcommandValuesCount
    write_unsigned_varint32(&mut buf, 0); // suffixesCount
    write_unsigned_varint32(&mut buf, 0); // enumsCount
    write_unsigned_varint32(&mut buf, 0); // chainedSubcommandsCount
    write_unsigned_varint32(&mut buf, 0); // commandDataCount
    write_unsigned_varint32(&mut buf, 0); // softEnumsCount
    write_unsigned_varint32(&mut buf, 0); // constraintsCount
    buf
}
