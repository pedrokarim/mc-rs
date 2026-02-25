//! AddActor (0x0D) — Server → Client.
//!
//! Spawns a non-player entity (mob, item, projectile, etc.) visible to the client.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::packets::add_player::{encode_entity_metadata, EntityMetadataEntry};
use crate::types::{VarLong, VarUInt32, VarUInt64, Vec3};

/// A single attribute sent with AddActor.
pub struct ActorAttribute {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub current: f32,
    pub default: f32,
}

/// Spawn a non-player entity.
pub struct AddActor {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub entity_type: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub body_yaw: f32,
    pub attributes: Vec<ActorAttribute>,
    pub metadata: Vec<EntityMetadataEntry>,
}

impl ProtoEncode for AddActor {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarLong(self.entity_unique_id).proto_encode(buf);
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        write_string(buf, &self.entity_type);
        self.position.proto_encode(buf);
        self.velocity.proto_encode(buf);
        buf.put_f32_le(self.pitch);
        buf.put_f32_le(self.yaw);
        buf.put_f32_le(self.head_yaw);
        buf.put_f32_le(self.body_yaw);
        // Attributes
        VarUInt32(self.attributes.len() as u32).proto_encode(buf);
        for attr in &self.attributes {
            write_string(buf, &attr.name);
            buf.put_f32_le(attr.min);
            buf.put_f32_le(attr.max);
            buf.put_f32_le(attr.current);
            buf.put_f32_le(attr.default);
        }
        // Entity metadata
        encode_entity_metadata(buf, &self.metadata);
        // Entity sync properties (int count + float count)
        VarUInt32(0).proto_encode(buf);
        VarUInt32(0).proto_encode(buf);
        // Entity links
        VarUInt32(0).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::add_player::MetadataValue;
    use bytes::BytesMut;

    #[test]
    fn encode_zombie_basic() {
        let pkt = AddActor {
            entity_unique_id: 10,
            entity_runtime_id: 10,
            entity_type: "minecraft:zombie".to_string(),
            position: Vec3::new(5.0, 4.0, 5.0),
            velocity: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            body_yaw: 0.0,
            attributes: vec![],
            metadata: vec![],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarLong(10) + VarUInt64(10) + string("minecraft:zombie") + Vec3 + Vec3 + 4xf32 + ...
        assert!(buf.len() > 40);
    }

    #[test]
    fn encode_with_health_attribute() {
        let pkt = AddActor {
            entity_unique_id: 5,
            entity_runtime_id: 5,
            entity_type: "minecraft:cow".to_string(),
            position: Vec3::new(0.0, 4.0, 0.0),
            velocity: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            body_yaw: 0.0,
            attributes: vec![ActorAttribute {
                name: "minecraft:health".to_string(),
                min: 0.0,
                max: 10.0,
                current: 10.0,
                default: 10.0,
            }],
            metadata: vec![],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should be larger than basic due to attribute
        assert!(buf.len() > 60);
    }

    #[test]
    fn encode_with_metadata() {
        let pkt = AddActor {
            entity_unique_id: 3,
            entity_runtime_id: 3,
            entity_type: "minecraft:pig".to_string(),
            position: Vec3::new(1.0, 4.0, 1.0),
            velocity: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            body_yaw: 0.0,
            attributes: vec![],
            metadata: vec![
                EntityMetadataEntry {
                    key: 0,
                    data_type: 7,
                    value: MetadataValue::Long(0),
                },
                EntityMetadataEntry {
                    key: 23,
                    data_type: 3,
                    value: MetadataValue::Float(1.0),
                },
            ],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 50);
    }
}
