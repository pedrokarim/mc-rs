//! SpawnParticleEffect (0x76) — Server → Client.
//!
//! Spawns a named particle effect at a position.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::{VarLong, Vec3};

/// SpawnParticleEffect packet.
pub struct SpawnParticleEffect {
    /// Dimension ID (0=overworld, 1=nether, 2=end).
    pub dimension_id: u8,
    /// Entity unique ID (-1 if not attached to entity).
    pub entity_unique_id: i64,
    /// World position of the particle.
    pub position: Vec3,
    /// Particle identifier (e.g. "minecraft:campfire_smoke_particle").
    pub particle_name: String,
}

impl SpawnParticleEffect {
    /// Create a particle effect at a position (not attached to any entity).
    pub fn at_position(particle_name: impl Into<String>, x: f32, y: f32, z: f32) -> Self {
        Self {
            dimension_id: 0,
            entity_unique_id: -1,
            position: Vec3::new(x, y, z),
            particle_name: particle_name.into(),
        }
    }
}

impl ProtoEncode for SpawnParticleEffect {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.dimension_id);
        VarLong(self.entity_unique_id).proto_encode(buf);
        self.position.proto_encode(buf);
        write_string(buf, &self.particle_name);
        // Optional MoLang variables JSON — empty string
        write_string(buf, "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_particle_effect() {
        let pkt = SpawnParticleEffect::at_position("minecraft:heart_particle", 5.0, 70.0, -3.0);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 0);
        assert_eq!(pkt.dimension_id, 0);
        assert_eq!(pkt.entity_unique_id, -1);
        assert_eq!(pkt.position.x, 5.0);
    }

    #[test]
    fn encode_particle_with_entity() {
        let pkt = SpawnParticleEffect {
            dimension_id: 1,
            entity_unique_id: 42,
            position: Vec3::ZERO,
            particle_name: "minecraft:campfire_smoke_particle".into(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 0);
        assert_eq!(pkt.dimension_id, 1);
        assert_eq!(pkt.entity_unique_id, 42);
    }
}
