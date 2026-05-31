//! Packets combat S→C : ActorEvent (hurt animation), SetActorMotion (knockback),
//! Respawn. Ports PMMP `vendor/pocketmine/bedrock-protocol/src/*`.

use mc_rs_proto::io::ProtoWriter;

// ── ActorEvent (S→C, 0x1B) ──────────────────────────────────────────────────

/// PMMP `ActorEvent::EVENT_*` constants.
pub mod actor_event {
    pub const HURT_ANIMATION: u32 = 2;
    pub const DEATH_ANIMATION: u32 = 3;
    pub const COMPLETE_TRADE: u32 = 4;
    pub const TAME_FAIL: u32 = 6;
    pub const TAME_SUCCESS: u32 = 7;
    pub const SHAKE_WET: u32 = 8;
    pub const EATING_ITEM: u32 = 57;
    pub const RESPAWN: u32 = 58;
}

/// `ActorEventPacket` encoding : VarU64 actorRuntimeId, u8 eventId, VarI32 data.
pub fn encode_actor_event(runtime_entity_id: u64, event_id: u32, data: i32) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(16);
    w.write_var_u64(runtime_entity_id);
    w.write_u8(event_id as u8);
    w.write_var_i32(data);
    w.into_bytes()
}

/// Hurt animation = `ActorEvent::HURT_ANIMATION`.
pub fn hurt_animation(runtime_entity_id: u64) -> Vec<u8> {
    encode_actor_event(runtime_entity_id, actor_event::HURT_ANIMATION, 0)
}

/// Death animation.
pub fn death_animation(runtime_entity_id: u64) -> Vec<u8> {
    encode_actor_event(runtime_entity_id, actor_event::DEATH_ANIMATION, 0)
}

// ── BossEvent (S→C, 0x4A) ───────────────────────────────────────────────────

/// `BossEventPacket` — barre de boss. Format PMMP `BossEventPacket::encodePayload`.
pub mod boss_event {
    pub const TYPE_SHOW: u32 = 0;
    pub const TYPE_HIDE: u32 = 2;
    pub const TYPE_HEALTH_PERCENT: u32 = 4;
    // Couleurs de barre (BossBarColor) : 0=rose, 1=bleu, 2=rouge, 3=vert,
    // 4=jaune, 5=violet, 6=blanc.
    pub const COLOR_PINK: u32 = 0;
    pub const COLOR_PURPLE: u32 = 5;
}

/// SHOW : crée/montre la barre (titre + santé% + couleur, écran assombri).
pub fn boss_show(boss_unique_id: i64, title: &str, health_percent: f32, color: u32) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(48);
    w.write_var_i64(boss_unique_id); // ActorUniqueId (zigzag varlong)
    w.write_var_u32(boss_event::TYPE_SHOW);
    w.write_string(title);
    w.write_string(""); // filteredTitle
    w.write_f32_le(health_percent);
    w.write_u16_le(1); // darkenScreen
    w.write_var_u32(color);
    w.write_var_u32(0); // overlay
    w.into_bytes()
}

/// HEALTH_PERCENT : met à jour la santé de la barre.
pub fn boss_health(boss_unique_id: i64, health_percent: f32) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(16);
    w.write_var_i64(boss_unique_id);
    w.write_var_u32(boss_event::TYPE_HEALTH_PERCENT);
    w.write_f32_le(health_percent);
    w.into_bytes()
}

/// HIDE : retire la barre.
pub fn boss_hide(boss_unique_id: i64) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(8);
    w.write_var_i64(boss_unique_id);
    w.write_var_u32(boss_event::TYPE_HIDE);
    w.into_bytes()
}

// ── Animate (S→C, 0x2C) ─────────────────────────────────────────────────────

/// `AnimatePacket` (protocol 975) : u8 action, VarU64 actorRuntimeId, f32 LE
/// data, optional string swingSource. Réf PMMP `AnimatePacket::encodePayload`.
pub fn arm_swing(runtime_entity_id: u64) -> Vec<u8> {
    const ACTION_SWING_ARM: u8 = 1;
    let mut w = ProtoWriter::with_capacity(16);
    w.write_u8(ACTION_SWING_ARM);
    w.write_var_u64(runtime_entity_id);
    w.write_f32_le(0.0);
    w.write_bool(false); // pas de swingSource (Optional = false)
    w.into_bytes()
}

// ── Respawn (S→C, 0x2D) ─────────────────────────────────────────────────────

/// PMMP `RespawnPacket::STATE_*`.
pub mod respawn_state {
    pub const SEARCHING_FOR_SPAWN: u8 = 0;
    pub const READY_TO_SPAWN: u8 = 1;
    pub const CLIENT_READY_TO_SPAWN: u8 = 2;
}

/// `RespawnPacket` : Vec3 position, u8 state, VarU64 runtime_entity_id.
pub fn encode_respawn(position: [f32; 3], state: u8, runtime_entity_id: u64) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(16);
    w.write_f32_le(position[0]);
    w.write_f32_le(position[1]);
    w.write_f32_le(position[2]);
    w.write_u8(state);
    w.write_var_u64(runtime_entity_id);
    w.into_bytes()
}

// ── SetActorMotion (S→C, 0x28) ──────────────────────────────────────────────
// Used to apply knockback after a hit. Protocol 944 ajoute un champ `tick` (VarU64).

/// `SetActorMotionPacket` 944 : VarU64 runtime_id, Vec3 motion, VarU64 tick.
pub fn encode_set_actor_motion(runtime_entity_id: u64, motion: [f32; 3], tick: u64) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(24);
    w.write_var_u64(runtime_entity_id);
    w.write_f32_le(motion[0]);
    w.write_f32_le(motion[1]);
    w.write_f32_le(motion[2]);
    w.write_var_u64(tick);
    w.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_event_hurt_3_bytes_min() {
        let bytes = hurt_animation(1);
        // runtime_id VarU64(1)=1 + event_id u8=1 + data VarI32=1 → at least 3 bytes
        assert!(bytes.len() >= 3);
    }

    #[test]
    fn respawn_len_is_correct() {
        let bytes = encode_respawn([1.0, 64.0, 2.0], respawn_state::READY_TO_SPAWN, 1);
        // 3 f32 (12) + u8 (1) + VarU64(1) (1) = 14
        assert_eq!(bytes.len(), 14);
    }

    #[test]
    fn boss_event_encoding() {
        // HIDE(id=1) = ActorUniqueId zigzag(1)=2 (0x02) + eventType VarU32(2) (0x02).
        assert_eq!(boss_hide(1), vec![0x02, 0x02]);
        // HEALTH_PERCENT = id + type(4) + f32 → 2 + 4 = 6 octets.
        assert_eq!(boss_health(1, 0.5).len(), 6);
        // SHOW est plus long (titre + santé + couleur + overlay).
        assert!(boss_show(1, "Wither", 1.0, 5).len() > boss_health(1, 1.0).len());
    }
}
