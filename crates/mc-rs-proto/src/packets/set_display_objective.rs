//! SetDisplayObjective (0x6B) — Server → Client.
//!
//! Assigns a scoreboard objective to a display slot.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarInt;

/// SetDisplayObjective packet.
pub struct SetDisplayObjective {
    /// Display slot: "sidebar", "list", or "belowname".
    pub display_slot: String,
    /// Objective name (internal identifier).
    pub objective_name: String,
    /// Display name shown to players.
    pub display_name: String,
    /// Criteria type (typically "dummy").
    pub criteria: String,
    /// Sort order: 0=ascending, 1=descending.
    pub sort_order: i32,
}

impl SetDisplayObjective {
    /// Create a sidebar display objective.
    pub fn sidebar(objective_name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            display_slot: "sidebar".into(),
            objective_name: objective_name.into(),
            display_name: display_name.into(),
            criteria: "dummy".into(),
            sort_order: 1,
        }
    }

    /// Clear a display slot (empty objective name).
    pub fn clear(display_slot: impl Into<String>) -> Self {
        Self {
            display_slot: display_slot.into(),
            objective_name: String::new(),
            display_name: String::new(),
            criteria: "dummy".into(),
            sort_order: 0,
        }
    }
}

impl ProtoEncode for SetDisplayObjective {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        write_string(buf, &self.display_slot);
        write_string(buf, &self.objective_name);
        write_string(buf, &self.display_name);
        write_string(buf, &self.criteria);
        VarInt(self.sort_order).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_sidebar_objective() {
        let pkt = SetDisplayObjective::sidebar("kills", "Player Kills");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 0);
        assert_eq!(pkt.display_slot, "sidebar");
        assert_eq!(pkt.objective_name, "kills");
    }

    #[test]
    fn encode_clear_display() {
        let pkt = SetDisplayObjective::clear("sidebar");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 0);
        assert!(pkt.objective_name.is_empty());
    }
}
