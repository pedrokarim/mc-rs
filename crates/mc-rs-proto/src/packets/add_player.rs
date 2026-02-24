//! AddPlayer (0x0C) — Server → Client.
//!
//! Spawns a remote player entity visible to the receiving client.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::item_stack::ItemStack;
use crate::types::{Uuid, VarInt, VarLong, VarUInt32, VarUInt64, Vec3};

/// Entity metadata value types.
pub enum MetadataValue {
    Byte(u8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Long(i64),
}

/// A single entity metadata entry.
pub struct EntityMetadataEntry {
    /// Metadata key (e.g. 0=FLAGS, 4=NAMETAG, 23=SCALE).
    pub key: u32,
    /// Data type ID (0=byte, 1=short, 2=int, 3=float, 4=string, 7=long).
    pub data_type: u32,
    /// The value.
    pub value: MetadataValue,
}

/// Spawn a remote player.
pub struct AddPlayer {
    pub uuid: Uuid,
    pub username: String,
    pub entity_runtime_id: u64,
    pub platform_chat_id: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub held_item: ItemStack,
    pub gamemode: i32,
    pub metadata: Vec<EntityMetadataEntry>,
    // AbilityData
    pub entity_unique_id: i64,
    pub permission_level: u8,
    pub command_permission_level: u8,
    // Device
    pub device_id: String,
    pub device_os: i32,
}

impl ProtoEncode for AddPlayer {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.uuid.proto_encode(buf);
        write_string(buf, &self.username);
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        write_string(buf, &self.platform_chat_id);
        self.position.proto_encode(buf);
        self.velocity.proto_encode(buf);
        // Rotation: pitch, yaw, head_yaw as f32 LE
        buf.put_f32_le(self.pitch);
        buf.put_f32_le(self.yaw);
        buf.put_f32_le(self.head_yaw);
        self.held_item.proto_encode(buf);
        VarInt(self.gamemode).proto_encode(buf);
        // Entity metadata
        encode_entity_metadata(buf, &self.metadata);
        // Ability data (minimal, 1 layer)
        encode_ability_data(buf, self);
        // Entity links (empty)
        VarUInt32(0).proto_encode(buf);
        // Device info
        write_string(buf, &self.device_id);
        buf.put_i32_le(self.device_os);
    }
}

/// Encode entity metadata entries.
fn encode_entity_metadata(buf: &mut impl BufMut, entries: &[EntityMetadataEntry]) {
    VarUInt32(entries.len() as u32).proto_encode(buf);
    for entry in entries {
        VarUInt32(entry.key).proto_encode(buf);
        VarUInt32(entry.data_type).proto_encode(buf);
        match &entry.value {
            MetadataValue::Byte(v) => buf.put_u8(*v),
            MetadataValue::Short(v) => buf.put_i16_le(*v),
            MetadataValue::Int(v) => VarInt(*v).proto_encode(buf),
            MetadataValue::Float(v) => buf.put_f32_le(*v),
            MetadataValue::String(v) => write_string(buf, v),
            MetadataValue::Long(v) => VarLong(*v).proto_encode(buf),
        }
    }
}

/// Encode minimal ability data (one Base layer).
fn encode_ability_data(buf: &mut impl BufMut, player: &AddPlayer) {
    buf.put_u8(player.command_permission_level);
    buf.put_u8(player.permission_level);
    buf.put_i64_le(player.entity_unique_id);
    // 1 ability layer
    VarUInt32(1).proto_encode(buf);
    // Layer type = Base (0)
    buf.put_u16_le(0);
    // Abilities allowed bitmask
    buf.put_u32_le(0x0001_BFFF);
    // Abilities values bitmask (creative vs survival)
    let values = if player.gamemode == 1 {
        0x0000_0477 // creative: fly, instabuild, mayfly, etc.
    } else {
        0x0000_0003 // survival: basic
    };
    buf.put_u32_le(values);
    // Fly speed
    buf.put_f32_le(0.05);
    // Walk speed
    buf.put_f32_le(0.1);
}

/// Build default entity metadata for a player.
pub fn default_player_metadata(display_name: &str) -> Vec<EntityMetadataEntry> {
    vec![
        EntityMetadataEntry {
            key: 0,
            data_type: 7,                  // i64
            value: MetadataValue::Long(0), // FLAGS — no special flags
        },
        EntityMetadataEntry {
            key: 4,
            data_type: 4,                                           // string
            value: MetadataValue::String(display_name.to_string()), // NAMETAG
        },
        EntityMetadataEntry {
            key: 23,
            data_type: 3,                     // f32
            value: MetadataValue::Float(1.0), // SCALE
        },
        EntityMetadataEntry {
            key: 38,
            data_type: 3,                     // f32
            value: MetadataValue::Float(0.6), // BOUNDING_BOX_WIDTH
        },
        EntityMetadataEntry {
            key: 39,
            data_type: 3,                     // f32
            value: MetadataValue::Float(1.8), // BOUNDING_BOX_HEIGHT
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_add_player_basic() {
        let pkt = AddPlayer {
            uuid: Uuid::new(0x0123456789ABCDEF, 0xFEDCBA9876543210),
            username: "Steve".to_string(),
            entity_runtime_id: 2,
            platform_chat_id: String::new(),
            position: Vec3::new(0.0, 64.0, 0.0),
            velocity: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            held_item: ItemStack::empty(),
            gamemode: 1,
            metadata: default_player_metadata("Steve"),
            entity_unique_id: 2,
            permission_level: 1,
            command_permission_level: 0,
            device_id: String::new(),
            device_os: 7,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // UUID (16 bytes) at the start
        assert!(buf.len() > 16);
        // Followed by username "Steve" = VarUInt32(5) + 5 bytes
        assert_eq!(buf[16], 5); // string length
        assert_eq!(&buf[17..22], b"Steve");
    }

    #[test]
    fn default_metadata_has_nametag() {
        let meta = default_player_metadata("Alex");
        assert_eq!(meta.len(), 5);
        // Check nametag entry (key 4)
        let nametag = meta.iter().find(|e| e.key == 4).unwrap();
        assert_eq!(nametag.data_type, 4); // string
        match &nametag.value {
            MetadataValue::String(s) => assert_eq!(s, "Alex"),
            _ => panic!("expected string"),
        }
    }
}
