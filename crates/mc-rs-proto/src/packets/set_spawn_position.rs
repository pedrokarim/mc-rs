//! SetSpawnPosition (0x2B) — Server → Client.
//!
//! Synchronizes world/player spawn positions.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{BlockPos, VarInt};

pub const TYPE_PLAYER_SPAWN: i32 = 0;
pub const TYPE_WORLD_SPAWN: i32 = 1;

/// SetSpawnPosition packet.
pub struct SetSpawnPosition {
    /// 0 = player spawn, 1 = world spawn.
    pub spawn_type: i32,
    /// Target spawn position.
    pub spawn_position: BlockPos,
    /// Dimension ID (0=overworld, 1=nether, 2=end).
    pub dimension: i32,
    /// Respawn anchor / bed causing block position.
    pub causing_block_position: BlockPos,
}

impl SetSpawnPosition {
    pub fn world_spawn(spawn_position: BlockPos, dimension: i32) -> Self {
        Self {
            spawn_type: TYPE_WORLD_SPAWN,
            spawn_position,
            dimension,
            // PMMP uses INT32_MIN triplet for "no causing block" in world-spawn mode.
            causing_block_position: BlockPos::new(i32::MIN, i32::MIN, i32::MIN),
        }
    }
}

impl ProtoEncode for SetSpawnPosition {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.spawn_type).proto_encode(buf);
        self.spawn_position.proto_encode(buf);
        VarInt(self.dimension).proto_encode(buf);
        self.causing_block_position.proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_world_spawn_starts_with_type_1() {
        let pkt = SetSpawnPosition::world_spawn(BlockPos::new(0, 4, 0), 0);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x02); // VarInt(1) zigzag
    }

    #[test]
    fn encode_player_spawn_type_0() {
        let pkt = SetSpawnPosition {
            spawn_type: TYPE_PLAYER_SPAWN,
            spawn_position: BlockPos::new(10, 64, 10),
            dimension: 0,
            causing_block_position: BlockPos::new(10, 64, 10),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x00); // VarInt(0)
    }
}
