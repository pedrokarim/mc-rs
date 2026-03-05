//! ModalFormRequest (0x64) — Server → Client.
//!
//! Sends a form (simple, modal, or custom) to the client as a JSON string.

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::VarUInt32;

/// A server-initiated form request.
#[derive(Debug, Clone)]
pub struct ModalFormRequest {
    /// Unique form ID used to correlate the response.
    pub form_id: u32,
    /// JSON-encoded form data.
    pub form_data: String,
}

impl ProtoEncode for ModalFormRequest {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.form_id).proto_encode(buf);
        codec::write_string(buf, &self.form_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_form_request() {
        let pkt = ModalFormRequest {
            form_id: 1,
            form_data: r#"{"type":"form"}"#.into(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // form_id: VarUInt32(1) = 1 byte
        // form_data: VarUInt32(15) + 15 bytes = 16
        // Total = 17
        assert_eq!(buf.len(), 17);
        // The JSON should appear in the output
        assert!(buf[..].windows(6).any(|w| w == b"\"type\""));
    }
}
