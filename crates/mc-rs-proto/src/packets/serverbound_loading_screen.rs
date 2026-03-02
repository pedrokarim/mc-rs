//! ServerboundLoadingScreen (0x138) - Client -> Server.
//!
//! Wire format:
//! - type: VarInt
//! - loading_screen_id: Optional<bool + u32_le>

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::VarInt;

/// Sent by the client to report loading screen state.
#[derive(Debug, Clone)]
pub struct ServerboundLoadingScreen {
    pub loading_screen_type: i32,
    pub loading_screen_id: Option<u32>,
}

impl ProtoDecode for ServerboundLoadingScreen {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let loading_screen_type = VarInt::proto_decode(buf)?.0;

        if !buf.has_remaining() {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: 0,
            });
        }
        let has_loading_screen_id = buf.get_u8() != 0;
        let loading_screen_id = if has_loading_screen_id {
            if buf.remaining() < 4 {
                return Err(ProtoError::BufferTooShort {
                    needed: 4,
                    remaining: buf.remaining(),
                });
            }
            Some(buf.get_u32_le())
        } else {
            None
        };

        Ok(Self {
            loading_screen_type,
            loading_screen_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoEncode;
    use bytes::BytesMut;

    #[test]
    fn decode_without_loading_screen_id() {
        let mut buf = BytesMut::new();
        VarInt(1).proto_encode(&mut buf);
        buf.extend_from_slice(&[0]);

        let pkt = ServerboundLoadingScreen::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.loading_screen_type, 1);
        assert_eq!(pkt.loading_screen_id, None);
    }

    #[test]
    fn decode_with_loading_screen_id() {
        let mut buf = BytesMut::new();
        VarInt(2).proto_encode(&mut buf);
        buf.extend_from_slice(&[1]);
        buf.extend_from_slice(&42u32.to_le_bytes());

        let pkt = ServerboundLoadingScreen::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.loading_screen_type, 2);
        assert_eq!(pkt.loading_screen_id, Some(42));
    }
}
