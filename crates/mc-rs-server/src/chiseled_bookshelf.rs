//! Chiseled bookshelf — 6-slot book display.

#[derive(Debug, Clone)]
pub struct ChiseledBookshelf {
    pub slots: [Option<BookSlotItem>; 6],
    pub last_interacted_slot: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookSlotItem {
    Book,
    EnchantedBook,
    WrittenBook,
    WritableBook,
}

impl ChiseledBookshelf {
    pub fn new() -> Self {
        Self {
            slots: [None; 6],
            last_interacted_slot: None,
        }
    }

    pub fn place_book(&mut self, idx: usize, kind: BookSlotItem) -> bool {
        if idx >= 6 || self.slots[idx].is_some() {
            return false;
        }
        self.slots[idx] = Some(kind);
        self.last_interacted_slot = Some(idx as u8);
        true
    }

    pub fn take_book(&mut self, idx: usize) -> Option<BookSlotItem> {
        if idx >= 6 {
            return None;
        }
        let b = self.slots[idx].take();
        if b.is_some() {
            self.last_interacted_slot = Some(idx as u8);
        }
        b
    }

    /// Redstone output = slot of last interaction + 1 (or 0 if none).
    pub fn comparator_output(&self) -> u8 {
        match self.last_interacted_slot {
            Some(i) => i + 1,
            None => 0,
        }
    }
}

impl Default for ChiseledBookshelf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparator_outputs_slot() {
        let mut s = ChiseledBookshelf::new();
        s.place_book(3, BookSlotItem::Book);
        assert_eq!(s.comparator_output(), 4);
    }
}
