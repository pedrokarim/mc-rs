//! UpdateAbilities (0xBB) — Server → Client.
//!
//! Updates the player's ability flags (fly, instabuild, etc.).
//! Sent when gamemode changes or when operator status changes.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarUInt32;

/// Update a player's ability data.
pub struct UpdateAbilities {
    pub command_permission_level: u8,
    pub permission_level: u8,
    pub entity_unique_id: i64,
    /// Used to compute ability values bitmask.
    pub gamemode: i32,
}

impl ProtoEncode for UpdateAbilities {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.command_permission_level);
        buf.put_u8(self.permission_level);
        buf.put_i64_le(self.entity_unique_id);
        // 1 ability layer
        VarUInt32(1).proto_encode(buf);
        // Layer type = Base (0)
        buf.put_u16_le(0);
        // Abilities allowed bitmask
        buf.put_u32_le(0x0001_BFFF);
        // Abilities values bitmask
        let values = match self.gamemode {
            1 | 3 => 0x0000_0477, // creative/spectator: fly, instabuild, mayfly
            _ => 0x0000_0003,     // survival/adventure: basic
        };
        buf.put_u32_le(values);
        // Fly speed
        buf.put_f32_le(0.05);
        // Walk speed
        buf.put_f32_le(0.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_creative_abilities() {
        let pkt = UpdateAbilities {
            command_permission_level: 0,
            permission_level: 1,
            entity_unique_id: 1,
            gamemode: 1,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // cmd_perm(1) + perm(1) + i64(8) + VarUInt32(1)(1) + u16(2) + u32(4) + u32(4) + f32(4) + f32(4)
        assert_eq!(buf.len(), 29);
        // Check ability values contain creative flag
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        assert_eq!(values, 0x0000_0477);
    }

    #[test]
    fn encode_survival_abilities() {
        let pkt = UpdateAbilities {
            command_permission_level: 0,
            permission_level: 1,
            entity_unique_id: 1,
            gamemode: 0,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let values = u32::from_le_bytes([buf[17], buf[18], buf[19], buf[20]]);
        assert_eq!(values, 0x0000_0003);
    }
}
