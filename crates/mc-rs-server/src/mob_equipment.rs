//! Mob equipment — port PMMP `src/inventory/EntityInventory.php`.
//! Mobs peuvent équiper armor + main hand + offhand.

use mc_rs_proto::packets::player::ItemStack;

#[derive(Debug, Clone)]
pub struct MobEquipment {
    pub main_hand: ItemStack,
    pub off_hand: ItemStack,
    pub helmet: ItemStack,
    pub chestplate: ItemStack,
    pub leggings: ItemStack,
    pub boots: ItemStack,
}

impl Default for MobEquipment {
    fn default() -> Self {
        Self::empty()
    }
}

impl MobEquipment {
    pub fn empty() -> Self {
        Self {
            main_hand: ItemStack::AIR,
            off_hand: ItemStack::AIR,
            helmet: ItemStack::AIR,
            chestplate: ItemStack::AIR,
            leggings: ItemStack::AIR,
            boots: ItemStack::AIR,
        }
    }

    pub fn has_any(&self) -> bool {
        !self.main_hand.is_air()
            || !self.off_hand.is_air()
            || !self.helmet.is_air()
            || !self.chestplate.is_air()
            || !self.leggings.is_air()
            || !self.boots.is_air()
    }
}

/// Zombie equipment chance (vanilla selon difficulty).
pub fn zombie_armor_chance(difficulty: i32) -> f32 {
    match difficulty {
        0 => 0.0,
        1 => 0.05,
        2 => 0.15,
        _ => 0.25,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_has_no_items() {
        let eq = MobEquipment::empty();
        assert!(!eq.has_any());
    }
}
