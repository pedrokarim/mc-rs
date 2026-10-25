//! Données d'un item (`extra_data` d'un `ItemStack`) : NBT, listes CanPlaceOn et CanDestroy.
//!
//! Format : PMMP `ItemStackExtraData::read` (bedrock-protocol) — i16 LE (−1 : NBT présente, suivie d'un
//! octet de version 1 et d'une NBT little-endian ; 0 : pas de NBT), puis deux listes (u32 LE nombre,
//! chaînes à longueur u16 LE). Les octets suivants (horodatage de blocage du bouclier) sont ignorés.

use mc_rs_nbt::{tag::NbtCompound, NbtTag};
use mc_rs_proto::packets::player::ItemStack;

use crate::enchantments::EnchantmentKind;

/// Contenu décodé de l'`extra_data` d'un item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemExtraData {
    pub nbt: Option<NbtCompound>,
    pub can_place_on: Vec<String>,
    pub can_destroy: Vec<String>,
}

impl ItemExtraData {
    /// Décode l'`extra_data` ; `None` si les octets sont mal formés. Vide = item sans données.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return Some(Self::default());
        }
        let mut buf = bytes;
        let nbt_len = i16::from_le_bytes(take::<2>(&mut buf)?);
        let nbt = match nbt_len {
            -1 => {
                if take::<1>(&mut buf)?[0] != 1 {
                    return None;
                }
                Some(mc_rs_nbt::read_nbt_le(&mut buf).ok()?.compound)
            }
            0 => None,
            _ => return None,
        };
        let can_place_on = read_string_list(&mut buf)?;
        let can_destroy = read_string_list(&mut buf)?;
        Some(Self {
            nbt,
            can_place_on,
            can_destroy,
        })
    }

    /// Niveau d'un enchantement (PMMP `Item::getEnchantmentLevel`) : liste `ench` de compounds
    /// `{id: Short, lvl: Short}`, 0 si absent.
    pub fn enchantment_level(&self, kind: EnchantmentKind) -> u8 {
        let Some(NbtTag::List(list)) = self.nbt.as_ref().and_then(|nbt| nbt.get("ench")) else {
            return 0;
        };
        list.iter()
            .filter_map(|entry| match entry {
                NbtTag::Compound(c) => Some(c),
                _ => None,
            })
            .find(|c| matches!(c.get("id"), Some(NbtTag::Short(id)) if *id == i16::from(kind.id())))
            .and_then(|c| match c.get("lvl") {
                Some(NbtTag::Short(lvl)) => u8::try_from(*lvl).ok(),
                _ => None,
            })
            .unwrap_or(0)
    }
}

/// Niveau d'un enchantement sur une pile d'items, 0 si absent ou données illisibles.
pub fn enchantment_level(stack: &ItemStack, kind: EnchantmentKind) -> u8 {
    ItemExtraData::parse(&stack.extra_data).map_or(0, |data| data.enchantment_level(kind))
}

fn take<const N: usize>(buf: &mut &[u8]) -> Option<[u8; N]> {
    let (head, rest) = buf.split_first_chunk::<N>()?;
    *buf = rest;
    Some(*head)
}

fn read_string_list(buf: &mut &[u8]) -> Option<Vec<String>> {
    let count = u32::from_le_bytes(take::<4>(buf)?);
    let mut out = Vec::new();
    for _ in 0..count {
        let len = usize::from(u16::from_le_bytes(take::<2>(buf)?));
        if buf.len() < len {
            return None;
        }
        let (s, rest) = buf.split_at(len);
        out.push(String::from_utf8_lossy(s).into_owned());
        *buf = rest;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_minimal_extra_data_have_no_nbt() {
        assert_eq!(ItemExtraData::parse(&[]), Some(ItemExtraData::default()));
        assert_eq!(
            ItemExtraData::parse(&[0; 10]),
            Some(ItemExtraData::default())
        );
    }

    #[test]
    fn reads_enchantment_written_by_enchantments_module() {
        let bytes =
            crate::enchantments::build_extra_data_with_enchant(EnchantmentKind::Efficiency.id(), 5);
        let data = ItemExtraData::parse(&bytes).unwrap();
        assert_eq!(data.enchantment_level(EnchantmentKind::Efficiency), 5);
        assert_eq!(data.enchantment_level(EnchantmentKind::SilkTouch), 0);
    }

    #[test]
    fn reads_can_destroy_list() {
        let mut bytes = vec![0, 0];
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(b"stone");
        let data = ItemExtraData::parse(&bytes).unwrap();
        assert_eq!(data.can_destroy, vec!["stone".to_string()]);
    }

    #[test]
    fn truncated_data_is_rejected() {
        assert_eq!(ItemExtraData::parse(&[0xFF, 0xFF]), None);
        assert_eq!(ItemExtraData::parse(&[0, 0, 1, 0, 0, 0]), None);
    }
}
