//! ContainerClose (0x2F) — Bidirectional.
//!
//! Closes a container window. Sent by the client when the player closes
//! a container, or by the server to force-close a container.

use bytes::{Buf, BufMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;

/// Closes a container window.
#[derive(Debug, Clone)]
pub struct ContainerClose {
    /// The window ID of the container to close.
    pub window_id: u8,
    /// Whether the server initiated this close.
    pub server_initiated: bool,
}

impl ProtoEncode for ContainerClose {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.window_id);
        buf.put_u8(self.server_initiated as u8);
    }
}

impl ProtoDecode for ContainerClose {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 2 {
            return Err(ProtoError::BufferTooShort {
                needed: 2,
                remaining: buf.remaining(),
            });
        }
        let window_id = buf.get_u8();
        let server_initiated = buf.get_u8() != 0;
        Ok(Self {
            window_id,
            server_initiated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_container_close() {
        let pkt = ContainerClose {
            window_id: 3,
            server_initiated: true,
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], 3);
        assert_eq!(buf[1], 1);
    }

    #[test]
    fn decode_container_close() {
        let data: &[u8] = &[5, 0];
        let pkt = ContainerClose::proto_decode(&mut &data[..]).unwrap();
        assert_eq!(pkt.window_id, 5);
        assert!(!pkt.server_initiated);
    }
}
