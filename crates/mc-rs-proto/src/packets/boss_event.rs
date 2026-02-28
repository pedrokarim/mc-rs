//! BossEvent (0x4A) — Server → Client.
//!
//! Controls boss bar display for a player.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::{VarLong, VarUInt32};

/// Boss bar event types.
pub const EVENT_SHOW: u32 = 0;
pub const EVENT_REGISTER_PLAYER: u32 = 1;
pub const EVENT_HIDE: u32 = 2;
pub const EVENT_UNREGISTER_PLAYER: u32 = 3;
pub const EVENT_HEALTH_PERCENT: u32 = 4;
pub const EVENT_TITLE: u32 = 5;
pub const EVENT_DARKEN_SKY: u32 = 6;
pub const EVENT_OVERLAY: u32 = 7;

/// BossEvent packet.
pub struct BossEvent {
    /// Boss entity unique ID (or arbitrary ID for custom boss bars).
    pub boss_entity_id: i64,
    /// Event type (see EVENT_* constants).
    pub event_type: u32,
    /// Title text (used for Show and Title events).
    pub title: String,
    /// Health percentage 0.0–1.0 (used for Show and HealthPercent events).
    pub health_percent: f32,
    /// Darken sky flag (used for Show and DarkenSky events).
    pub darken_sky: u16,
    /// Color (used for Show and Overlay events).
    pub color: u32,
    /// Overlay type (used for Show and Overlay events).
    pub overlay: u32,
    /// Player unique ID (used for RegisterPlayer/UnregisterPlayer events).
    pub player_unique_id: i64,
}

impl BossEvent {
    /// Create a Show boss bar event.
    pub fn show(boss_id: i64, title: impl Into<String>, health_percent: f32, color: u32) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_SHOW,
            title: title.into(),
            health_percent,
            darken_sky: 0,
            color,
            overlay: 0,
            player_unique_id: 0,
        }
    }

    /// Create a Hide boss bar event.
    pub fn hide(boss_id: i64) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_HIDE,
            title: String::new(),
            health_percent: 0.0,
            darken_sky: 0,
            color: 0,
            overlay: 0,
            player_unique_id: 0,
        }
    }

    /// Create a HealthPercent update event.
    pub fn update_health(boss_id: i64, health_percent: f32) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_HEALTH_PERCENT,
            title: String::new(),
            health_percent,
            darken_sky: 0,
            color: 0,
            overlay: 0,
            player_unique_id: 0,
        }
    }

    /// Create a Title update event.
    pub fn update_title(boss_id: i64, title: impl Into<String>) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_TITLE,
            title: title.into(),
            health_percent: 0.0,
            darken_sky: 0,
            color: 0,
            overlay: 0,
            player_unique_id: 0,
        }
    }

    /// Create a RegisterPlayer event.
    pub fn register_player(boss_id: i64, player_id: i64) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_REGISTER_PLAYER,
            title: String::new(),
            health_percent: 0.0,
            darken_sky: 0,
            color: 0,
            overlay: 0,
            player_unique_id: player_id,
        }
    }

    /// Create an UnregisterPlayer event.
    pub fn unregister_player(boss_id: i64, player_id: i64) -> Self {
        Self {
            boss_entity_id: boss_id,
            event_type: EVENT_UNREGISTER_PLAYER,
            title: String::new(),
            health_percent: 0.0,
            darken_sky: 0,
            color: 0,
            overlay: 0,
            player_unique_id: player_id,
        }
    }
}

impl ProtoEncode for BossEvent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarLong(self.boss_entity_id).proto_encode(buf);
        VarUInt32(self.event_type).proto_encode(buf);
        match self.event_type {
            EVENT_SHOW => {
                write_string(buf, &self.title);
                buf.put_f32_le(self.health_percent);
                buf.put_u16_le(self.darken_sky);
                VarUInt32(self.color).proto_encode(buf);
                VarUInt32(self.overlay).proto_encode(buf);
            }
            EVENT_REGISTER_PLAYER | EVENT_UNREGISTER_PLAYER => {
                VarLong(self.player_unique_id).proto_encode(buf);
            }
            EVENT_HIDE => {
                // No additional data
            }
            EVENT_HEALTH_PERCENT => {
                buf.put_f32_le(self.health_percent);
            }
            EVENT_TITLE => {
                write_string(buf, &self.title);
            }
            EVENT_DARKEN_SKY => {
                buf.put_u16_le(self.darken_sky);
            }
            EVENT_OVERLAY => {
                VarUInt32(self.overlay).proto_encode(buf);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_show_event() {
        let pkt = BossEvent::show(1, "Ender Dragon", 1.0, 1);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.event_type, EVENT_SHOW);
        assert_eq!(pkt.title, "Ender Dragon");
        assert_eq!(pkt.health_percent, 1.0);
    }

    #[test]
    fn encode_hide_event() {
        let pkt = BossEvent::hide(1);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.event_type, EVENT_HIDE);
    }

    #[test]
    fn encode_health_update() {
        let pkt = BossEvent::update_health(1, 0.5);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.event_type, EVENT_HEALTH_PERCENT);
        assert_eq!(pkt.health_percent, 0.5);
    }

    #[test]
    fn encode_register_player() {
        let pkt = BossEvent::register_player(1, 42);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.event_type, EVENT_REGISTER_PLAYER);
        assert_eq!(pkt.player_unique_id, 42);
    }
}
