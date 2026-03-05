//! ModalFormResponse (0x65) — Client → Server.
//!
//! The client's response to a ModalFormRequest.

use bytes::Buf;

use crate::codec::{self, ProtoDecode};
use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Client response to a form.
#[derive(Debug, Clone)]
pub struct ModalFormResponse {
    /// The form ID from the original request.
    pub form_id: u32,
    /// Whether response data is present.
    pub has_response_data: bool,
    /// The response data (JSON string). Present only if `has_response_data` is true.
    pub response_data: Option<String>,
    /// Whether a cancel reason is present.
    pub has_cancel_reason: bool,
    /// Cancel reason byte. Present only if `has_cancel_reason` is true.
    pub cancel_reason: Option<u8>,
}

impl ProtoDecode for ModalFormResponse {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let form_id = VarUInt32::proto_decode(buf)?.0;

        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: 0,
            });
        }
        let has_response_data = buf.get_u8() != 0;

        let response_data = if has_response_data {
            Some(codec::read_string(buf)?)
        } else {
            None
        };

        let has_cancel_reason = if buf.remaining() >= 1 {
            buf.get_u8() != 0
        } else {
            false
        };

        let cancel_reason = if has_cancel_reason && buf.remaining() >= 1 {
            Some(buf.get_u8())
        } else {
            None
        };

        Ok(Self {
            form_id,
            has_response_data,
            response_data,
            has_cancel_reason,
            cancel_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn decode_with_response() {
        let mut buf = BytesMut::new();
        // form_id = 5
        buf.put_u8(5);
        // has_response_data = true
        buf.put_u8(1);
        // response_data = "42"
        buf.put_u8(2);
        buf.put_slice(b"42");
        // has_cancel_reason = false
        buf.put_u8(0);

        let pkt = ModalFormResponse::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.form_id, 5);
        assert!(pkt.has_response_data);
        assert_eq!(pkt.response_data.as_deref(), Some("42"));
        assert!(!pkt.has_cancel_reason);
        assert!(pkt.cancel_reason.is_none());
    }

    #[test]
    fn decode_cancelled() {
        let mut buf = BytesMut::new();
        // form_id = 10
        buf.put_u8(10);
        // has_response_data = false
        buf.put_u8(0);
        // has_cancel_reason = true
        buf.put_u8(1);
        // cancel_reason = 0 (closed by user)
        buf.put_u8(0);

        let pkt = ModalFormResponse::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.form_id, 10);
        assert!(!pkt.has_response_data);
        assert!(pkt.response_data.is_none());
        assert!(pkt.has_cancel_reason);
        assert_eq!(pkt.cancel_reason, Some(0));
    }
}
