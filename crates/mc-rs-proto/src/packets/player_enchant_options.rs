//! PlayerEnchantOptions (0x92) — Server → Client.
//!
//! Sent when a player places an enchantable item in an enchanting table.
//! Contains up to 3 enchantment options the player can choose from.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarUInt32;

/// A single enchantment entry (id + level).
#[derive(Debug, Clone)]
pub struct EnchantData {
    pub id: u8,
    pub level: u8,
}

/// A single enchantment option offered to the player.
#[derive(Debug, Clone)]
pub struct EnchantOptionEntry {
    /// XP level cost for this option.
    pub cost: u32,
    /// Bitmask of slot flags.
    pub slot_flags: u32,
    /// Enchantments activated when equipped.
    pub equip_enchantments: Vec<EnchantData>,
    /// Enchantments activated when held.
    pub held_enchantments: Vec<EnchantData>,
    /// Enchantments activated on self.
    pub self_enchantments: Vec<EnchantData>,
    /// Display name for the option.
    pub name: String,
    /// Unique option ID (used in CraftRecipeOptional to select).
    pub option_id: u32,
}

/// PlayerEnchantOptions packet.
pub struct PlayerEnchantOptions {
    pub options: Vec<EnchantOptionEntry>,
}

fn encode_enchant_list(buf: &mut impl BufMut, list: &[EnchantData]) {
    VarUInt32(list.len() as u32).proto_encode(buf);
    for entry in list {
        buf.put_u8(entry.id);
        buf.put_u8(entry.level);
    }
}

impl ProtoEncode for PlayerEnchantOptions {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarUInt32(self.options.len() as u32).proto_encode(buf);
        for opt in &self.options {
            VarUInt32(opt.cost).proto_encode(buf);
            buf.put_u32_le(opt.slot_flags);
            encode_enchant_list(buf, &opt.equip_enchantments);
            encode_enchant_list(buf, &opt.held_enchantments);
            encode_enchant_list(buf, &opt.self_enchantments);
            write_string(buf, &opt.name);
            VarUInt32(opt.option_id).proto_encode(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_empty_options() {
        let pkt = PlayerEnchantOptions {
            options: Vec::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // VarUInt32(0) = 1 byte
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn encode_single_option() {
        let pkt = PlayerEnchantOptions {
            options: vec![EnchantOptionEntry {
                cost: 3,
                slot_flags: 0x01,
                equip_enchantments: vec![EnchantData { id: 9, level: 2 }],
                held_enchantments: Vec::new(),
                self_enchantments: Vec::new(),
                name: "test".to_string(),
                option_id: 42,
            }],
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        // count(1) + cost(1) + flags(4) + equip_list(1+2) + held_list(1) + self_list(1)
        // + name_len(1) + "test"(4) + option_id(1)
        assert!(buf.len() > 10);
        // First byte: count = 1
        assert_eq!(buf[0], 1);
    }
}
