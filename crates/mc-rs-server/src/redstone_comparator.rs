//! Redstone comparator mode — mesure contenu d'inventaire.
//! Port PMMP `src/block/Comparator.php` (partial).

use crate::item_properties::max_stack_size;
use mc_rs_proto::packets::player::ItemStack;

/// Calcule la fullness d'un inventaire en signal 0-15.
/// Formule : 1 + (sum(item_frac) / total_slots) * 14.
pub fn comparator_signal_from_inventory(items: &[ItemStack]) -> u8 {
    let total = items.len() as f32;
    if total == 0.0 {
        return 0;
    }
    let mut frac = 0.0f32;
    for item in items {
        if item.is_air() {
            continue;
        }
        let name = crate::item_registry::item_name_by_id(item.id).unwrap_or("");
        let stack_max = max_stack_size(name).max(1) as f32;
        frac += item.count as f32 / stack_max;
    }
    if frac == 0.0 {
        0
    } else {
        (1.0 + (frac / total) * 14.0).floor() as u8
    }
}

/// Signal selon block special : cauldron level 0-3, composter level 0-7, etc.
pub fn cauldron_signal(water_level: u8) -> u8 {
    // water level 0-3 → signal 0-3 (linear).
    water_level.min(3)
}

pub fn composter_signal(compost_level: u8) -> u8 {
    // level 0-7, 8 (full) → 15.
    if compost_level == 0 {
        0
    } else if compost_level >= 8 {
        15
    } else {
        compost_level * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_signal_is_0() {
        let items: Vec<ItemStack> = vec![ItemStack::AIR; 27];
        assert_eq!(comparator_signal_from_inventory(&items), 0);
    }

    #[test]
    fn composter_full_signal_15() {
        assert_eq!(composter_signal(8), 15);
    }

    #[test]
    fn cauldron_full_signal_3() {
        assert_eq!(cauldron_signal(3), 3);
    }
}
