//! Crafter — 1.21 automation block.

use crate::piston::Facing;

#[derive(Debug, Clone)]
pub struct Crafter {
    pub facing: Facing,
    pub inventory: [Option<(u16, u16)>; 9],
    pub disabled_slots: [bool; 9],
    pub powered: bool,
    pub craft_recipe: Option<&'static str>,
}

impl Crafter {
    pub fn new(facing: Facing) -> Self {
        Self {
            facing,
            inventory: [None; 9],
            disabled_slots: [false; 9],
            powered: false,
            craft_recipe: None,
        }
    }

    pub fn toggle_slot(&mut self, idx: usize) -> bool {
        if idx >= 9 {
            return false;
        }
        self.disabled_slots[idx] = !self.disabled_slots[idx];
        true
    }

    /// Trigger on redstone pulse.
    pub fn trigger(&mut self) -> bool {
        if self.powered {
            return false;
        }
        self.powered = true;
        true
    }

    pub fn comparator_output(&self) -> u8 {
        let filled = self.inventory.iter().filter(|s| s.is_some()).count();
        let disabled = self.disabled_slots.iter().filter(|&&d| d).count();
        let usable = 9 - disabled;
        if usable == 0 {
            return 0;
        }
        ((filled as f32 / usable as f32) * 15.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_toggle() {
        let mut c = Crafter::new(Facing::Up);
        assert!(c.toggle_slot(0));
        assert!(c.disabled_slots[0]);
    }
}
