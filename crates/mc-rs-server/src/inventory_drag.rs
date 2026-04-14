//! Inventory drag operations (split/fill across slots).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    SplitEven,    // left-click drag: distribute evenly
    PickUpOne,    // right-click drag: 1 per slot
    CreativeCopy, // middle-click (creative only)
}

#[derive(Debug, Clone)]
pub struct DragOperation {
    pub mode: DragMode,
    pub slots: Vec<u8>,
    pub source_stack: (u16, u16), // (id, count)
}

impl DragOperation {
    /// Compute how many items go to each slot.
    pub fn items_per_slot(&self) -> u16 {
        if self.slots.is_empty() {
            return 0;
        }
        match self.mode {
            DragMode::SplitEven => self.source_stack.1 / self.slots.len() as u16,
            DragMode::PickUpOne => 1,
            DragMode::CreativeCopy => self.source_stack.1, // fills each slot with max
        }
    }

    pub fn remainder(&self) -> u16 {
        match self.mode {
            DragMode::SplitEven => self.source_stack.1 % self.slots.len() as u16,
            DragMode::PickUpOne => self.source_stack.1 - self.slots.len() as u16,
            DragMode::CreativeCopy => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_even_distribution() {
        let op = DragOperation {
            mode: DragMode::SplitEven,
            slots: vec![0, 1, 2],
            source_stack: (1, 9),
        };
        assert_eq!(op.items_per_slot(), 3);
    }

    #[test]
    fn split_with_remainder() {
        let op = DragOperation {
            mode: DragMode::SplitEven,
            slots: vec![0, 1, 2],
            source_stack: (1, 10),
        };
        assert_eq!(op.remainder(), 1);
    }
}
