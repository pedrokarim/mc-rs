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
}
