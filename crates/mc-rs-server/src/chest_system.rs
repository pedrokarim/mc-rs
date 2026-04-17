//! Chest system — chest/trapped chest + pairing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestKind {
    Normal,
    Trapped,    // Emits redstone when opened
    EnderChest, // Per-player inventory
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestPair {
    Single,
    Left,  // Part of double, west half
    Right, // Part of double, east half
}

#[derive(Debug, Clone)]
pub struct Chest {
    pub kind: ChestKind,
    pub pair: ChestPair,
    pub facing: u8, // 0=south,1=west,2=north,3=east
    pub inventory: Vec<Option<(u16, u16)>>,
    pub viewers: u32, // player count currently viewing
}

/// Normal chest 27 slots.
pub const NORMAL_SLOTS: usize = 27;
/// Double chest 54 slots.
pub const DOUBLE_SLOTS: usize = 54;

impl Chest {
    pub fn new_single(kind: ChestKind) -> Self {
        Self {
            kind,
            pair: ChestPair::Single,
            facing: 0,
            inventory: vec![None; NORMAL_SLOTS],
            viewers: 0,
        }
    }

    pub fn is_double(&self) -> bool {
        self.pair != ChestPair::Single
    }

    pub fn slot_count(&self) -> usize {
        match self.pair {
            ChestPair::Single => NORMAL_SLOTS,
            _ => DOUBLE_SLOTS,
        }
    }

    pub fn add_viewer(&mut self) {
        self.viewers = self.viewers.saturating_add(1);
    }

    pub fn remove_viewer(&mut self) {
        self.viewers = self.viewers.saturating_sub(1);
    }

    pub fn is_open(&self) -> bool {
        self.viewers > 0
    }

    /// Trapped chest emits redstone based on viewers (1-15).
    pub fn redstone_power(&self) -> u8 {
        if self.kind != ChestKind::Trapped {
            return 0;
        }
        self.viewers.min(15) as u8
    }

    /// Cat blocks chest opening (vanilla PMMP: can't open with cat on top).
    pub fn opening_blocked_by_cat_on_top() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trapped_emits_redstone() {
        let mut c = Chest::new_single(ChestKind::Trapped);
        c.add_viewer();
        assert_eq!(c.redstone_power(), 1);
    }

    #[test]
    fn normal_no_redstone() {
        let mut c = Chest::new_single(ChestKind::Normal);
        c.add_viewer();
        assert_eq!(c.redstone_power(), 0);
    }

    #[test]
    fn double_has_54_slots() {
        let mut c = Chest::new_single(ChestKind::Normal);
        c.pair = ChestPair::Left;
        assert_eq!(c.slot_count(), DOUBLE_SLOTS);
    }
}
