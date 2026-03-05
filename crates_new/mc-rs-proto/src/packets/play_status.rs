use bytes::{BufMut, BytesMut};

pub const LOGIN_SUCCESS: i32 = 0;
pub const LOGIN_FAILED_CLIENT: i32 = 1;
pub const LOGIN_FAILED_SERVER: i32 = 2;
pub const PLAYER_SPAWN: i32 = 3;
pub const LOGIN_FAILED_INVALID_TENANT: i32 = 4;
pub const LOGIN_FAILED_EDITION_MISMATCH_EDU_TO_VANILLA: i32 = 5;
pub const LOGIN_FAILED_EDITION_MISMATCH_VANILLA_TO_EDU: i32 = 6;
pub const LOGIN_FAILED_SERVER_FULL: i32 = 7;

pub fn encode(status: i32) -> BytesMut {
    let mut buf = BytesMut::with_capacity(4);
    buf.put_i32(status); // i32 BE (PocketMine uses putInt = BE)
    buf
}
