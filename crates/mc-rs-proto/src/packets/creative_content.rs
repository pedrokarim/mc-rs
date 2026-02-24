//! CreativeContent (0x91) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;
use crate::types::VarUInt32;

/// Sends the list of items available in creative mode.
///
/// For Phase 0.6, this is always empty (no creative items).
#[derive(Debug, Clone, Default)]
pub struct CreativeContent {
    // Placeholder — will be Vec<CreativeItem> when items are implemented.
    pub item_count: u32,
}

impl ProtoEncode for CreativeContent {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.item_count).proto_encode(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty() {
        let pkt = CreativeContent::default();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0x00);
    }
}
