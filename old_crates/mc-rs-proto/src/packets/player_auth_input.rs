//! PlayerAuthInput (0x90) — Client → Server.
//!
//! Sent every tick (20/sec) by the client with position, rotation,
//! and input state. This is the primary movement packet under
//! server-authoritative movement (auth_type = 1 or 2).

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::{VarUInt32, VarUInt64, Vec2, Vec3};

/// Bitflags for the `input_data` field of [`PlayerAuthInput`].
pub mod input_flags {
    pub const ASCEND: u64 = 1 << 0;
    pub const DESCEND: u64 = 1 << 1;
    pub const NORTH_JUMP: u64 = 1 << 2;
    pub const JUMP_DOWN: u64 = 1 << 3;
    pub const SPRINT_DOWN: u64 = 1 << 4;
    pub const CHANGE_HEIGHT: u64 = 1 << 5;
    pub const JUMPING: u64 = 1 << 6;
    pub const AUTO_JUMPING_IN_WATER: u64 = 1 << 7;
    pub const SNEAKING: u64 = 1 << 8;
    pub const SNEAK_DOWN: u64 = 1 << 9;
    pub const UP: u64 = 1 << 10;
    pub const DOWN: u64 = 1 << 11;
    pub const LEFT: u64 = 1 << 12;
    pub const RIGHT: u64 = 1 << 13;
    pub const UP_LEFT: u64 = 1 << 14;
    pub const UP_RIGHT: u64 = 1 << 15;
    pub const WANT_UP: u64 = 1 << 16;
    pub const WANT_DOWN: u64 = 1 << 17;
    pub const WANT_DOWN_SLOW: u64 = 1 << 18;
    pub const WANT_UP_SLOW: u64 = 1 << 19;
    pub const SPRINTING: u64 = 1 << 20;
    pub const ASCEND_BLOCK: u64 = 1 << 21;
    pub const DESCEND_BLOCK: u64 = 1 << 22;
    pub const SNEAK_TOGGLE_DOWN: u64 = 1 << 23;
    pub const PERSIST_SNEAK: u64 = 1 << 24;
    pub const START_SWIMMING: u64 = 1 << 25;
    pub const STOP_SWIMMING: u64 = 1 << 26;
    pub const START_SPRINTING: u64 = 1 << 27;
    pub const STOP_SPRINTING: u64 = 1 << 28;
    pub const START_SNEAKING: u64 = 1 << 29;
    pub const STOP_SNEAKING: u64 = 1 << 30;
    pub const START_CRAWLING: u64 = 1 << 31;
    pub const STOP_CRAWLING: u64 = 1 << 32;
    pub const START_FLYING: u64 = 1 << 33;
    pub const STOP_FLYING: u64 = 1 << 34;

    // Conditional sub-packet triggers (not parsed in Phase 1.1):
    pub const PERFORM_ITEM_INTERACTION: u64 = 1 << 35;
    pub const PERFORM_BLOCK_ACTIONS: u64 = 1 << 36;
    pub const PERFORM_ITEM_STACK_REQUEST: u64 = 1 << 37;
}

/// Core fields of the PlayerAuthInput packet.
///
/// We parse only the fixed-layout core fields and stop before the
/// conditional sub-packets (item interaction, block actions, item stack
/// requests). Since each sub-packet in a batch has its own length
/// prefix, partial reads are safe.
#[derive(Debug, Clone)]
pub struct PlayerAuthInput {
    pub pitch: f32,
    pub yaw: f32,
    pub position: Vec3,
    pub move_vector: Vec2,
    pub head_yaw: f32,
    pub input_data: u64,
    pub input_mode: u32,
    pub play_mode: u32,
    pub interaction_model: u32,
    pub tick: u64,
    pub position_delta: Vec3,
}

impl PlayerAuthInput {
    /// Check whether a specific input flag is set.
    pub fn has_flag(&self, flag: u64) -> bool {
        self.input_data & flag != 0
    }
}

impl ProtoDecode for PlayerAuthInput {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 8 {
            return Err(ProtoError::BufferTooShort {
                needed: 8,
                remaining: buf.remaining(),
            });
        }
        let pitch = buf.get_f32_le();
        let yaw = buf.get_f32_le();

        let position = Vec3::proto_decode(buf)?;
        let move_vector = Vec2::proto_decode(buf)?;

        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let head_yaw = buf.get_f32_le();

        let input_data = VarUInt64::proto_decode(buf)?.0;
        let input_mode = VarUInt32::proto_decode(buf)?.0;
        let play_mode = VarUInt32::proto_decode(buf)?.0;
        let interaction_model = VarUInt32::proto_decode(buf)?.0;

        // PlayMode 5 = VR: skip GazeDirection Vec3
        if play_mode == 5 {
            let _gaze = Vec3::proto_decode(buf)?;
        }

        let tick = VarUInt64::proto_decode(buf)?.0;
        let position_delta = Vec3::proto_decode(buf)?;

        // Stop here — conditional sub-packets are not parsed.

