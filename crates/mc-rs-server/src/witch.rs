//! Witch — potion thrower + drinks potions pour se heal.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitchPotion {
    Healing,      // Self-heal
    FireResistance, // Self-fire-res
    Speed,        // Self-speed
    WaterBreathing, // Self-water breath
    Harming,      // Throw at target
    Poison,       // Throw at target
    Slowness,     // Throw at target
    Weakness,     // Throw at target
}

#[derive(Debug, Clone)]
pub struct Witch {
    pub drinking_potion: Option<WitchPotion>,
    pub drink_ticks: u32,
    pub attack_cooldown: u32,
    pub target_entity: Option<u64>,
}

/// Drink duration (40 ticks).
pub const DRINK_DURATION: u32 = 40;
/// Throw cooldown.
pub const THROW_COOLDOWN: u32 = 60;

impl Witch {
    pub fn new() -> Self {
        Self {
            drinking_potion: None,
            drink_ticks: 0,
            attack_cooldown: 0,
            target_entity: None,
        }
    }

    pub fn start_drinking(&mut self, potion: WitchPotion) -> bool {
        if self.drinking_potion.is_some() {
            return false;
        }
        self.drinking_potion = Some(potion);
        self.drink_ticks = 0;
        true
    }

    pub fn tick(&mut self) {
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
        if self.drinking_potion.is_some() {
            self.drink_ticks += 1;
            if self.drink_ticks >= DRINK_DURATION {
                self.drinking_potion = None;
                self.drink_ticks = 0;
            }
        }
    }

    pub fn is_drinking(&self) -> bool {
        self.drinking_potion.is_some()
    }

    pub fn throw_potion(&mut self, target: u64, potion: WitchPotion) -> bool {
        if self.is_drinking() || self.attack_cooldown > 0 {
            return false;
        }
        self.target_entity = Some(target);
        self.attack_cooldown = THROW_COOLDOWN;
        let _ = potion;
        true
    }

    /// Damage reduction when drinking (immune to damage increase while drinking).
    pub fn damage_reduction_drinking() -> f32 { 0.15 }
}

impl Default for Witch {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drinking_prevents_throw() {
        let mut w = Witch::new();
        w.start_drinking(WitchPotion::Healing);
        assert!(!w.throw_potion(1, WitchPotion::Harming));
    }

    #[test]
    fn drinking_ends() {
        let mut w = Witch::new();
        w.start_drinking(WitchPotion::Healing);
        for _ in 0..=DRINK_DURATION {
            w.tick();
        }
        assert!(!w.is_drinking());
    }
}
