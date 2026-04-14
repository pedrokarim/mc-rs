//! Horse — tameable ridable, variants + Donkey/Mule/ZombieHorse/SkeletonHorse.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorseVariant {
    White, Creamy, Chestnut, Brown, Black, Gray, DarkBrown,
    Donkey, Mule,
    ZombieHorse, SkeletonHorse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorseMarking {
    None, WhiteSocks, WhiteField, WhiteDots, BlackDots,
}

#[derive(Debug, Clone)]
pub struct Horse {
    pub variant: HorseVariant,
    pub marking: HorseMarking,
    pub age: i32,
    pub tamed: bool,
    pub owner: Option<u64>,
    pub temperament: u8, // 0-100 (higher = easier tame)
    pub saddled: bool,
    pub armor: Option<&'static str>, // iron/gold/diamond/leather/netherite
    pub chest: bool, // donkey/mule only
    pub max_hp: f32,
    pub speed: f32, // blocks/sec
    pub jump_strength: f32,
}

impl Horse {
    pub fn new(variant: HorseVariant) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let hp = rng.gen_range(15.0..=30.0);
        let speed = rng.gen_range(4.74..=14.5);
        let jump = rng.gen_range(0.4..=1.0);
        Self {
            variant,
            marking: HorseMarking::None,
            age: 0,
            tamed: false,
            owner: None,
            temperament: 0,
            saddled: false,
            armor: None,
            chest: false,
            max_hp: hp,
            speed,
            jump_strength: jump,
        }
    }

    /// Try tame by riding, chance increases with temperament.
    pub fn try_tame_by_ride(&mut self, owner: u64) -> bool {
        if self.tamed {
            return true;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chance = (self.temperament as f32) / 100.0;
        if rng.gen::<f32>() < chance {
            self.tamed = true;
            self.owner = Some(owner);
            true
        } else {
            self.temperament = (self.temperament + 5).min(100);
            false
        }
    }

    pub fn equip_armor(&mut self, kind: &'static str) -> bool {
        if !self.tamed || !matches!(self.variant,
            HorseVariant::White | HorseVariant::Creamy | HorseVariant::Chestnut |
            HorseVariant::Brown | HorseVariant::Black | HorseVariant::Gray |
            HorseVariant::DarkBrown) {
            return false;
        }
        self.armor = Some(kind);
        true
    }

    pub fn can_wear_armor(&self) -> bool {
        matches!(self.variant,
            HorseVariant::White | HorseVariant::Creamy | HorseVariant::Chestnut |
            HorseVariant::Brown | HorseVariant::Black | HorseVariant::Gray |
            HorseVariant::DarkBrown)
    }

    /// Only donkey/mule can carry chest.
    pub fn can_carry_chest(&self) -> bool {
        matches!(self.variant, HorseVariant::Donkey | HorseVariant::Mule)
    }

    pub fn put_chest(&mut self) -> bool {
        if !self.can_carry_chest() || !self.tamed || self.chest {
            return false;
        }
        self.chest = true;
        true
    }

    /// Feeding items (apple/sugar/wheat/hay bale/golden apple/golden carrot).
    pub fn feed_growth_from(item: &str) -> u32 {
        match item {
            "minecraft:sugar" => 30 * 20,
            "minecraft:wheat" => 60 * 20,
            "minecraft:apple" => 60 * 20,
            "minecraft:golden_carrot" => 60 * 20,
            "minecraft:hay_block" => 180 * 20,
            "minecraft:golden_apple" | "minecraft:enchanted_golden_apple" => 240 * 20,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donkey_can_chest() {
        let h = Horse::new(HorseVariant::Donkey);
        assert!(h.can_carry_chest());
    }

    #[test]
    fn horse_cant_chest() {
        let h = Horse::new(HorseVariant::White);
        assert!(!h.can_carry_chest());
    }

    #[test]
    fn armor_only_on_tamed_normal_horse() {
        let mut h = Horse::new(HorseVariant::Donkey);
        h.tamed = true;
        assert!(!h.equip_armor("iron"));
    }
}
