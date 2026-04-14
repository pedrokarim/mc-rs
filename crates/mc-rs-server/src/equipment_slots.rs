//! Equipment slot identifiers — unification cross-entity.

use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Head,
    Chest,
    Legs,
    Feet,
    Body, // horse armor
}

impl EquipmentSlot {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::MainHand => "mainhand",
            Self::OffHand => "offhand",
            Self::Head => "head",
            Self::Chest => "chest",
            Self::Legs => "legs",
            Self::Feet => "feet",
            Self::Body => "body",
        }
    }

    /// Slot value in Bedrock inventory protocol.
    pub fn armor_slot_index(&self) -> Option<usize> {
        match self {
            Self::Head => Some(0),
            Self::Chest => Some(1),
            Self::Legs => Some(2),
            Self::Feet => Some(3),
            _ => None,
        }
    }
}

/// Détecte le slot équipable d'un item donné (armor_info lookup + bow/shield).
pub fn detect_slot_for_item(item: &ItemStack) -> Option<EquipmentSlot> {
    use crate::armor::{armor_info, SLOT_CHEST, SLOT_FEET, SLOT_HEAD, SLOT_LEGS};
    if let Some((_, slot)) = armor_info(item.id) {
        return match slot {
            SLOT_HEAD => Some(EquipmentSlot::Head),
            SLOT_CHEST => Some(EquipmentSlot::Chest),
            SLOT_LEGS => Some(EquipmentSlot::Legs),
            SLOT_FEET => Some(EquipmentSlot::Feet),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_slot_is_0() {
        assert_eq!(EquipmentSlot::Head.armor_slot_index(), Some(0));
    }

    #[test]
    fn feet_slot_is_3() {
        assert_eq!(EquipmentSlot::Feet.armor_slot_index(), Some(3));
    }
}
