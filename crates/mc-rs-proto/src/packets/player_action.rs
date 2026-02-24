//! PlayerAction (0x24) — Client → Server.
//!
//! Sent when the player performs various actions: start/stop mining, etc.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::{BlockPos, VarInt, VarUInt64};

/// Player action types relevant for mining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerActionType {
    StartBreak,
    AbortBreak,
    StopBreak,
    PredictDestroyBlock,
    ContinueDestroyBlock,
    /// Any action type we don't handle specifically.
    Other(i32),
}

impl PlayerActionType {
    fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::StartBreak,
            1 => Self::AbortBreak,
            2 => Self::StopBreak,
            22 => Self::PredictDestroyBlock,
            23 => Self::ContinueDestroyBlock,
            other => Self::Other(other),
        }
    }
}

/// PlayerAction packet fields.
#[derive(Debug, Clone)]
pub struct PlayerAction {
    pub entity_runtime_id: u64,
    pub action: PlayerActionType,
    pub block_position: BlockPos,
    pub result_position: BlockPos,
    pub face: i32,
}

impl ProtoDecode for PlayerAction {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let entity_runtime_id = VarUInt64::proto_decode(buf)?.0;
        let action_raw = VarInt::proto_decode(buf)?.0;
        let action = PlayerActionType::from_i32(action_raw);
        let block_position = BlockPos::proto_decode(buf)?;
        let result_position = BlockPos::proto_decode(buf)?;
        let face = VarInt::proto_decode(buf)?.0;

        Ok(Self {
            entity_runtime_id,
            action,
            block_position,
            result_position,
            face,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use std::io::Cursor;

    use crate::codec::ProtoEncode;
    use crate::types::VarUInt64;

    fn encode_action(action: i32, pos: BlockPos) -> BytesMut {
        let mut buf = BytesMut::new();
        VarUInt64(1).proto_encode(&mut buf); // entity_runtime_id
        VarInt(action).proto_encode(&mut buf); // action
        pos.proto_encode(&mut buf); // block_position
        BlockPos::new(0, 0, 0).proto_encode(&mut buf); // result_position
        VarInt(1).proto_encode(&mut buf); // face
        buf
    }

    #[test]
    fn decode_start_break() {
        let data = encode_action(0, BlockPos::new(10, 64, -5));
        let mut cursor = Cursor::new(&data[..]);
        let action = PlayerAction::proto_decode(&mut cursor).unwrap();
        assert_eq!(action.entity_runtime_id, 1);
        assert_eq!(action.action, PlayerActionType::StartBreak);
        assert_eq!(action.block_position.x, 10);
        assert_eq!(action.block_position.y, 64);
        assert_eq!(action.block_position.z, -5);
        assert_eq!(action.face, 1);
    }

    #[test]
    fn decode_abort_break() {
        let data = encode_action(1, BlockPos::new(0, 0, 0));
        let mut cursor = Cursor::new(&data[..]);
        let action = PlayerAction::proto_decode(&mut cursor).unwrap();
        assert_eq!(action.action, PlayerActionType::AbortBreak);
    }

    #[test]
    fn decode_predict_destroy() {
        let data = encode_action(22, BlockPos::new(5, 3, 5));
        let mut cursor = Cursor::new(&data[..]);
        let action = PlayerAction::proto_decode(&mut cursor).unwrap();
        assert_eq!(action.action, PlayerActionType::PredictDestroyBlock);
    }

    #[test]
    fn decode_unknown_action() {
        let data = encode_action(99, BlockPos::new(0, 0, 0));
        let mut cursor = Cursor::new(&data[..]);
        let action = PlayerAction::proto_decode(&mut cursor).unwrap();
        assert_eq!(action.action, PlayerActionType::Other(99));
    }

    #[test]
    fn decode_buffer_too_short() {
        let data = [0x01]; // Not enough data
        let mut cursor = Cursor::new(&data[..]);
        assert!(PlayerAction::proto_decode(&mut cursor).is_err());
    }
}
