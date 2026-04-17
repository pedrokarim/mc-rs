//! Dispenser / Dropper — port PMMP.

use crate::piston::Facing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispenserKind {
    Dispenser, // Can shoot items (arrows, fire charges)
    Dropper,   // Only drops items
}

#[derive(Debug, Clone)]
pub struct Dispenser {
    pub kind: DispenserKind,
    pub facing: Facing,
    pub triggered: bool,
    pub inventory: Vec<Option<(u16, u16)>>, // (id, count) 9 slots
    pub last_fired_tick: u64,
}

/// Dispenser has 9 slots.
pub const SLOTS: usize = 9;
/// Trigger cooldown (2 ticks).
pub const TRIGGER_COOLDOWN: u32 = 2;

/// Items that dispenser treats specially (shoots/uses).
pub fn special_items() -> &'static [&'static str] {
    &[
        "minecraft:arrow",
        "minecraft:tipped_arrow",
        "minecraft:spectral_arrow",
        "minecraft:fire_charge",
        "minecraft:snowball",
        "minecraft:egg",
        "minecraft:potion",
        "minecraft:lingering_potion",
        "minecraft:splash_potion",
        "minecraft:experience_bottle",
        "minecraft:flint_and_steel",
        "minecraft:bone_meal",
        "minecraft:shears",
        "minecraft:trident",
        "minecraft:water_bucket",
        "minecraft:lava_bucket",
        "minecraft:empty_bucket",
        "minecraft:tnt",
    ]
}

impl Dispenser {
    pub fn new(kind: DispenserKind, facing: Facing) -> Self {
        Self {
            kind,
            facing,
            triggered: false,
            inventory: vec![None; SLOTS],
            last_fired_tick: 0,
        }
    }

    pub fn trigger(&mut self) {
        self.triggered = true;
    }

    /// Pick random non-empty slot.
    pub fn pick_random_slot(&self) -> Option<usize> {
        use rand::seq::IteratorRandom;
        let mut rng = rand::thread_rng();
        self.inventory
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|_| i))
            .choose(&mut rng)
    }

    pub fn dispense(&mut self) -> Option<(u16, u16)> {
        let slot = self.pick_random_slot()?;
        if let Some((id, count)) = self.inventory[slot].as_mut() {
            let id_val = *id;
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.inventory[slot] = None;
            }
            return Some((id_val, 1));
        }
        None
    }

    pub fn is_special(item: &str) -> bool {
        special_items().contains(&item)
    }

    /// Only Dispenser actually "shoots" specials; dropper just drops.
    pub fn uses_special_items(&self) -> bool {
        self.kind == DispenserKind::Dispenser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_is_special() {
        assert!(Dispenser::is_special("minecraft:arrow"));
    }

    #[test]
    fn stone_not_special() {
        assert!(!Dispenser::is_special("minecraft:stone"));
    }

    #[test]
    fn dispense_empty_returns_none() {
        let mut d = Dispenser::new(DispenserKind::Dropper, Facing::Up);
        assert!(d.dispense().is_none());
    }
}
