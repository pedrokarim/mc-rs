//! ItemRegistryPacket (0xA2) — Server → Client.
//!
//! Replaces the item_table that was previously in StartGame (protocol 924+).

use bytes::BufMut;

use crate::codec::{self, ProtoEncode};
use crate::types::{VarInt, VarUInt32};

/// Empty NBT compound in network format.
const EMPTY_NBT_COMPOUND: &[u8] = &[0x0A, 0x00, 0x00];

/// An entry in the item registry.
#[derive(Debug, Clone)]
pub struct ItemRegistryEntry {
    pub string_id: String,
    pub numeric_id: i16,
    pub is_component_based: bool,
    pub version: i32,
    pub component_nbt: Vec<u8>, // raw NBT bytes, default empty compound
}

/// ItemRegistryPacket (0xA2) — Server → Client.
#[derive(Debug, Clone)]
pub struct ItemRegistry {
    pub entries: Vec<ItemRegistryEntry>,
}

impl ProtoEncode for ItemRegistry {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.entries.len() as u32).proto_encode(buf);
        for entry in &self.entries {
            codec::write_string(buf, &entry.string_id);
            buf.put_i16_le(entry.numeric_id);
            buf.put_u8(entry.is_component_based as u8);
            VarInt(entry.version).proto_encode(buf);
            if entry.component_nbt.is_empty() {
                buf.put_slice(EMPTY_NBT_COMPOUND);
            } else {
                buf.put_slice(&entry.component_nbt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty_registry() {
        let pkt = ItemRegistry {
            entries: Vec::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(buf[0], 0x00); // VarUInt32(0)
    }

    #[test]
    fn encode_single_entry() {
        let pkt = ItemRegistry {
            entries: vec![ItemRegistryEntry {
                string_id: "minecraft:stone".into(),
                numeric_id: 1,
                is_component_based: false,
                version: 0,
                component_nbt: Vec::new(),
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
    }

    #[test]
    fn encode_entry_with_custom_nbt() {
        let custom_nbt = vec![0x0A, 0x00, 0x00]; // empty compound
        let pkt = ItemRegistry {
            entries: vec![ItemRegistryEntry {
                string_id: "minecraft:diamond".into(),
                numeric_id: 264,
                is_component_based: true,
                version: 2,
                component_nbt: custom_nbt.clone(),
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // Should contain the custom NBT bytes at the end
        assert!(buf.len() > 3);
        assert_eq!(&buf[buf.len() - 3..], &custom_nbt[..]);
    }

    #[test]
    fn encode_multiple_entries() {
        let pkt = ItemRegistry {
            entries: vec![
                ItemRegistryEntry {
                    string_id: "minecraft:stone".into(),
                    numeric_id: 1,
                    is_component_based: false,
                    version: 0,
                    component_nbt: Vec::new(),
                },
                ItemRegistryEntry {
                    string_id: "minecraft:dirt".into(),
                    numeric_id: 3,
                    is_component_based: false,
                    version: 1,
                    component_nbt: Vec::new(),
                },
            ],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // First byte should be VarUInt32(2)
        assert_eq!(buf[0], 0x02);
    }
}
