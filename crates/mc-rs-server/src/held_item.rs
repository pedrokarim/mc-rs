//! Held item swap / hotbar slot.

#[derive(Debug, Clone)]
pub struct HeldItemState {
    pub slot: u8, // 0-8
    pub item: Option<(u16, u16, u16)>, // (id, count, metadata)
}

impl HeldItemState {
    pub fn new() -> Self {
        Self { slot: 0, item: None }
    }

    pub fn set_slot(&mut self, slot: u8) -> bool {
        if slot > 8 {
            return false;
        }
        self.slot = slot;
        true
    }

    pub fn scroll_next(&mut self) {
        self.slot = (self.slot + 1) % 9;
    }

    pub fn scroll_prev(&mut self) {
        self.slot = if self.slot == 0 { 8 } else { self.slot - 1 };
    }
}

impl Default for HeldItemState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_wraps_around() {
        let mut h = HeldItemState::new();
        h.slot = 8;
        h.scroll_next();
        assert_eq!(h.slot, 0);
    }
}
