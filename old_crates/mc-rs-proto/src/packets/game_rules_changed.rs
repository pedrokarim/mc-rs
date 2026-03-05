//! GameRulesChanged (0x48) — Server → Client.
//!
//! Notifies clients of changed game rules (e.g. doDaylightCycle, doWeatherCycle).

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::packets::start_game::{encode_game_rules, GameRule};

/// GameRulesChanged packet.
pub struct GameRulesChanged {
    pub rules: Vec<GameRule>,
}

impl ProtoEncode for GameRulesChanged {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        encode_game_rules(buf, &self.rules);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::start_game::GameRuleValue;
    use bytes::BytesMut;

    #[test]
    fn encode_single_bool_rule() {
        let pkt = GameRulesChanged {
            rules: vec![GameRule {
                name: "doDaylightCycle".into(),
                editable: false,
                value: GameRuleValue::Bool(false),
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt32(1) + string("doDaylightCycle") + u8(editable) + VarUInt32(type=1) + u8(value)
        assert!(buf.len() > 10);
    }

    #[test]
    fn encode_two_rules() {
        let pkt = GameRulesChanged {
            rules: vec![
                GameRule {
                    name: "doDaylightCycle".into(),
                    editable: false,
                    value: GameRuleValue::Bool(true),
                },
                GameRule {
                    name: "doWeatherCycle".into(),
                    editable: false,
                    value: GameRuleValue::Bool(false),
                },
            ],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 20);
    }
}
