//! Comportements de containers automatisés — Hopper, Dropper, Dispenser.
//! Port PMMP `src/block/inventory/*`.

use mc_rs_proto::packets::player::ItemStack;

/// Hopper : pull items from container above, push items to container below/side.
/// Tick rate : PMMP `Hopper::TICK_DELAY = 8`.
pub const HOPPER_TICK_DELAY: u32 = 8;

#[derive(Debug, Clone)]
pub struct HopperState {
    pub items: Vec<ItemStack>, // 5 slots
    pub cooldown: u32,
    pub locked: bool,
}

impl HopperState {
    pub fn new() -> Self {
        Self {
            items: vec![ItemStack::AIR; 5],
            cooldown: 0,
            locked: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return false;
        }
        if self.locked {
            return false;
        }
        self.cooldown = HOPPER_TICK_DELAY;
        true // ready to pull/push
    }

    pub fn has_items(&self) -> bool {
        self.items.iter().any(|s| !s.is_air())
    }

    pub fn find_non_empty_slot(&self) -> Option<usize> {
        self.items.iter().position(|s| !s.is_air())
    }

    pub fn find_space_for(&self, item: &ItemStack) -> Option<usize> {
        // Check if any slot can stack.
        for (i, slot) in self.items.iter().enumerate() {
            if slot.is_air() {
                return Some(i);
            }
            if slot.id == item.id && slot.meta == item.meta && slot.count < 64 {
                return Some(i);
            }
        }
        None
    }

    /// Transfer un item du slot `from_slot` vers `target_hopper` si place.
    pub fn try_transfer_to(&mut self, from_slot: usize, target: &mut HopperState) -> bool {
        if from_slot >= self.items.len() || self.items[from_slot].is_air() {
            return false;
        }
        let item = self.items[from_slot].clone();
        if let Some(target_slot) = target.find_space_for(&item) {
            if target.items[target_slot].is_air() {
                let mut moved = item.clone();
                moved.count = 1;
                target.items[target_slot] = moved;
            } else {
                target.items[target_slot].count += 1;
            }
            self.items[from_slot].count -= 1;
            if self.items[from_slot].count == 0 {
                self.items[from_slot] = ItemStack::AIR;
            }
            return true;
        }
        false
    }
}

impl Default for HopperState {
    fn default() -> Self {
        Self::new()
    }
}

/// Dispenser/Dropper : 9 slots, tire un item random quand powered.
#[derive(Debug, Clone)]
pub struct DispenserState {
    pub items: Vec<ItemStack>, // 9 slots
    pub is_dropper: bool,      // dropper ne fait pas d'action spéciale sur items
}

impl DispenserState {
    pub fn new(is_dropper: bool) -> Self {
        Self {
            items: vec![ItemStack::AIR; 9],
            is_dropper,
        }
    }

    /// Retire un item random et le retourne. None si vide.
    pub fn pick_random(&mut self) -> Option<ItemStack> {
        use rand::Rng;
        let non_empty: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_air())
            .map(|(i, _)| i)
            .collect();
        if non_empty.is_empty() {
            return None;
        }
        let idx = non_empty[rand::thread_rng().gen_range(0..non_empty.len())];
        let mut item = self.items[idx].clone();
        let taken = item.clone();
        self.items[idx].count -= 1;
        if self.items[idx].count == 0 {
            self.items[idx] = ItemStack::AIR;
        }
        item.count = 1;
        Some(taken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hopper_ready_after_cooldown() {
        let mut h = HopperState::new();
        assert!(h.tick()); // first tick : ready
        assert!(!h.tick()); // cooldown now 8
    }

    #[test]
    fn hopper_transfers_item() {
        let mut src = HopperState::new();
        let mut dst = HopperState::new();
        src.items[0] = ItemStack::new(3, 5, 0);
        assert!(src.try_transfer_to(0, &mut dst));
        assert_eq!(src.items[0].count, 4);
        assert_eq!(dst.items[0].count, 1);
    }

    #[test]
    fn dispenser_random_pick() {
        let mut d = DispenserState::new(false);
        d.items[3] = ItemStack::new(5, 10, 0);
        let picked = d.pick_random().unwrap();
        assert_eq!(picked.id, 5);
        assert_eq!(d.items[3].count, 9);
    }
}
