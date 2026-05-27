//! Packets visuels client-side — particles + boss bar.
//!
//! Ports :
//! - `.reference/PocketMine-MP/vendor/.../SpawnParticleEffectPacket.php`
//! - `.reference/PocketMine-MP/vendor/.../BossEventPacket.php`

use mc_rs_proto::io::ProtoWriter;
use mc_rs_proto::packets::packet_id;

// ── SpawnParticleEffect (S→C, 0x77) ─────────────────────────────────────────

pub const DIMENSION_OVERWORLD: u8 = 0;
pub const DIMENSION_NETHER: u8 = 1;
pub const DIMENSION_END: u8 = 2;

/// Port de `SpawnParticleEffectPacket`. Particules identifiées par nom
/// (`"minecraft:basic_flame_particle"`, `"minecraft:explosion_particle"`, etc.).
pub struct SpawnParticleEffect {
    pub dimension_id: u8,
    pub actor_unique_id: i64,
    pub position: [f32; 3],
    pub particle_name: String,
    pub molang_variables_json: Option<String>,
}

impl SpawnParticleEffect {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(64);
        w.write_u8(self.dimension_id);
        w.write_var_i64(self.actor_unique_id);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_string(&self.particle_name);
        w.write_bool(self.molang_variables_json.is_some());
        if let Some(ref s) = self.molang_variables_json {
            w.write_string(s);
        }
        w.into_bytes()
    }

    /// Helper : particule basique à une position sans molang.
    pub fn at(position: [f32; 3], particle_name: impl Into<String>) -> Vec<u8> {
        Self {
            dimension_id: DIMENSION_OVERWORLD,
            actor_unique_id: -1,
            position,
            particle_name: particle_name.into(),
            molang_variables_json: None,
        }
        .encode()
    }
}

// ── BossEvent (S→C, 0x4A) ───────────────────────────────────────────────────

/// PMMP `BossEventPacket::TYPE_*` constants.
pub mod boss_event_type {
    pub const SHOW: u32 = 0;
    pub const REGISTER_PLAYER: u32 = 1;
    pub const HIDE: u32 = 2;
    pub const UNREGISTER_PLAYER: u32 = 3;
    pub const HEALTH_PERCENT: u32 = 4;
    pub const TITLE: u32 = 5;
    pub const PROPERTIES: u32 = 6;
    pub const TEXTURE: u32 = 7;
    pub const QUERY: u32 = 8;
}

/// PMMP `BossEventPacket::COLOR_*` (pass-through au client pour teinture UI).
pub mod boss_color {
    pub const PINK: u32 = 0;
    pub const BLUE: u32 = 1;
    pub const RED: u32 = 2;
    pub const GREEN: u32 = 3;
    pub const YELLOW: u32 = 4;
    pub const PURPLE: u32 = 5;
    pub const WHITE: u32 = 6;
}

/// Show a new boss bar. Combine PMMP `BossEvent::show()`.
pub fn boss_show(
    boss_actor_unique_id: i64,
    title: &str,
    health_percent: f32,
    color: u32,
) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(64);
    w.write_var_i64(boss_actor_unique_id);
    w.write_var_u32(boss_event_type::SHOW);
    w.write_string(title);
    w.write_string(""); // filtered title
    w.write_f32_le(health_percent);
    // Fall-through PROPERTIES
    w.write_u16_le(0); // darken_screen = false
                       // TEXTURE
    w.write_var_u32(color);
    w.write_var_u32(0); // overlay
    w.into_bytes()
}

pub fn boss_hide(boss_actor_unique_id: i64) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(8);
    w.write_var_i64(boss_actor_unique_id);
    w.write_var_u32(boss_event_type::HIDE);
    w.into_bytes()
}

pub fn boss_update_health(boss_actor_unique_id: i64, health_percent: f32) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(16);
    w.write_var_i64(boss_actor_unique_id);
    w.write_var_u32(boss_event_type::HEALTH_PERCENT);
    w.write_f32_le(health_percent);
    w.into_bytes()
}

pub fn boss_update_title(boss_actor_unique_id: i64, title: &str) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(32 + title.len());
    w.write_var_i64(boss_actor_unique_id);
    w.write_var_u32(boss_event_type::TITLE);
    w.write_string(title);
    w.write_string(""); // filtered title
    w.into_bytes()
}

/// Packet IDs (for caller's `encode_compressed_packet`).
/// Chercher les vrais IDs dans `mc_rs_proto::packets::packet_id`.
pub const SPAWN_PARTICLE_EFFECT: u32 = packet_id::SPAWN_PARTICLE_EFFECT;
pub const BOSS_EVENT: u32 = packet_id::BOSS_EVENT;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_particle_encodes() {
        let pkt = SpawnParticleEffect {
            dimension_id: 0,
            actor_unique_id: -1,
            position: [1.0, 64.0, 2.0],
            particle_name: "minecraft:basic_flame_particle".to_string(),
            molang_variables_json: None,
        };
        let bytes = pkt.encode();
        // dimension(1) + actor_uid(varint64 zigzag -1 = 1 byte) + 3x f32 (12) +
        // string varlen(1) + "minecraft:basic_flame_particle"(30) + bool(1)
        assert!(bytes.len() > 1 + 1 + 12 + 1 + 30);
        assert_eq!(bytes[0], 0); // overworld
    }

    #[test]
    fn boss_show_contains_title() {
        let bytes = boss_show(-1, "Ender Dragon", 1.0, boss_color::PURPLE);
        let title_start = bytes.iter().position(|&b| b == b'E').unwrap();
        let title_bytes = &bytes[title_start..title_start + 12];
        assert_eq!(title_bytes, b"Ender Dragon");
    }
}
