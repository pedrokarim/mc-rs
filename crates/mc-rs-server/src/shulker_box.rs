//! Shulker Box — stores contents even when broken.

#[derive(Debug, Clone)]
pub struct ShulkerBox {
    pub color: Option<u8>, // None = undyed
    pub facing: u8,
    pub inventory: Vec<Option<(u16, u16)>>,
    pub opening_progress: u8, // 0 = closed, 6 = fully open
}

/// Shulker box has 27 slots.
pub const SLOTS: usize = 27;
/// Open animation length (6 ticks).
pub const OPEN_DURATION: u8 = 6;

impl ShulkerBox {
    pub fn new(color: Option<u8>) -> Self {
        Self {
            color,
            facing: 1, // up default
            inventory: vec![None; SLOTS],
            opening_progress: 0,
        }
    }

    /// Can't stack shulker box filled.
    pub fn is_stackable(&self) -> bool {
        self.inventory.iter().all(|s| s.is_none())
    }

    /// Can't be placed with contents dropped — keeps contents in item form.
    pub fn drops_self_as_item() -> bool {
        true
    }

    /// Shulker boxes can't be put inside other shulker boxes.
    pub fn can_contain_shulker_box() -> bool {
        false
    }

    pub fn open(&mut self) {
        self.opening_progress = self.opening_progress.saturating_add(1).min(OPEN_DURATION);
    }

    pub fn close(&mut self) {
        self.opening_progress = self.opening_progress.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_stackable() {
        let s = ShulkerBox::new(None);
        assert!(s.is_stackable());
    }

    #[test]
    fn full_not_stackable() {
        let mut s = ShulkerBox::new(None);
        s.inventory[0] = Some((1, 1));
        assert!(!s.is_stackable());
    }
}
