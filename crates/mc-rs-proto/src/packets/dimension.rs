//! Paquets liés au changement de dimension (Overworld / Nether / End).
//!
//! Formats reproduits fidèlement depuis BedrockProtocol (PMMP, bedrock-1.26.20) :
//! - `ChangeDimensionPacket.php`
//! - `RespawnPacket.php`
//! - `PlayerActionPacket.php`
//!
//! Séquence wire d'un transfert (réf dragonfly `session.go`, valable depuis 1.19.50) :
//!   ChangeDimension → (chunks) → StopSound(silent) → PlayStatus(PlayerSpawn)
//!   → PlayerAction{ DIMENSION_CHANGE_ACK }   (le serveur s'auto-ack)

use crate::io::reader::ProtoReadError;
use crate::io::{ProtoReader, ProtoWriter};

/// IDs canoniques de dimension (réf `types/DimensionIds.php`, tous serveurs Bedrock).
pub mod dimension_ids {
    pub const OVERWORLD: i32 = 0;
    pub const NETHER: i32 = 1;
    pub const THE_END: i32 = 2;
}

// ── ChangeDimension (S→C, 0x3D) ──
//
// VarInt32 dimension
// Vector3  position (3× f32 LE)
// bool     respawn
// Optional<UInt32 LE> loadingScreenId  (bool présent + valeur conditionnelle)

pub struct ChangeDimension {
    pub dimension: i32,
    pub position: [f32; 3],
    pub respawn: bool,
    pub loading_screen_id: Option<u32>,
}

impl ChangeDimension {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(24);
        w.write_var_i32(self.dimension);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_bool(self.respawn);
        match self.loading_screen_id {
            Some(id) => {
                w.write_bool(true);
                w.write_u32_le(id);
            }
            None => w.write_bool(false),
        }
        w.into_bytes()
    }

    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let dimension = reader.read_var_i32()?;
        let position = [
            reader.read_f32_le()?,
            reader.read_f32_le()?,
            reader.read_f32_le()?,
        ];
        let respawn = reader.read_bool()?;
        let loading_screen_id = if reader.read_bool()? {
            Some(reader.read_u32_le()?)
        } else {
            None
        };
        Ok(Self {
            dimension,
            position,
            respawn,
            loading_screen_id,
        })
    }
}

// ── Respawn (S↔C, 0x2D) ──
//
// Vector3 position (3× f32 LE)
// u8      respawnState
// VarU64  actorRuntimeId

/// États de la state machine de respawn (réf `RespawnPacket.php`).
pub mod respawn_state {
    pub const SEARCHING_FOR_SPAWN: u8 = 0;
    pub const READY_TO_SPAWN: u8 = 1;
    pub const CLIENT_READY_TO_SPAWN: u8 = 2;
}

pub struct Respawn {
    pub position: [f32; 3],
    pub respawn_state: u8,
    pub actor_runtime_id: u64,
}

impl Respawn {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(20);
        w.write_f32_le(self.position[0]);
        w.write_f32_le(self.position[1]);
        w.write_f32_le(self.position[2]);
        w.write_u8(self.respawn_state);
        w.write_var_u64(self.actor_runtime_id);
        w.into_bytes()
    }

    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let position = [
            reader.read_f32_le()?,
            reader.read_f32_le()?,
            reader.read_f32_le()?,
        ];
        let respawn_state = reader.read_u8()?;
        let actor_runtime_id = reader.read_var_u64()?;
        Ok(Self {
            position,
            respawn_state,
            actor_runtime_id,
        })
    }
}

// ── PlayerAction (C↔S, 0x24) ──
//
// VarU64        actorRuntimeId
// VarInt32      action
// BlockPosition blockPosition  (3× VarInt32 signés)
// BlockPosition resultPosition (3× VarInt32 signés)
// VarInt32      face

/// Constantes d'action (réf `types/PlayerAction.php`).
/// Seules celles utilisées sont déclarées ; compléter au besoin.
pub mod player_action_type {
    pub const RESPAWN: i32 = 7;
    /// Envoyé quand on spawn dans une autre dimension pour signaler la fin du transfert.
    /// Depuis 1.19.50, le serveur l'envoie lui-même (cf. dragonfly).
    pub const DIMENSION_CHANGE_ACK: i32 = 14;
}

