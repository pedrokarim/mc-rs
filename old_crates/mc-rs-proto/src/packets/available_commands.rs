//! AvailableCommands (0x4C) — Server → Client.
//!
//! Sends the full command tree for client-side autocompletion.
//! The real format is extremely complex (enums, overloads, constraints).
//! We send an empty stub: 6× VarUInt32(0).

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarUInt32;

/// Empty stub — no autocompletion, but commands typed manually work fine.
#[derive(Debug, Clone)]
pub struct AvailableCommands;

impl ProtoEncode for AvailableCommands {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        // Must match PocketMine's 8 arrays:
        // enumValues, chainedSubCommandValues, postfixes, enums,
        // chainedSubCommandData, commandData, softEnums, enumConstraints
        for _ in 0..8 {
            VarUInt32(0).proto_encode(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty_stub() {
        let mut buf = BytesMut::new();
        AvailableCommands.proto_encode(&mut buf);
        // 8 × VarUInt32(0) = 8 bytes (each 0 encodes as a single 0x00 byte)
        assert_eq!(buf.len(), 8);
        assert!(buf.iter().all(|&b| b == 0));
    }
}
