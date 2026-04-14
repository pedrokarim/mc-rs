//! Llama + TraderLlama — spit attack, carpet equip.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaVariant {
    Creamy,
    White,
    Brown,
    Gray,
    TraderLlama, // Grey with purple carpet
}

#[derive(Debug, Clone)]
pub struct Llama {
    pub variant: LlamaVariant,
    pub age: i32,
    pub tamed: bool,
    pub carpet: Option<&'static str>,
    pub chest: bool,
    pub spit_cooldown: u32,
    pub strength: u8, // 1-5 (chest slots)
    pub caravan_leader: Option<u64>,
}

/// Spit cooldown (60 ticks).
pub const SPIT_COOLDOWN: u32 = 60;
/// Spit damage.
pub const SPIT_DAMAGE: f32 = 1.0;
/// Breeding = hay bale.
pub const BREEDING_ITEM: u16 = 170;

impl Llama {
    pub fn new(variant: LlamaVariant) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            variant,
            age: 0,
            tamed: false,
            carpet: None,
            chest: false,
            spit_cooldown: 0,
            strength: rng.gen_range(1..=5),
            caravan_leader: None,
        }
    }

    pub fn add_carpet(&mut self, color: &'static str) -> bool {
        if !self.tamed {
            return false;
        }
        self.carpet = Some(color);
        true
    }

    pub fn put_chest(&mut self) -> bool {
        if !self.tamed || self.chest {
            return false;
        }
        self.chest = true;
        true
    }

    /// Inventory slots = strength * 3.
    pub fn inventory_slots(&self) -> u8 {
        if !self.chest {
            return 0;
        }
        self.strength * 3
    }

    pub fn try_spit(&mut self, target: u64) -> bool {
        if self.spit_cooldown > 0 {
            return false;
        }
        self.spit_cooldown = SPIT_COOLDOWN;
        let _ = target;
        true
    }

    pub fn tick(&mut self) {
        if self.age < 0 {
            self.age += 1;
        }
        if self.spit_cooldown > 0 {
            self.spit_cooldown -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_requires_tame() {
        let mut l = Llama::new(LlamaVariant::Creamy);
        assert!(!l.put_chest());
        l.tamed = true;
        assert!(l.put_chest());
    }

    #[test]
    fn strength_affects_slots() {
        let mut l = Llama::new(LlamaVariant::Creamy);
        l.tamed = true;
        l.strength = 3;
        l.put_chest();
        assert_eq!(l.inventory_slots(), 9);
    }
}
