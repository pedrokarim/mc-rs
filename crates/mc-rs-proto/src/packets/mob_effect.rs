//! MobEffect (0x1C) — Server → Client.
//!
//! Adds, modifies, or removes status effects on entities.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::{VarInt, VarUInt64};

/// Add a new effect.
pub const EFFECT_OP_ADD: u8 = 1;
/// Modify an existing effect.
pub const EFFECT_OP_MODIFY: u8 = 2;
/// Remove an effect.
pub const EFFECT_OP_REMOVE: u8 = 3;

/// Well-known Bedrock effect IDs.
pub mod effect_id {
    pub const SPEED: i32 = 1;
    pub const SLOWNESS: i32 = 2;
    pub const HASTE: i32 = 3;
    pub const MINING_FATIGUE: i32 = 4;
    pub const STRENGTH: i32 = 5;
    pub const INSTANT_HEALTH: i32 = 6;
    pub const INSTANT_DAMAGE: i32 = 7;
    pub const JUMP_BOOST: i32 = 8;
    pub const NAUSEA: i32 = 9;
    pub const REGENERATION: i32 = 10;
    pub const RESISTANCE: i32 = 11;
    pub const FIRE_RESISTANCE: i32 = 12;
    pub const WATER_BREATHING: i32 = 13;
    pub const INVISIBILITY: i32 = 14;
    pub const BLINDNESS: i32 = 15;
    pub const NIGHT_VISION: i32 = 16;
    pub const HUNGER: i32 = 17;
    pub const WEAKNESS: i32 = 18;
    pub const POISON: i32 = 19;
    pub const WITHER: i32 = 20;
    pub const ABSORPTION: i32 = 22;
}

/// MobEffect packet.
pub struct MobEffect {
    /// Runtime ID of the entity.
    pub entity_runtime_id: u64,
    /// Operation: add(1), modify(2), or remove(3).
    pub operation: u8,
    /// Effect ID (see `effect_id` module).
    pub effect_id: i32,
    /// Amplifier (0 = level I, 1 = level II, etc.).
    pub amplifier: i32,
    /// Whether particles are visible.
    pub show_particles: bool,
    /// Duration in ticks.
    pub duration: i32,
    /// Server tick (can be 0).
    pub tick: u64,
}

impl MobEffect {
    /// Create an "add effect" packet.
    pub fn add(
        entity_runtime_id: u64,
        effect_id: i32,
        amplifier: i32,
        duration: i32,
        show_particles: bool,
    ) -> Self {
        Self {
            entity_runtime_id,
            operation: EFFECT_OP_ADD,
            effect_id,
            amplifier,
            show_particles,
            duration,
            tick: 0,
        }
    }

    /// Create a "remove effect" packet.
    pub fn remove(entity_runtime_id: u64, effect_id: i32) -> Self {
        Self {
            entity_runtime_id,
            operation: EFFECT_OP_REMOVE,
            effect_id,
            amplifier: 0,
            show_particles: false,
            duration: 0,
            tick: 0,
        }
    }
}

impl ProtoEncode for MobEffect {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.entity_runtime_id).proto_encode(buf);
        buf.put_u8(self.operation);
        VarInt(self.effect_id).proto_encode(buf);
        VarInt(self.amplifier).proto_encode(buf);
        buf.put_u8(if self.show_particles { 1 } else { 0 });
        VarInt(self.duration).proto_encode(buf);
        VarUInt64(self.tick).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn mob_effect_add_encode() {
        let pkt = MobEffect::add(1, effect_id::STRENGTH, 0, 600, true);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt64(1) + u8(1) + VarInt(5) + VarInt(0) + u8(1) + VarInt(600) + VarUInt64(0)
        assert!(buf.len() >= 7);
        assert_eq!(buf[1], EFFECT_OP_ADD); // operation byte
    }

    #[test]
    fn mob_effect_remove_encode() {
        let pkt = MobEffect::remove(1, effect_id::STRENGTH);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() >= 7);
        assert_eq!(buf[1], EFFECT_OP_REMOVE);
    }
}
