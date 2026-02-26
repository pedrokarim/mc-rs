//! UpdateAttributes (0x1D) — Server → Client.
//!
//! Syncs entity attributes (health, movement speed, etc.) to the client.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::{VarUInt32, VarUInt64};

/// A single attribute entry.
pub struct AttributeEntry {
    pub min: f32,
    pub max: f32,
    pub current: f32,
    pub default: f32,
    pub name: String,
}

/// UpdateAttributes packet.
pub struct UpdateAttributes {
    pub entity_runtime_id: u64,
    pub attributes: Vec<AttributeEntry>,
    pub tick: u64,
}

impl UpdateAttributes {
    /// Create a health-only UpdateAttributes packet.
    pub fn health(entity_runtime_id: u64, current: f32, tick: u64) -> Self {
        Self {
            entity_runtime_id,
            attributes: vec![AttributeEntry {
                min: 0.0,
                max: 20.0,
                current,
                default: 20.0,
                name: "minecraft:health".to_string(),
            }],
            tick,
        }
    }

    /// Create a hunger-only UpdateAttributes packet (food + saturation + exhaustion).
    pub fn hunger(
        entity_runtime_id: u64,
        food: f32,
        saturation: f32,
        exhaustion: f32,
        tick: u64,
    ) -> Self {
        Self {
            entity_runtime_id,
            attributes: vec![
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: food,
                    default: 20.0,
                    name: "minecraft:player.hunger".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: saturation,
                    default: 5.0,
                    name: "minecraft:player.saturation".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 5.0,
                    current: exhaustion,
                    default: 0.0,
                    name: "minecraft:player.exhaustion".to_string(),
                },
            ],
            tick,
        }
    }

    /// Create a combined health + hunger UpdateAttributes packet.
    pub fn health_and_hunger(
        entity_runtime_id: u64,
        health: f32,
        food: f32,
        saturation: f32,
        exhaustion: f32,
        tick: u64,
    ) -> Self {
        Self {
            entity_runtime_id,
            attributes: vec![
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: health,
                    default: 20.0,
                    name: "minecraft:health".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: food,
                    default: 20.0,
                    name: "minecraft:player.hunger".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: saturation,
                    default: 5.0,
                    name: "minecraft:player.saturation".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 5.0,
                    current: exhaustion,
                    default: 0.0,
                    name: "minecraft:player.exhaustion".to_string(),
                },
            ],
            tick,
        }
    }
    /// Create an XP-only UpdateAttributes packet (level + progress).
    pub fn xp(entity_runtime_id: u64, level: i32, progress: f32, tick: u64) -> Self {
        Self {
            entity_runtime_id,
            attributes: vec![
                AttributeEntry {
                    min: 0.0,
                    max: 24791.0,
                    current: level as f32,
                    default: 0.0,
                    name: "minecraft:player.level".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 1.0,
                    current: progress,
                    default: 0.0,
                    name: "minecraft:player.experience".to_string(),
                },
            ],
            tick,
        }
    }

    /// Create a full UpdateAttributes packet (health + hunger + XP).
    #[allow(clippy::too_many_arguments)]
    pub fn all(
        entity_runtime_id: u64,
        health: f32,
        food: f32,
        saturation: f32,
        exhaustion: f32,
        level: i32,
        progress: f32,
        tick: u64,
    ) -> Self {
        Self {
            entity_runtime_id,
            attributes: vec![
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: health,
                    default: 20.0,
                    name: "minecraft:health".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: food,
                    default: 20.0,
                    name: "minecraft:player.hunger".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 20.0,
                    current: saturation,
                    default: 5.0,
                    name: "minecraft:player.saturation".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 5.0,
                    current: exhaustion,
                    default: 0.0,
                    name: "minecraft:player.exhaustion".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 24791.0,
                    current: level as f32,
                    default: 0.0,
                    name: "minecraft:player.level".to_string(),
                },
                AttributeEntry {
                    min: 0.0,
                    max: 1.0,
                    current: progress,
                    default: 0.0,
                    name: "minecraft:player.experience".to_string(),
                },
            ],
            tick,
        }
    }
}

impl ProtoEncode for UpdateAttributes {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        VarUInt32(self.attributes.len() as u32).proto_encode(buf);
        for attr in &self.attributes {
            buf.put_f32_le(attr.min);
            buf.put_f32_le(attr.max);
            buf.put_f32_le(attr.current);
            buf.put_f32_le(attr.default);
            write_string(buf, &attr.name);
            VarUInt32(0).proto_encode(buf); // modifier count = 0
        }
        VarUInt64(self.tick).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_health_full() {
        let pkt = UpdateAttributes::health(1, 20.0, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt64(1) + VarUInt32(1) + 4*f32(16) + string + VarUInt32(0) + VarUInt64(100)
        assert!(buf.len() >= 20);
    }

    #[test]
    fn encode_health_zero() {
        let pkt = UpdateAttributes::health(5, 0.0, 200);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 20);
    }

    #[test]
    fn encode_hunger_attributes() {
        let pkt = UpdateAttributes::hunger(1, 20.0, 5.0, 0.0, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // 3 attributes: hunger + saturation + exhaustion
        assert_eq!(pkt.attributes.len(), 3);
        assert_eq!(pkt.attributes[0].name, "minecraft:player.hunger");
        assert_eq!(pkt.attributes[1].name, "minecraft:player.saturation");
        assert_eq!(pkt.attributes[2].name, "minecraft:player.exhaustion");
        assert!(buf.len() > 60); // 3 attributes with strings
    }

    #[test]
    fn encode_health_and_hunger() {
        let pkt = UpdateAttributes::health_and_hunger(1, 20.0, 20.0, 5.0, 0.0, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // 4 attributes: health + hunger + saturation + exhaustion
        assert_eq!(pkt.attributes.len(), 4);
        assert_eq!(pkt.attributes[0].name, "minecraft:health");
        assert_eq!(pkt.attributes[1].name, "minecraft:player.hunger");
        assert!(buf.len() > 80); // 4 attributes with strings
    }

    #[test]
    fn encode_xp_attributes() {
        let pkt = UpdateAttributes::xp(1, 10, 0.5, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(pkt.attributes.len(), 2);
        assert_eq!(pkt.attributes[0].name, "minecraft:player.level");
        assert!((pkt.attributes[0].current - 10.0).abs() < 0.001);
        assert_eq!(pkt.attributes[1].name, "minecraft:player.experience");
        assert!((pkt.attributes[1].current - 0.5).abs() < 0.001);
        assert!(buf.len() > 40);
    }

    #[test]
    fn encode_all_attributes() {
        let pkt = UpdateAttributes::all(1, 20.0, 20.0, 5.0, 0.0, 5, 0.3, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // 6 attributes: health + hunger + saturation + exhaustion + level + experience
        assert_eq!(pkt.attributes.len(), 6);
        assert_eq!(pkt.attributes[4].name, "minecraft:player.level");
        assert_eq!(pkt.attributes[5].name, "minecraft:player.experience");
        assert!(buf.len() > 120);
    }

    #[test]
    fn xp_attribute_values() {
        let pkt = UpdateAttributes::xp(1, 0, 0.0, 0);
        assert!((pkt.attributes[0].min - 0.0).abs() < 0.001);
        assert!((pkt.attributes[0].max - 24791.0).abs() < 0.001);
        assert!((pkt.attributes[0].default - 0.0).abs() < 0.001);
        assert!((pkt.attributes[1].max - 1.0).abs() < 0.001);
    }
}