pub struct PlayerAction {
    pub actor_runtime_id: u64,
    pub action: i32,
    pub block_position: [i32; 3],
    pub result_position: [i32; 3],
    pub face: i32,
}

impl PlayerAction {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ProtoWriter::with_capacity(16);
        w.write_var_u64(self.actor_runtime_id);
        w.write_var_i32(self.action);
        for c in self.block_position {
            w.write_var_i32(c);
        }
        for c in self.result_position {
            w.write_var_i32(c);
        }
        w.write_var_i32(self.face);
        w.into_bytes()
    }

    pub fn decode(reader: &mut ProtoReader) -> Result<Self, ProtoReadError> {
        let actor_runtime_id = reader.read_var_u64()?;
        let action = reader.read_var_i32()?;
        let block_position = [
            reader.read_var_i32()?,
            reader.read_var_i32()?,
            reader.read_var_i32()?,
        ];
        let result_position = [
            reader.read_var_i32()?,
            reader.read_var_i32()?,
            reader.read_var_i32()?,
        ];
        let face = reader.read_var_i32()?;
        Ok(Self {
            actor_runtime_id,
            action,
            block_position,
            result_position,
            face,
        })
    }

    /// Construit l'ack de fin de changement de dimension que le serveur s'envoie à lui-même.
    pub fn dimension_change_ack(actor_runtime_id: u64) -> Self {
        Self {
            actor_runtime_id,
            action: player_action_type::DIMENSION_CHANGE_ACK,
            block_position: [0, 0, 0],
            result_position: [0, 0, 0],
            face: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_dimension_roundtrip_with_loading_screen() {
        let pkt = ChangeDimension {
            dimension: dimension_ids::NETHER,
            position: [12.5, 64.0, -8.25],
            respawn: false,
            loading_screen_id: Some(7),
        };
        let bytes = pkt.encode();
        let mut r = ProtoReader::new(&bytes);
        let got = ChangeDimension::decode(&mut r).unwrap();
        assert_eq!(got.dimension, dimension_ids::NETHER);
        assert_eq!(got.position, [12.5, 64.0, -8.25]);
        assert!(!got.respawn);
        assert_eq!(got.loading_screen_id, Some(7));
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn change_dimension_roundtrip_no_loading_screen() {
        let pkt = ChangeDimension {
            dimension: dimension_ids::THE_END,
            position: [100.5, 50.0, 0.5],
            respawn: true,
            loading_screen_id: None,
        };
        let bytes = pkt.encode();
        let mut r = ProtoReader::new(&bytes);
        let got = ChangeDimension::decode(&mut r).unwrap();
        assert_eq!(got.dimension, dimension_ids::THE_END);
        assert!(got.respawn);
        assert_eq!(got.loading_screen_id, None);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn respawn_roundtrip() {
        let pkt = Respawn {
            position: [1.0, 2.0, 3.0],
            respawn_state: respawn_state::READY_TO_SPAWN,
            actor_runtime_id: 42,
        };
        let bytes = pkt.encode();
        let mut r = ProtoReader::new(&bytes);
        let got = Respawn::decode(&mut r).unwrap();
        assert_eq!(got.position, [1.0, 2.0, 3.0]);
        assert_eq!(got.respawn_state, respawn_state::READY_TO_SPAWN);
        assert_eq!(got.actor_runtime_id, 42);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn player_action_roundtrip() {
        let pkt = PlayerAction {
            actor_runtime_id: 99,
            action: player_action_type::DIMENSION_CHANGE_ACK,
            block_position: [-1, 2, -3],
            result_position: [0, 0, 0],
            face: 4,
        };
        let bytes = pkt.encode();
        let mut r = ProtoReader::new(&bytes);
        let got = PlayerAction::decode(&mut r).unwrap();
        assert_eq!(got.actor_runtime_id, 99);
        assert_eq!(got.action, player_action_type::DIMENSION_CHANGE_ACK);
        assert_eq!(got.block_position, [-1, 2, -3]);
        assert_eq!(got.face, 4);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn dimension_change_ack_helper() {
        let ack = PlayerAction::dimension_change_ack(123);
        assert_eq!(ack.action, player_action_type::DIMENSION_CHANGE_ACK);
        assert_eq!(ack.actor_runtime_id, 123);
        assert_eq!(ack.block_position, [0, 0, 0]);
    }
}
