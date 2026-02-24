//! AvailableEntityIdentifiers (0x78) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;

/// Canonical entity identifiers extracted from pmmp/BedrockData (1.21.50).
const CANONICAL_ENTITY_IDENTIFIERS: &[u8] = include_bytes!("../../data/entity_identifiers.nbt");

/// Sends available entity type identifiers as a raw network NBT blob.
#[derive(Debug, Clone)]
pub struct AvailableEntityIdentifiers {
    pub nbt_data: &'static [u8],
}

impl AvailableEntityIdentifiers {
    /// Create with the embedded canonical entity identifiers.
    pub fn canonical() -> Self {
        Self {
            nbt_data: CANONICAL_ENTITY_IDENTIFIERS,
        }
    }
}

impl Default for AvailableEntityIdentifiers {
    fn default() -> Self {
        Self::canonical()
    }
}

impl ProtoEncode for AvailableEntityIdentifiers {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_slice(self.nbt_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn canonical_data_is_nbt() {
        let pkt = AvailableEntityIdentifiers::canonical();
        // First byte should be TAG_Compound (0x0A)
        assert_eq!(pkt.nbt_data[0], 0x0A);
        assert!(pkt.nbt_data.len() > 100, "entity data too small");
    }

    #[test]
    fn encode_produces_output() {
        let pkt = AvailableEntityIdentifiers::canonical();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), CANONICAL_ENTITY_IDENTIFIERS.len());
    }
}
