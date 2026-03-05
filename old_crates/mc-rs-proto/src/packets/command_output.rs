//! CommandOutput (0x4F) — Server → Client.
//!
//! Sent in response to a CommandRequest with the execution result.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarUInt32;

use super::command_request::CommandOrigin;

/// A single output message within a command result.
#[derive(Debug, Clone)]
pub struct OutputMessage {
    pub is_success: bool,
    pub message_id: String,
    pub parameters: Vec<String>,
}

/// CommandOutput packet.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub origin: CommandOrigin,
    /// 0=None, 1=LastOutput, 2=Silent, 3=AllOutput, 4=DataSet.
    pub output_type: u8,
    pub success_count: u32,
    pub messages: Vec<OutputMessage>,
}

impl CommandOutput {
    /// Create a successful command output with a single message.
    pub fn success(origin: CommandOrigin, message: impl Into<String>) -> Self {
        Self {
            origin,
            output_type: 3, // AllOutput
            success_count: 1,
            messages: vec![OutputMessage {
                is_success: true,
                message_id: message.into(),
                parameters: Vec::new(),
            }],
        }
    }

    /// Create a failed command output with a single message.
    pub fn failure(origin: CommandOrigin, message: impl Into<String>) -> Self {
        Self {
            origin,
            output_type: 3, // AllOutput
            success_count: 0,
            messages: vec![OutputMessage {
                is_success: false,
                message_id: message.into(),
                parameters: Vec::new(),
            }],
        }
    }
}

impl ProtoEncode for CommandOutput {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.origin.proto_encode(buf);
        buf.put_u8(self.output_type);
        VarUInt32(self.success_count).proto_encode(buf);

        VarUInt32(self.messages.len() as u32).proto_encode(buf);
        for msg in &self.messages {
            buf.put_u8(msg.is_success as u8);
            write_string(buf, &msg.message_id);
            VarUInt32(msg.parameters.len() as u32).proto_encode(buf);
            for param in &msg.parameters {
                write_string(buf, param);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Uuid;
    use bytes::BytesMut;

    fn test_origin() -> CommandOrigin {
        CommandOrigin {
            origin_type: 0,
            uuid: Uuid::ZERO,
            request_id: String::new(),
            player_entity_id: None,
        }
    }

    #[test]
    fn encode_success() {
        let pkt = CommandOutput::success(test_origin(), "commands.generic.success");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should produce some bytes without panicking
        assert!(!buf.is_empty());
    }

    #[test]
    fn encode_failure() {
        let pkt = CommandOutput::failure(test_origin(), "Unknown command");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
    }
}
