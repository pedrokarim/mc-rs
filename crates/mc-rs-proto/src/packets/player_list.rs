//! PlayerList (0x3F) — Server → Client.
//!
//! Manages the player tab list: add or remove entries.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::jwt::{ClientData, SkinImage};
use crate::types::{Uuid, VarLong, VarUInt32};

/// A single "Add" entry for the player list.
pub struct PlayerListAdd {
    pub uuid: Uuid,
    pub entity_unique_id: i64,
    pub username: String,
    pub xuid: String,
    pub platform_chat_id: String,
    pub device_os: i32,
    pub skin_data: ClientData,
    pub is_teacher: bool,
    pub is_host: bool,
    pub is_sub_client: bool,
}

/// PlayerList packet — Add action (type 0).
pub struct PlayerListAddPacket {
    pub entries: Vec<PlayerListAdd>,
}

/// PlayerList packet — Remove action (type 1).
pub struct PlayerListRemove {
    pub uuids: Vec<Uuid>,
}

impl ProtoEncode for PlayerListAddPacket {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(0); // action = Add
        VarUInt32(self.entries.len() as u32).proto_encode(buf);
        for entry in &self.entries {
            entry.uuid.proto_encode(buf);
            VarLong(entry.entity_unique_id).proto_encode(buf);
            write_string(buf, &entry.username);
            write_string(buf, &entry.xuid);
            write_string(buf, &entry.platform_chat_id);
            buf.put_i32_le(entry.device_os);
            encode_skin_data(buf, &entry.skin_data);
            buf.put_u8(entry.is_teacher as u8);
            buf.put_u8(entry.is_host as u8);
            buf.put_u8(entry.is_sub_client as u8);
        }
        // Verified entries — one bool per entry, after all entries
        for _ in &self.entries {
            buf.put_u8(1); // is_skin_verified = true
        }
    }
}

impl ProtoEncode for PlayerListRemove {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(1); // action = Remove
        VarUInt32(self.uuids.len() as u32).proto_encode(buf);
        for uuid in &self.uuids {
            uuid.proto_encode(buf);
        }
    }
}

/// Encode a skin image (width, height, data with VarUInt32 length).
fn encode_skin_image(buf: &mut impl BufMut, image: &SkinImage) {
    buf.put_i32_le(image.width as i32);
    buf.put_i32_le(image.height as i32);
    VarUInt32(image.data.len() as u32).proto_encode(buf);
    buf.put_slice(&image.data);
}

/// Encode the full skin data block within a PlayerList Add entry.
fn encode_skin_data(buf: &mut impl BufMut, data: &ClientData) {
    write_string(buf, &data.skin_id);
    write_string(buf, &data.play_fab_id);
    write_string(buf, &data.skin_resource_patch);
    // Skin image
    encode_skin_image(buf, &data.skin_image);
    // Animations (empty array)
    buf.put_i32_le(0);
    // Cape image
    encode_skin_image(buf, &data.cape_image);
    // Geometry data
    write_string(buf, &data.skin_geometry_data);
    // Geometry data engine version
    write_string(buf, "");
    // Animation data
    write_string(buf, "");
    // Cape ID
    write_string(buf, &data.cape_id);
    // Full skin ID (composite: skin_id + "_" + cape_id)
    let full_skin_id = if data.cape_id.is_empty() {
        data.skin_id.clone()
    } else {
        format!("{}_{}", data.skin_id, data.cape_id)
    };
    write_string(buf, &full_skin_id);
    // Arm size
    write_string(buf, &data.arm_size);
    // Skin color
    write_string(buf, &data.skin_color);
    // Persona pieces (empty)
    buf.put_i32_le(0);
    // Piece tint colors (empty)
    buf.put_i32_le(0);
    // Is premium skin
    buf.put_u8(0);
    // Is persona skin
    buf.put_u8(data.persona_skin as u8);
    // Is persona cape on classic skin
    buf.put_u8(0);
    // Is primary user
    buf.put_u8(1);
    // Override appearance
    buf.put_u8(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_remove_single() {
        let uuid = Uuid::new(1, 2);
        let pkt = PlayerListRemove { uuids: vec![uuid] };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // action(1) + VarUInt32(1) + UUID(16)
        assert_eq!(buf[0], 1); // action = Remove
        assert_eq!(buf[1], 1); // count = 1
        assert_eq!(buf.len(), 1 + 1 + 16);
    }

    #[test]
    fn encode_add_single() {
        let pkt = PlayerListAddPacket {
            entries: vec![PlayerListAdd {
                uuid: Uuid::new(1, 2),
                entity_unique_id: 1,
                username: "Steve".into(),
                xuid: "".into(),
                platform_chat_id: "".into(),
                device_os: 7,
                skin_data: ClientData::default(),
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should start with action=0, count=1
        assert_eq!(buf[0], 0); // action = Add
        assert_eq!(buf[1], 1); // count = 1 (VarUInt32)
                               // Should have some substantial length due to skin data
        assert!(buf.len() > 50);
    }

    #[test]
    fn encode_add_empty_skin() {
        // Verify encoding with default (empty) skin doesn't panic
        let pkt = PlayerListAddPacket {
            entries: vec![PlayerListAdd {
                uuid: Uuid::ZERO,
                entity_unique_id: 0,
                username: "".into(),
                xuid: "".into(),
                platform_chat_id: "".into(),
                device_os: 0,
                skin_data: ClientData::default(),
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
    }
}
