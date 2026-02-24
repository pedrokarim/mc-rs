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
        // EnumValues, ChainedSubCommands, Suffixes, Enums, ChainedSubCmds, Commands
        for _ in 0..6 {
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
        // 6 × VarUInt32(0) = 6 bytes (each 0 encodes as a single 0x00 byte)
        assert_eq!(buf.len(), 6);
        assert!(buf.iter().all(|&b| b == 0));
    }
}
