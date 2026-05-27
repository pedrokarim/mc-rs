//! Hopper — port PMMP `src/block/Hopper.php`.

use crate::piston::Facing;

#[derive(Debug, Clone)]
pub struct Hopper {
    pub facing: Facing,
    pub inventory: Vec<Option<(u16, u16)>>, // 5 slots
    pub transfer_cooldown: u32,
    pub powered: bool,
}

/// Hopper has 5 slots.
pub const SLOTS: usize = 5;
/// Transfer cooldown (8 ticks = ~0.4s).
pub const TRANSFER_COOLDOWN: u32 = 8;

impl Hopper {
    pub fn new(facing: Facing) -> Self {
        Self {
            facing,
            inventory: vec![None; SLOTS],
            transfer_cooldown: 0,
            powered: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.powered {
            return false;
        }
        if self.transfer_cooldown > 0 {
            self.transfer_cooldown -= 1;
            return false;
        }
        self.transfer_cooldown = TRANSFER_COOLDOWN;
        true
    }

    pub fn is_empty(&self) -> bool {
        self.inventory.iter().all(|s| s.is_none())
    }

    pub fn is_full(&self) -> bool {
        self.inventory.iter().all(|s| match s {
            Some((_, count)) => *count >= 64,
            None => false,
        })
    }

    pub fn first_non_empty_slot(&self) -> Option<usize> {
        self.inventory.iter().position(|s| s.is_some())
    }

    pub fn add_item(&mut self, id: u16, count: u16) -> u16 {
        let mut remaining = count;
        // First try to stack.
        for (sid, scount) in self.inventory.iter_mut().flatten() {
            if *sid == id && *scount < 64 {
                let add = (64 - *scount).min(remaining);
                *scount += add;
                remaining -= add;
                if remaining == 0 {
                    return 0;
                }
            }
        }
        // Then fill empty slots.
        for slot in self.inventory.iter_mut() {
            if slot.is_none() {
                let add = 64.min(remaining);
                *slot = Some((id, add));
                remaining -= add;
                if remaining == 0 {
                    return 0;
                }
            }
        }
        remaining
    }

    pub fn take_first_item(&mut self) -> Option<(u16, u16)> {
        let idx = self.first_non_empty_slot()?;
        let slot = self.inventory[idx].as_mut()?;
        let id = slot.0;
        slot.1 -= 1;
        let taken = slot.1 == 0;
        if taken {
            self.inventory[idx] = None;
        }
        Some((id, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_by_default() {
        let h = Hopper::new(Facing::Down);
        assert!(h.is_empty());
    }

    #[test]
    fn add_item_fills_slots() {
        let mut h = Hopper::new(Facing::Down);
        assert_eq!(h.add_item(1, 64), 0);
        assert!(!h.is_empty());
    }

    #[test]
    fn powered_blocks_transfer() {
        let mut h = Hopper::new(Facing::Down);
        h.powered = true;
        assert!(!h.tick());
    }
}
