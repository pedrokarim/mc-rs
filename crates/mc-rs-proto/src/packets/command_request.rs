//! CommandRequest (0x4D) — Client → Server.
//!
//! Sent when the player types a `/command` in chat.

use bytes::Buf;

use crate::codec::{read_string, write_string, ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::{Uuid, VarLong, VarUInt32};

/// Origin information for a command request.
///
/// Must be echoed back verbatim in CommandOutput.
#[derive(Debug, Clone)]
pub struct CommandOrigin {
    pub origin_type: u32,
    pub uuid: Uuid,
    pub request_id: String,
    /// Only present when origin_type == 3 (DevConsole) or 5 (Test).
    pub player_entity_id: Option<i64>,
}

impl ProtoDecode for CommandOrigin {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let origin_type = VarUInt32::proto_decode(buf)?.0;
        let uuid = Uuid::proto_decode(buf)?;
        let request_id = read_string(buf)?;

        let player_entity_id = if origin_type == 3 || origin_type == 5 {
            Some(VarLong::proto_decode(buf)?.0)
        } else {
            None
        };

        Ok(Self {
            origin_type,
            uuid,
            request_id,
            player_entity_id,
        })
    }
}

impl ProtoEncode for CommandOrigin {
    fn proto_encode(&self, buf: &mut impl bytes::BufMut) {
        VarUInt32(self.origin_type).proto_encode(buf);
        self.uuid.proto_encode(buf);
        write_string(buf, &self.request_id);
        if self.origin_type == 3 || self.origin_type == 5 {
            VarLong(self.player_entity_id.unwrap_or(0)).proto_encode(buf);
        }
    }
}

/// CommandRequest packet.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub command: String,
    pub origin: CommandOrigin,
}

impl ProtoDecode for CommandRequest {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let command = read_string(buf)?;
        let origin = CommandOrigin::proto_decode(buf)?;
        Ok(Self { command, origin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn encode_command_request(command: &str, origin_type: u32) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, command);
        VarUInt32(origin_type).proto_encode(&mut buf);
        Uuid::ZERO.proto_encode(&mut buf);
        write_string(&mut buf, ""); // request_id
        buf
    }

    #[test]
    fn decode_player_origin() {
        let buf = encode_command_request("/say hello", 0);
        let pkt = CommandRequest::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.command, "/say hello");
        assert_eq!(pkt.origin.origin_type, 0);
        assert_eq!(pkt.origin.uuid, Uuid::ZERO);
        assert!(pkt.origin.player_entity_id.is_none());
    }

    #[test]
    fn decode_dev_console_origin() {
        let mut buf = BytesMut::new();
        write_string(&mut buf, "/stop");
        VarUInt32(3).proto_encode(&mut buf); // DevConsole
        Uuid::ZERO.proto_encode(&mut buf);
        write_string(&mut buf, "req-1");
        VarLong(42).proto_encode(&mut buf); // player_entity_id
        let pkt = CommandRequest::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.command, "/stop");
        assert_eq!(pkt.origin.origin_type, 3);
        assert_eq!(pkt.origin.request_id, "req-1");
        assert_eq!(pkt.origin.player_entity_id, Some(42));
    }
}
