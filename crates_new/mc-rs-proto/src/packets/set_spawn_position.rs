use bytes::BytesMut;

use crate::codec::{write_block_pos, write_signed_varint32};

pub const TYPE_PLAYER_SPAWN: i32 = 0;
pub const TYPE_WORLD_SPAWN: i32 = 1;

/// SetSpawnPosition (0x2B)
/// spawn_type, spawn_position, dimension, causing_block_position
pub fn encode(
    spawn_type: i32,
    spawn_x: i32,
    spawn_y: u32,
    spawn_z: i32,
    dimension: i32,
    cause_x: i32,
    cause_y: u32,
    cause_z: i32,
) -> BytesMut {
    let mut buf = BytesMut::new();
    write_signed_varint32(&mut buf, spawn_type);
    write_block_pos(&mut buf, spawn_x, spawn_y, spawn_z);
    write_signed_varint32(&mut buf, dimension);
    write_block_pos(&mut buf, cause_x, cause_y, cause_z);
    buf
}

/// PMMP-compatible world spawn helper.
/// causing block is INT32_MIN triplet.
pub fn encode_world_spawn(spawn_x: i32, spawn_y: u32, spawn_z: i32, dimension: i32) -> BytesMut {
    encode(
        TYPE_WORLD_SPAWN,
        spawn_x,
        spawn_y,
        spawn_z,
        dimension,
        i32::MIN,
        i32::MIN as u32,
        i32::MIN,
    )
}

/// Player spawn helper.
pub fn encode_player_spawn(spawn_x: i32, spawn_y: u32, spawn_z: i32, dimension: i32) -> BytesMut {
    encode(
        TYPE_PLAYER_SPAWN,
        spawn_x,
        spawn_y,
        spawn_z,
        dimension,
        spawn_x,
        spawn_y,
        spawn_z,
    )
}
