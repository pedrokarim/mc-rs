//! Enchanted books — store enchantments via NBT.

use crate::enchantments::EnchantmentKind;

#[derive(Debug, Clone)]
pub struct EnchantedBook {
    pub stored_enchantments: Vec<(EnchantmentKind, u8)>,
}

impl EnchantedBook {
    pub fn new() -> Self {
        Self {
            stored_enchantments: Vec::new(),
        }
    }

    pub fn add_enchantment(&mut self, kind: EnchantmentKind, level: u8) {
        if let Some(existing) = self
            .stored_enchantments
            .iter_mut()
            .find(|(k, _)| *k == kind)
        {
            existing.1 = existing.1.max(level);
        } else {
            self.stored_enchantments.push((kind, level));
        }
    }

    /// Bookshelf weight for villager trades (high = harder to get).
    pub fn villager_emerald_cost(level: u8, treasure: bool) -> u32 {
        let base = level as u32 * 5;
        if treasure {
            base * 2
        } else {
            base
        }
    }
}

impl Default for EnchantedBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_enchant_keeps_max_level() {
        let mut b = EnchantedBook::new();
        b.add_enchantment(EnchantmentKind::Sharpness, 3);
        b.add_enchantment(EnchantmentKind::Sharpness, 5);
        assert_eq!(b.stored_enchantments.len(), 1);
        assert_eq!(b.stored_enchantments[0].1, 5);
    }
}
