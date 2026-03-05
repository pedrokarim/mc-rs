use bytes::{BufMut, BytesMut};

pub const COMPRESSION_ZLIB: u16 = 0;
pub const COMPRESSION_SNAPPY: u16 = 1;

pub fn encode(
    compression_threshold: u16,
    compression_algorithm: u16,
    client_throttle_enabled: bool,
    client_throttle_threshold: u8,
    client_throttle_scalar: f32,
) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u16_le(compression_threshold);
    buf.put_u16_le(compression_algorithm);
    buf.put_u8(client_throttle_enabled as u8);
    buf.put_u8(client_throttle_threshold);
    buf.put_f32_le(client_throttle_scalar);
    buf
}

pub fn encode_default() -> BytesMut {
    encode(256, COMPRESSION_ZLIB, false, 0, 0.0)
}
