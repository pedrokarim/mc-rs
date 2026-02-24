//! Disconnect (0x05) — Server → Client.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::VarInt;

/// Sent by the server to disconnect a client.
#[derive(Debug, Clone)]
pub struct Disconnect {
    /// Disconnect reason code.
    pub reason: i32,
    /// If true, the client shows no disconnect screen.
    pub hide_disconnect_screen: bool,
    /// Message shown on the disconnect screen (only if `hide_disconnect_screen` is false).
    pub message: Option<String>,
}

impl Disconnect {
    /// Create a disconnect with a visible message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            reason: 0,
            hide_disconnect_screen: false,
            message: Some(message.into()),
        }
    }

    /// Create a silent disconnect (no screen shown).
    pub fn silent() -> Self {
        Self {
            reason: 0,
            hide_disconnect_screen: true,
            message: None,
        }
    }
}

impl ProtoEncode for Disconnect {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.reason).proto_encode(buf);
        buf.put_u8(self.hide_disconnect_screen as u8);
        if !self.hide_disconnect_screen {
            if let Some(ref msg) = self.message {
                codec::write_string(buf, msg);
            } else {
                codec::write_string(buf, "");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_with_message() {
        let pkt = Disconnect::with_message("Server closed");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // reason=0 (1 byte VarInt) + hide=0 (1 byte) + string
        assert_eq!(buf[0], 0x00); // VarInt(0) = 0x00
        assert_eq!(buf[1], 0x00); // hide = false
                                  // Rest is VarUInt32 length + "Server closed"
        assert!(buf.len() > 2);
    }

    #[test]
    fn encode_silent() {
        let pkt = Disconnect::silent();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // reason=0 (1 byte) + hide=1 (1 byte), no message
        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], 0x00);
        assert_eq!(buf[1], 0x01);
    }

    #[test]
    fn encode_with_empty_message() {
        let pkt = Disconnect {
            reason: 0,
            hide_disconnect_screen: false,
            message: None,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // reason=0 + hide=0 + VarUInt32(0) for empty string
        assert_eq!(buf[0], 0x00);
        assert_eq!(buf[1], 0x00);
        assert_eq!(buf[2], 0x00); // VarUInt32(0)
        assert_eq!(buf.len(), 3);
    }
}
