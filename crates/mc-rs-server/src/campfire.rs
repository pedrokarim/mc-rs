//! Campfire / soul campfire — cooking block.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampfireKind {
    Normal,   // Light 15, damage 1
    Soul,     // Light 10, damage 2 (slower smoke)
}

#[derive(Debug, Clone)]
pub struct Campfire {
    pub kind: CampfireKind,
    pub lit: bool,
    pub slots: [Option<CampfireSlot>; 4],
    pub signal_boost: bool, // hay bale below
}

#[derive(Debug, Clone)]
pub struct CampfireSlot {
    pub item: u16,
    pub cook_ticks: u32,
    pub cook_total: u32,
}

/// Cook time (600 ticks = 30s) — no fuel needed.
pub const COOK_TIME: u32 = 600;
/// Damage from stepping on.
pub const DAMAGE_NORMAL: f32 = 1.0;
pub const DAMAGE_SOUL: f32 = 2.0;

impl Campfire {
    pub fn new(kind: CampfireKind) -> Self {
        Self {
            kind,
            lit: true,
            slots: [None, None, None, None],
            signal_boost: false,
        }
    }

    pub fn add_item(&mut self, item: u16) -> bool {
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                *slot = Some(CampfireSlot {
                    item,
                    cook_ticks: 0,
                    cook_total: COOK_TIME,
                });
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self) {
        if !self.lit {
            return;
        }
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot {
                s.cook_ticks += 1;
            }
        }
    }

    pub fn damage_value(&self) -> f32 {
        match self.kind {
            CampfireKind::Normal => DAMAGE_NORMAL,
            CampfireKind::Soul => DAMAGE_SOUL,
        }
    }

    /// Signal fire range (hay bale increases smoke distance).
    pub fn smoke_range(&self) -> u32 {
        if self.signal_boost { 24 } else { 8 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_4_items() {
        let mut c = Campfire::new(CampfireKind::Normal);
        for _ in 0..4 {
            assert!(c.add_item(1));
        }
        assert!(!c.add_item(1));
    }
}
