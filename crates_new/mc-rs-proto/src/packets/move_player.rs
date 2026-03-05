use bytes::{BufMut, BytesMut};

use crate::codec::{write_unsigned_varlong, write_vec3f};

/// MovePlayer mode values (subset).
pub const MODE_NORMAL: u8 = 0;
pub const MODE_RESET: u8 = 1;

pub fn encode(
    runtime_entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
    pitch: f32,
    yaw: f32,
    head_yaw: f32,
    mode: u8,
    on_ground: bool,
    tick: u64,
) -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varlong(&mut buf, runtime_entity_id);
    write_vec3f(&mut buf, x, y, z);
    buf.put_f32_le(pitch);
    buf.put_f32_le(yaw);
    buf.put_f32_le(head_yaw);
    buf.put_u8(mode);
    buf.put_u8(on_ground as u8);
    write_unsigned_varlong(&mut buf, 0); // ridden_entity_runtime_id
    write_unsigned_varlong(&mut buf, tick);
    buf
}

/// MovePlayer (0x13) with Reset mode for server-authoritative correction.
pub fn encode_reset(
    runtime_entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
    pitch: f32,
    yaw: f32,
    head_yaw: f32,
    on_ground: bool,
    tick: u64,
) -> BytesMut {
    encode(
        runtime_entity_id,
        x,
        y,
        z,
        pitch,
        yaw,
        head_yaw,
        MODE_RESET,
        on_ground,
        tick,
    )
}
