//! SetScore (0x6C) — Server → Client.
//!
//! Updates scoreboard score entries.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarLong;

/// Score action: set/update scores.
pub const SCORE_ACTION_CHANGE: u8 = 0;
/// Score action: remove scores.
pub const SCORE_ACTION_REMOVE: u8 = 1;

/// Identity type for fake player entries.
pub const IDENTITY_FAKE_PLAYER: u8 = 3;

/// A scoreboard score entry.
pub struct ScoreEntry {
    /// Unique entry ID.
    pub entry_id: i64,
    /// Objective name this score belongs to.
    pub objective_name: String,
    /// Score value.
    pub score: i32,
    /// Identity type (1=player, 2=entity, 3=fake_player). Only for CHANGE action.
    pub identity_type: u8,
    /// Custom name for fake player entries.
    pub custom_name: String,
}

/// SetScore packet.
pub struct SetScore {
    /// Action type (0=change, 1=remove).
    pub action_type: u8,
    /// Score entries.
    pub entries: Vec<ScoreEntry>,
}

impl SetScore {
    /// Create a change (set/update) packet for fake player scores.
    pub fn change(entries: Vec<ScoreEntry>) -> Self {
        Self {
            action_type: SCORE_ACTION_CHANGE,
            entries,
        }
    }

    /// Create a remove packet.
    pub fn remove(entries: Vec<ScoreEntry>) -> Self {
        Self {
            action_type: SCORE_ACTION_REMOVE,
            entries,
        }
    }
}

impl ProtoEncode for SetScore {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.action_type);
        crate::types::VarUInt32(self.entries.len() as u32).proto_encode(buf);
        for entry in &self.entries {
            VarLong(entry.entry_id).proto_encode(buf);
            write_string(buf, &entry.objective_name);
            buf.put_i32_le(entry.score);
            if self.action_type == SCORE_ACTION_CHANGE {
                buf.put_u8(entry.identity_type);
                if entry.identity_type == IDENTITY_FAKE_PLAYER {
                    write_string(buf, &entry.custom_name);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_change_single_entry() {
        let pkt = SetScore::change(vec![ScoreEntry {
            entry_id: 1,
            objective_name: "kills".into(),
            score: 42,
            identity_type: IDENTITY_FAKE_PLAYER,
            custom_name: "Alice".into(),
        }]);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.action_type, SCORE_ACTION_CHANGE);
        assert_eq!(pkt.entries.len(), 1);
    }

    #[test]
    fn encode_remove_entries() {
        let pkt = SetScore::remove(vec![
            ScoreEntry {
                entry_id: 1,
                objective_name: "kills".into(),
                score: 0,
                identity_type: 0,
                custom_name: String::new(),
            },
            ScoreEntry {
                entry_id: 2,
                objective_name: "kills".into(),
                score: 0,
                identity_type: 0,
                custom_name: String::new(),
            },
        ]);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.action_type, SCORE_ACTION_REMOVE);
        assert_eq!(pkt.entries.len(), 2);
    }

    #[test]
    fn encode_change_multiple_entries() {
        let pkt = SetScore::change(vec![
            ScoreEntry {
                entry_id: 1,
                objective_name: "score".into(),
                score: 100,
                identity_type: IDENTITY_FAKE_PLAYER,
                custom_name: "Player1".into(),
            },
            ScoreEntry {
                entry_id: 2,
                objective_name: "score".into(),
                score: 200,
                identity_type: IDENTITY_FAKE_PLAYER,
                custom_name: "Player2".into(),
            },
        ]);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.entries.len(), 2);
        assert_eq!(pkt.entries[0].score, 100);
        assert_eq!(pkt.entries[1].score, 200);
    }
}
