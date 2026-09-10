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
    pub const SHOW: u8 = 0;
    pub const REGISTER_PLAYER: u8 = 1;
    pub const HIDE: u8 = 2;
    pub const UNREGISTER_PLAYER: u8 = 3;
    pub const HEALTH_PERCENT: u8 = 4;
    pub const TITLE: u8 = 5;
    pub const PROPERTIES: u8 = 6;
    pub const TEXTURE: u8 = 7;
    pub const QUERY: u8 = 8;
}

/// PMMP `types/BossBarColor` (= gophertunnel `BossEventColour*`). Le 6 est
/// REBECCA_PURPLE, le blanc est 7.
pub mod boss_color {
    pub const PINK: u8 = 0;
    pub const BLUE: u8 = 1;
    pub const RED: u8 = 2;
    pub const GREEN: u8 = 3;
    pub const YELLOW: u8 = 4;
    pub const PURPLE: u8 = 5;
    pub const REBECCA_PURPLE: u8 = 6;
    pub const WHITE: u8 = 7;
}

/// Port de `BossEventPacket` (protocoles 1001 et 2169). Depuis le protocole
/// 984, les 8 champs sont TOUJOURS écrits, quel que soit le type d'événement
/// (plus de champs conditionnels ni de `darkenScreen`) : les champs inutiles
/// gardent leur valeur par défaut, comme les constructeurs statiques de PMMP.
///
/// Réf : BedrockProtocol `BossEventPacket::encodePayload` ; gophertunnel
/// `packet/boss_event.go` `Marshal`.
#[derive(Debug, Clone, PartialEq)]
pub struct BossEvent {
    pub boss_actor_unique_id: i64,
    /// Utile pour REGISTER_PLAYER / UNREGISTER_PLAYER / QUERY ; 0 sinon.
    pub player_actor_unique_id: i64,
    pub event_type: u8,
    pub title: String,
    pub filtered_title: String,
    pub health_percent: f32,
    pub color: u8,
    pub overlay: u8,
}

impl BossEvent {
    /// PMMP `BossEventPacket::base()` : valeurs par défaut des champs.
    fn base(boss_actor_unique_id: i64, event_type: u8) -> Self {
        Self {
            boss_actor_unique_id,
            player_actor_unique_id: 0,
            event_type,
            title: String::new(),
            filtered_title: String::new(),
            health_percent: 0.0,
            color: boss_color::PINK,
            overlay: 0,
        }
    }

    /// PMMP `BossEventPacket::show()` (overlay 0 = barre pleine).
    pub fn show(boss_actor_unique_id: i64, title: &str, health_percent: f32, color: u8) -> Self {
        Self {
            title: title.to_string(),
            filtered_title: title.to_string(),
            health_percent,
            color,
            ..Self::base(boss_actor_unique_id, boss_event_type::SHOW)
        }
    }

    /// PMMP `BossEventPacket::hide()`.
    pub fn hide(boss_actor_unique_id: i64) -> Self {
        Self::base(boss_actor_unique_id, boss_event_type::HIDE)
    }

    /// PMMP `BossEventPacket::healthPercent()`.
    pub fn health_percent(boss_actor_unique_id: i64, health_percent: f32) -> Self {
        Self {
            health_percent,
            ..Self::base(boss_actor_unique_id, boss_event_type::HEALTH_PERCENT)
        }
    }

    /// PMMP `BossEventPacket::title()`.
    pub fn title(boss_actor_unique_id: i64, title: &str) -> Self {
        Self {
            title: title.to_string(),
            filtered_title: title.to_string(),
            ..Self::base(boss_actor_unique_id, boss_event_type::TITLE)
        }
    }

    /// `ActorUniqueId` (VarInt64 zigzag) ×2, u8 type, 2 strings, f32 LE,
    /// u8 couleur, u8 overlay.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(32 + self.title.len() + self.filtered_title.len());
        w.write_var_i64(self.boss_actor_unique_id);
        w.write_var_i64(self.player_actor_unique_id);
        w.write_u8(self.event_type);
        w.write_string(&self.title);
        w.write_string(&self.filtered_title);
        w.write_f32_le(self.health_percent);
        w.write_u8(self.color);
        w.write_u8(self.overlay);
        w.into_bytes()
    }
}

/// Montre une barre de boss (payload prêt à envoyer).
pub fn boss_show(
    boss_actor_unique_id: i64,
    title: &str,
    health_percent: f32,
    color: u8,
) -> Vec<u8> {
    BossEvent::show(boss_actor_unique_id, title, health_percent, color).encode()
}

/// Retire la barre de boss.
pub fn boss_hide(boss_actor_unique_id: i64) -> Vec<u8> {
    BossEvent::hide(boss_actor_unique_id).encode()
}

/// Met à jour le pourcentage de santé de la barre.
pub fn boss_update_health(boss_actor_unique_id: i64, health_percent: f32) -> Vec<u8> {
    BossEvent::health_percent(boss_actor_unique_id, health_percent).encode()
}

/// Met à jour le titre de la barre.
pub fn boss_update_title(boss_actor_unique_id: i64, title: &str) -> Vec<u8> {
    BossEvent::title(boss_actor_unique_id, title).encode()
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
    fn boss_show_writes_all_fields() {
        // boss -1 (zigzag 0x01), joueur 0, type SHOW, titre et titre filtré
        // identiques (PMMP `show()`), santé 1.0, couleur PURPLE, overlay 0.
        let mut expected = vec![0x01, 0x00, 0x00, 0x06];
        expected.extend_from_slice(b"Wither");
        expected.push(0x06);
        expected.extend_from_slice(b"Wither");
        expected.extend_from_slice(&1.0f32.to_le_bytes());
        expected.extend_from_slice(&[boss_color::PURPLE, 0x00]);
        assert_eq!(boss_show(-1, "Wither", 1.0, boss_color::PURPLE), expected);
    }

    #[test]
    fn boss_hide_keeps_default_fields() {
        // Même HIDE écrit les 8 champs : id(1)=0x02, joueur 0, type 2, deux
        // strings vides, f32 0.0, couleur PINK, overlay 0.
        assert_eq!(
            boss_hide(1),
            vec![0x02, 0x00, 0x02, 0x00, 0x00, 0, 0, 0, 0, 0x00, 0x00]
        );
    }

    #[test]
    fn boss_update_health_and_title() {
        let mut expected = vec![0x02, 0x00, boss_event_type::HEALTH_PERCENT, 0x00, 0x00];
        expected.extend_from_slice(&0.5f32.to_le_bytes());
        expected.extend_from_slice(&[0x00, 0x00]);
        assert_eq!(boss_update_health(1, 0.5), expected);

        let mut expected = vec![0x02, 0x00, boss_event_type::TITLE, 0x02];
        expected.extend_from_slice(b"Ab");
        expected.push(0x02);
        expected.extend_from_slice(b"Ab");
        expected.extend_from_slice(&[0, 0, 0, 0, 0x00, 0x00]);
        assert_eq!(boss_update_title(1, "Ab"), expected);
    }

    #[test]
    fn boss_colors_match_protocol() {
        assert_eq!(boss_color::REBECCA_PURPLE, 6);
        assert_eq!(boss_color::WHITE, 7);
    }
}
