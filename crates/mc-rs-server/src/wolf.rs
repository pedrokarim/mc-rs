//! Wolf — peut être apprivoisé, collier dye-able, heal avec meat.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WolfVariant {
    Pale,
    Woods,
    Ashen,
    BlackAndWhite,
    Rusty,
    Snowy,
    Spotted,
    Striped,
    Chestnut,
}

#[derive(Debug, Clone, Copy)]
pub enum CollarColor {
    White, Orange, Magenta, LightBlue, Yellow, Lime, Pink, Gray,
    LightGray, Cyan, Purple, Blue, Brown, Green, Red, Black,
}

#[derive(Debug, Clone)]
pub struct Wolf {
    pub variant: WolfVariant,
    pub age: i32,
    pub tamed: bool,
    pub owner: Option<u64>,
    pub collar: CollarColor,
    pub sitting: bool,
    pub angry_ticks: u32,
    pub hp: f32,
    pub max_hp: f32,
}

/// Untamed taming chance (1/3 per bone).
pub const TAME_CHANCE: f32 = 1.0 / 3.0;
/// Hp untamed (8).
pub const HP_UNTAMED: f32 = 8.0;
/// Hp tamed (40, +10 per level).
pub const HP_TAMED: f32 = 40.0;

impl Wolf {
    pub fn new_wild(variant: WolfVariant) -> Self {
        Self {
            variant,
            age: 0,
            tamed: false,
            owner: None,
            collar: CollarColor::Red,
            sitting: false,
            angry_ticks: 0,
            hp: HP_UNTAMED,
            max_hp: HP_UNTAMED,
        }
    }

    pub fn try_tame(&mut self, owner: u64) -> bool {
        if self.tamed {
            return false;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < TAME_CHANCE {
            self.tamed = true;
            self.owner = Some(owner);
            self.max_hp = HP_TAMED;
            self.hp = HP_TAMED;
            true
        } else {
            false
        }
    }

    pub fn feed_meat(&mut self, heal_amount: f32) {
        self.hp = (self.hp + heal_amount).min(self.max_hp);
    }

    pub fn toggle_sit(&mut self) -> bool {
        if !self.tamed {
            return false;
        }
        self.sitting = !self.sitting;
        true
    }

    pub fn is_hostile(&self) -> bool {
        !self.tamed && self.angry_ticks > 0
    }

    pub fn anger(&mut self) {
        if !self.tamed {
            self.angry_ticks = 400;
        }
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.angry_ticks > 0 {
            self.angry_ticks -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wild_not_owned() {
        let w = Wolf::new_wild(WolfVariant::Pale);
        assert!(w.owner.is_none());
    }

    #[test]
    fn wild_cant_sit() {
        let mut w = Wolf::new_wild(WolfVariant::Pale);
        assert!(!w.toggle_sit());
    }

    #[test]
    fn meat_heals_wolf() {
        let mut w = Wolf::new_wild(WolfVariant::Pale);
        w.hp = 1.0;
        w.feed_meat(3.0);
        assert_eq!(w.hp, 4.0);
    }
}
