//! Player inventory slot conventions.

/// Main inventory: 36 slots (3×9 + 1×9 hotbar).
pub const MAIN_INVENTORY_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;
pub const HOTBAR_START: usize = 0;
pub const HOTBAR_END: usize = 8;
pub const INVENTORY_START: usize = 9;
pub const INVENTORY_END: usize = 35;

/// Armor slots (head, chest, legs, feet).
pub const ARMOR_START: usize = 36;
pub const HELMET: usize = 36;
pub const CHESTPLATE: usize = 37;
pub const LEGGINGS: usize = 38;
pub const BOOTS: usize = 39;

/// Off-hand slot.
pub const OFF_HAND: usize = 40;

/// Total player inventory slots.
pub const TOTAL_SLOTS: usize = 41;

pub fn is_hotbar(slot: usize) -> bool {
    slot <= HOTBAR_END
}

pub fn is_armor(slot: usize) -> bool {
    (ARMOR_START..OFF_HAND).contains(&slot)
}

pub fn is_off_hand(slot: usize) -> bool {
    slot == OFF_HAND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotbar_0_8() {
        assert!(is_hotbar(0));
        assert!(is_hotbar(8));
        assert!(!is_hotbar(9));
    }
}
