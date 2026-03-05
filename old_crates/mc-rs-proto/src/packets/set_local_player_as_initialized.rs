//! SetLocalPlayerAsInitialized (0x71) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::VarUInt64;

/// Sent by the client after receiving chunks and being ready to play.
#[derive(Debug, Clone)]
pub struct SetLocalPlayerAsInitialized {
    pub entity_runtime_id: u64,
}

impl ProtoDecode for SetLocalPlayerAsInitialized {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let entity_runtime_id = VarUInt64::proto_decode(buf)?.0;
        Ok(Self { entity_runtime_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::ProtoEncode;
    use bytes::BytesMut;

    #[test]
    fn decode_runtime_id() {
        let mut buf = BytesMut::new();
        VarUInt64(42).proto_encode(&mut buf);
        let pkt = SetLocalPlayerAsInitialized::proto_decode(&mut buf.freeze().as_ref()).unwrap();
        assert_eq!(pkt.entity_runtime_id, 42);
    }
}
