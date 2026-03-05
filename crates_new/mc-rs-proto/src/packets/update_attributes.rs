use crate::codec::*;
use bytes::{BufMut, BytesMut};

pub struct Attribute {
    pub min: f32,
    pub max: f32,
    pub current: f32,
    pub default: f32,
    pub name: String,
    pub modifiers: Vec<AttributeModifier>,
}

pub struct AttributeModifier {
    pub id: String,
    pub name: String,
    pub amount: f32,
    pub operation: i32,
    pub operand: i32,
    pub serializable: bool,
}

/// UpdateAttributesPacket
/// actorRuntimeId: UnsignedVarLong
/// attributes: UnsignedVarInt(count) + entries
/// tick: UnsignedVarLong
pub fn encode(actor_runtime_id: u64, attributes: &[Attribute], tick: u64) -> BytesMut {
    let mut buf = BytesMut::new();
    write_unsigned_varlong(&mut buf, actor_runtime_id);
    write_unsigned_varint32(&mut buf, attributes.len() as u32);
    for attr in attributes {
        buf.put_f32_le(attr.min);
        buf.put_f32_le(attr.max);
        buf.put_f32_le(attr.current);
        buf.put_f32_le(attr.default);
        write_string(&mut buf, &attr.name);
        write_unsigned_varint32(&mut buf, attr.modifiers.len() as u32);
        for m in &attr.modifiers {
            write_string(&mut buf, &m.id);
            write_string(&mut buf, &m.name);
            buf.put_f32_le(m.amount);
            write_signed_varint32(&mut buf, m.operation);
            write_signed_varint32(&mut buf, m.operand);
            buf.put_u8(m.serializable as u8);
        }
    }
    write_unsigned_varlong(&mut buf, tick);
    buf
}

/// Encode default player attributes (health, hunger, etc.)
pub fn encode_default_player(actor_runtime_id: u64) -> BytesMut {
    let attrs = vec![
        Attribute {
            min: 0.0,
            max: 20.0,
            current: 20.0,
            default: 20.0,
            name: "minecraft:health".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 20.0,
            current: 20.0,
            default: 20.0,
            name: "minecraft:player.hunger".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 0.1,
            current: 0.1,
            default: 0.1,
            name: "minecraft:movement".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 2048.0,
            current: 2048.0,
            default: 2048.0,
            name: "minecraft:follow_range".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 1.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:player.saturation".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 5.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:player.exhaustion".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 24791.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:player.level".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 24791.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:player.experience".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 20.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:absorption".into(),
            modifiers: vec![],
        },
        Attribute {
            min: 0.0,
            max: 1.0,
            current: 0.0,
            default: 0.0,
            name: "minecraft:luck".into(),
            modifiers: vec![],
        },
    ];
    encode(actor_runtime_id, &attrs, 0)
}