        Ok(Self {
            pitch,
            yaw,
            position,
            move_vector,
            head_yaw,
            input_data,
            input_mode,
            play_mode,
            interaction_model,
            tick,
            position_delta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoEncode;
    use bytes::{BufMut, BytesMut};

    /// Encode a PlayerAuthInput into raw bytes (wire format).
    fn encode_test_input(pkt: &PlayerAuthInput) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_f32_le(pkt.pitch);
        buf.put_f32_le(pkt.yaw);
        pkt.position.proto_encode(&mut buf);
        pkt.move_vector.proto_encode(&mut buf);
        buf.put_f32_le(pkt.head_yaw);
        VarUInt64(pkt.input_data).proto_encode(&mut buf);
        VarUInt32(pkt.input_mode).proto_encode(&mut buf);
        VarUInt32(pkt.play_mode).proto_encode(&mut buf);
        VarUInt32(pkt.interaction_model).proto_encode(&mut buf);
        VarUInt64(pkt.tick).proto_encode(&mut buf);
        pkt.position_delta.proto_encode(&mut buf);
        buf
    }

    fn default_input() -> PlayerAuthInput {
        PlayerAuthInput {
            pitch: 0.0,
            yaw: 0.0,
            position: Vec3::ZERO,
            move_vector: Vec2::ZERO,
            head_yaw: 0.0,
            input_data: 0,
            input_mode: 0,
            play_mode: 0,
            interaction_model: 0,
            tick: 0,
            position_delta: Vec3::ZERO,
        }
    }

    #[test]
    fn decode_stationary_player() {
        let input = PlayerAuthInput {
            yaw: 90.0,
            position: Vec3::new(0.5, 5.62, 0.5),
            head_yaw: 90.0,
            tick: 1,
            ..default_input()
        };
        let buf = encode_test_input(&input);
        let pkt = PlayerAuthInput::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.pitch, 0.0);
        assert_eq!(pkt.yaw, 90.0);
        assert_eq!(pkt.position, Vec3::new(0.5, 5.62, 0.5));
        assert_eq!(pkt.move_vector, Vec2::ZERO);
        assert_eq!(pkt.head_yaw, 90.0);
        assert_eq!(pkt.input_data, 0);
        assert_eq!(pkt.tick, 1);
        assert_eq!(pkt.position_delta, Vec3::ZERO);
    }

    #[test]
    fn decode_walking_forward() {
        let flags = input_flags::UP | input_flags::SPRINTING;
        let input = PlayerAuthInput {
            pitch: -5.0,
            yaw: 180.0,
            position: Vec3::new(10.0, 5.62, 20.0),
            move_vector: Vec2::new(0.0, 1.0),
            head_yaw: 180.0,
            input_data: flags,
            tick: 100,
            position_delta: Vec3::new(0.0, 0.0, 0.2),
            ..default_input()
        };
        let buf = encode_test_input(&input);
        let pkt = PlayerAuthInput::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.position, Vec3::new(10.0, 5.62, 20.0));
        assert!(pkt.has_flag(input_flags::UP));
        assert!(pkt.has_flag(input_flags::SPRINTING));
        assert!(!pkt.has_flag(input_flags::SNEAKING));
        assert_eq!(pkt.tick, 100);
    }

    #[test]
    fn decode_with_jump_flag() {
        let flags = input_flags::JUMPING | input_flags::JUMP_DOWN;
        let input = PlayerAuthInput {
            position: Vec3::new(0.0, 6.0, 0.0),
            input_data: flags,
            tick: 50,
            position_delta: Vec3::new(0.0, 0.42, 0.0),
            ..default_input()
        };
        let buf = encode_test_input(&input);
        let pkt = PlayerAuthInput::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert!(pkt.has_flag(input_flags::JUMPING));
        assert!(pkt.has_flag(input_flags::JUMP_DOWN));
    }

    #[test]
    fn decode_vr_mode_skips_gaze() {
        let mut buf = BytesMut::new();
        buf.put_f32_le(0.0);
        buf.put_f32_le(0.0);
        Vec3::ZERO.proto_encode(&mut buf);
        Vec2::ZERO.proto_encode(&mut buf);
        buf.put_f32_le(0.0);
        VarUInt64(0).proto_encode(&mut buf);
        VarUInt32(0).proto_encode(&mut buf);
        VarUInt32(5).proto_encode(&mut buf); // PlayMode = VR
        VarUInt32(0).proto_encode(&mut buf);
        Vec3::new(0.0, 0.0, 1.0).proto_encode(&mut buf); // GazeDirection
        VarUInt64(42).proto_encode(&mut buf);
        Vec3::ZERO.proto_encode(&mut buf);

        let pkt = PlayerAuthInput::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.play_mode, 5);
        assert_eq!(pkt.tick, 42);
    }

    #[test]
    fn has_flag_helper() {
        let pkt = PlayerAuthInput {
            pitch: 0.0,
            yaw: 0.0,
            position: Vec3::ZERO,
            move_vector: Vec2::ZERO,
            head_yaw: 0.0,
            input_data: input_flags::SNEAKING | input_flags::SPRINTING,
            input_mode: 0,
            play_mode: 0,
            interaction_model: 0,
            tick: 0,
            position_delta: Vec3::ZERO,
        };
        assert!(pkt.has_flag(input_flags::SNEAKING));
        assert!(pkt.has_flag(input_flags::SPRINTING));
        assert!(!pkt.has_flag(input_flags::JUMPING));
    }

    #[test]
    fn decode_buffer_too_short() {
        let mut buf = BytesMut::new();
        buf.put_f32_le(0.0);
        assert!(PlayerAuthInput::proto_decode(&mut buf.freeze().as_ref()).is_err());
    }
}
