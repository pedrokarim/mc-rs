//! Transfer (0x55) — Server → Client.
//!
//! Transfers the client to another server.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};

/// Transfer packet.
pub struct Transfer {
    /// Server address (hostname or IP).
    pub server_address: String,
    /// Server port.
    pub port: u16,
}

impl Transfer {
    /// Create a transfer packet.
    pub fn new(server_address: impl Into<String>, port: u16) -> Self {
        Self {
            server_address: server_address.into(),
            port,
        }
    }
}

impl ProtoEncode for Transfer {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        write_string(buf, &self.server_address);
        buf.put_u16_le(self.port);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_transfer() {
        let pkt = Transfer::new("play.example.com", 19132);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.server_address, "play.example.com");
        assert_eq!(pkt.port, 19132);
    }

    #[test]
    fn encode_transfer_custom_port() {
        let pkt = Transfer::new("192.168.1.1", 25565);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.port, 25565);
    }
}
