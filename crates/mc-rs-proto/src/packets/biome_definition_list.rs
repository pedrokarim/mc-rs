//! BiomeDefinitionList (0x7B) — Server → Client.

use bytes::BufMut;

use crate::codec::ProtoEncode;

/// Canonical biome definitions extracted from pmmp/BedrockData (1.21.50).
const CANONICAL_BIOME_DEFINITIONS: &[u8] = include_bytes!("../../data/biome_definitions.nbt");

/// Sends biome definitions as a raw network NBT blob.
#[derive(Debug, Clone)]
pub struct BiomeDefinitionList {
    pub nbt_data: &'static [u8],
}

impl BiomeDefinitionList {
    /// Create with the embedded canonical biome definitions.
    pub fn canonical() -> Self {
        Self {
            nbt_data: CANONICAL_BIOME_DEFINITIONS,
        }
    }
}

impl Default for BiomeDefinitionList {
    fn default() -> Self {
        Self::canonical()
    }
}

impl ProtoEncode for BiomeDefinitionList {
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
        let pkt = BiomeDefinitionList::canonical();
        // First byte should be TAG_Compound (0x0A)
        assert_eq!(pkt.nbt_data[0], 0x0A);
        assert!(pkt.nbt_data.len() > 100, "biome data too small");
    }

    #[test]
    fn encode_produces_output() {
        let pkt = BiomeDefinitionList::canonical();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf.len(), CANONICAL_BIOME_DEFINITIONS.len());
    }
}
