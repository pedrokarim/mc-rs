//! MovePlayer (0x13) — Server → Client.
//!
//! Broadcast a player's position to other players, or correct the
//! player's position with a Reset/Teleport mode.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::{VarUInt64, Vec3};

/// Movement mode for MovePlayer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveMode {
    /// Regular position update (broadcast to others).
    Normal = 0,
    /// Force position correction (server-authoritative reset).
    Reset = 1,
    /// Teleport with cause information.
    Teleport = 2,
    /// Rotation-only update.
    Rotation = 3,
}

impl MoveMode {
    fn from_u8(v: u8) -> Result<Self, ProtoError> {
        match v {
            0 => Ok(MoveMode::Normal),
            1 => Ok(MoveMode::Reset),
            2 => Ok(MoveMode::Teleport),
            3 => Ok(MoveMode::Rotation),
            _ => Err(ProtoError::InvalidLogin(format!(
                "unknown MovePlayer mode: {v}"
            ))),
        }
    }
}

/// MovePlayer packet.
#[derive(Debug, Clone)]
pub struct MovePlayer {
    pub runtime_entity_id: u64,
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    pub mode: MoveMode,
    pub on_ground: bool,
    pub ridden_entity_runtime_id: u64,
    /// Only present when mode == Teleport.
    pub teleport_cause: Option<i32>,
    /// Only present when mode == Teleport.
    pub teleport_entity_type: Option<i32>,
    pub tick: u64,
}

impl MovePlayer {
    /// Create a Reset (correction) packet.
    pub fn reset(
        runtime_entity_id: u64,
        position: Vec3,
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
        on_ground: bool,
        tick: u64,
    ) -> Self {
        Self {
            runtime_entity_id,
            position,
            pitch,
            yaw,
            head_yaw,
            mode: MoveMode::Reset,
            on_ground,
            ridden_entity_runtime_id: 0,
            teleport_cause: None,
            teleport_entity_type: None,
            tick,
        }
    }

    /// Create a Normal (broadcast) packet.
    pub fn normal(
        runtime_entity_id: u64,
        position: Vec3,
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
        on_ground: bool,
        tick: u64,
    ) -> Self {
        Self {
            runtime_entity_id,
            position,
            pitch,
            yaw,
            head_yaw,
            mode: MoveMode::Normal,
            on_ground,
            ridden_entity_runtime_id: 0,
            teleport_cause: None,
            teleport_entity_type: None,
            tick,
        }
    }
}

impl ProtoEncode for MovePlayer {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt64(self.runtime_entity_id).proto_encode(buf);
        self.position.proto_encode(buf);
        buf.put_f32_le(self.pitch);
        buf.put_f32_le(self.yaw);
        buf.put_f32_le(self.head_yaw);
        buf.put_u8(self.mode as u8);
        buf.put_u8(self.on_ground as u8);
        VarUInt64(self.ridden_entity_runtime_id).proto_encode(buf);
        if self.mode == MoveMode::Teleport {
            buf.put_i32_le(self.teleport_cause.unwrap_or(0));
            buf.put_i32_le(self.teleport_entity_type.unwrap_or(0));
        }
        VarUInt64(self.tick).proto_encode(buf);
    }
}

impl ProtoDecode for MovePlayer {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let runtime_entity_id = VarUInt64::proto_decode(buf)?.0;
        let position = Vec3::proto_decode(buf)?;

        if buf.remaining() < 14 {
            return Err(ProtoError::BufferTooShort {
                needed: 14,
                remaining: buf.remaining(),
            });
        }
        let pitch = buf.get_f32_le();
        let yaw = buf.get_f32_le();
        let head_yaw = buf.get_f32_le();
        let mode = MoveMode::from_u8(buf.get_u8())?;
        let on_ground = buf.get_u8() != 0;

        let ridden_entity_runtime_id = VarUInt64::proto_decode(buf)?.0;

        let (teleport_cause, teleport_entity_type) = if mode == MoveMode::Teleport {
            if buf.remaining() < 8 {
                return Err(ProtoError::BufferTooShort {
                    needed: 8,
                    remaining: buf.remaining(),
                });
            }
            (Some(buf.get_i32_le()), Some(buf.get_i32_le()))
        } else {
            (None, None)
        };

        let tick = VarUInt64::proto_decode(buf)?.0;

        Ok(Self {
            runtime_entity_id,
            position,
            pitch,
            yaw,
            head_yaw,
            mode,
            on_ground,
            ridden_entity_runtime_id,
            teleport_cause,
            teleport_entity_type,
            tick,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn roundtrip_normal() {
        let pkt = MovePlayer::normal(1, Vec3::new(10.0, 65.0, 20.0), -5.0, 90.0, 90.0, true, 100);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = MovePlayer::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.runtime_entity_id, 1);
        assert_eq!(decoded.position, Vec3::new(10.0, 65.0, 20.0));
        assert_eq!(decoded.pitch, -5.0);
        assert_eq!(decoded.yaw, 90.0);
        assert_eq!(decoded.head_yaw, 90.0);
        assert_eq!(decoded.mode, MoveMode::Normal);
        assert!(decoded.on_ground);
        assert_eq!(decoded.tick, 100);
        assert!(decoded.teleport_cause.is_none());
    }

    #[test]
    fn roundtrip_reset() {
        let pkt = MovePlayer::reset(1, Vec3::new(0.5, 5.62, 0.5), 0.0, 0.0, 0.0, true, 50);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = MovePlayer::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.mode, MoveMode::Reset);
        assert_eq!(decoded.position, Vec3::new(0.5, 5.62, 0.5));
    }

    #[test]
    fn roundtrip_teleport() {
        let pkt = MovePlayer {
            runtime_entity_id: 1,
            position: Vec3::new(100.0, 64.0, 200.0),
            pitch: 0.0,
            yaw: 0.0,
            head_yaw: 0.0,
            mode: MoveMode::Teleport,
            on_ground: true,
            ridden_entity_runtime_id: 0,
            teleport_cause: Some(0),
            teleport_entity_type: Some(0),
            tick: 200,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = MovePlayer::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.mode, MoveMode::Teleport);
        assert_eq!(decoded.teleport_cause, Some(0));
        assert_eq!(decoded.teleport_entity_type, Some(0));
        assert_eq!(decoded.position, Vec3::new(100.0, 64.0, 200.0));
    }

    #[test]
    fn teleport_mode_adds_8_bytes() {
        let base = MovePlayer::normal(1, Vec3::ZERO, 0.0, 0.0, 0.0, false, 0);
        let mut buf_normal = BytesMut::new();
        base.proto_encode(&mut buf_normal);

        let teleport = MovePlayer {
            mode: MoveMode::Teleport,
            teleport_cause: Some(0),
            teleport_entity_type: Some(0),
            ..base
        };
        let mut buf_teleport = BytesMut::new();
        teleport.proto_encode(&mut buf_teleport);

        assert_eq!(buf_teleport.len(), buf_normal.len() + 8);
    }

    #[test]
    fn mode_from_u8_invalid() {
        assert!(MoveMode::from_u8(4).is_err());
        assert!(MoveMode::from_u8(255).is_err());
    }
}
