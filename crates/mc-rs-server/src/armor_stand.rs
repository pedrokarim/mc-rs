//! Armor stand — pose editor + equipment.

#[derive(Debug, Clone, Copy)]
pub struct PoseRotation {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[derive(Debug, Clone)]
pub struct ArmorStand {
    pub head_pose: PoseRotation,
    pub body_pose: PoseRotation,
    pub left_arm_pose: PoseRotation,
    pub right_arm_pose: PoseRotation,
    pub left_leg_pose: PoseRotation,
    pub right_leg_pose: PoseRotation,
    pub has_base_plate: bool,
    pub has_arms: bool,
    pub is_small: bool,
    pub visible: bool,
    pub gravity: bool,
    pub invulnerable: bool,
    pub helmet: Option<u16>,
    pub chestplate: Option<u16>,
    pub leggings: Option<u16>,
    pub boots: Option<u16>,
    pub main_hand: Option<u16>,
    pub off_hand: Option<u16>,
}

impl PoseRotation {
    pub const ZERO: Self = Self {
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
    };
}

impl ArmorStand {
    pub fn new() -> Self {
        Self {
            head_pose: PoseRotation::ZERO,
            body_pose: PoseRotation::ZERO,
            left_arm_pose: PoseRotation::ZERO,
            right_arm_pose: PoseRotation::ZERO,
            left_leg_pose: PoseRotation::ZERO,
            right_leg_pose: PoseRotation::ZERO,
            has_base_plate: true,
            has_arms: false,
            is_small: false,
            visible: true,
            gravity: true,
            invulnerable: false,
            helmet: None,
            chestplate: None,
            leggings: None,
            boots: None,
            main_hand: None,
            off_hand: None,
        }
    }

    pub fn equip(&mut self, slot: ArmorSlot, item: Option<u16>) -> Option<u16> {
        use ArmorSlot::*;
        let prev = match slot {
            Helmet => self.helmet.take(),
            Chestplate => self.chestplate.take(),
            Leggings => self.leggings.take(),
            Boots => self.boots.take(),
            MainHand => self.main_hand.take(),
            OffHand => self.off_hand.take(),
        };
        match slot {
            Helmet => self.helmet = item,
            Chestplate => self.chestplate = item,
            Leggings => self.leggings = item,
            Boots => self.boots = item,
            MainHand => self.main_hand = item,
            OffHand => self.off_hand = item,
        }
        prev
    }
}

impl Default for ArmorStand {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    MainHand,
    OffHand,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_base_plate() {
        assert!(ArmorStand::new().has_base_plate);
    }

    #[test]
    fn equip_returns_prev() {
        let mut a = ArmorStand::new();
        a.equip(ArmorSlot::Helmet, Some(1));
        assert_eq!(a.equip(ArmorSlot::Helmet, Some(2)), Some(1));
    }
}
