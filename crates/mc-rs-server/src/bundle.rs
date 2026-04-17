//! Bundle item — holds up to 64 item counts across multiple stacks.

#[derive(Debug, Clone)]
pub struct Bundle {
    pub items: Vec<(u16, u16, u16)>, // (id, count, data)
    pub color: Option<u8>,
}

/// Bundle capacity (64 total items of standard stack size).
pub const CAPACITY: u32 = 64;

/// Each item takes some fraction of capacity.
/// 64-stackable = 1/64 each; 16-stackable = 4 each; 1-stackable = 64 each.
pub fn item_weight(max_stack: u16) -> u32 {
    match max_stack {
        64 => 1,
        16 => 4,
        1 => 64,
        _ => 1,
    }
}

impl Bundle {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            color: None,
        }
    }

    pub fn current_weight(&self, max_stack: impl Fn(u16) -> u16) -> u32 {
        self.items
            .iter()
            .map(|(id, count, _)| (*count as u32) * item_weight(max_stack(*id)))
            .sum()
    }

    pub fn can_add(&self, item_id: u16, count: u16, max_stack: u16) -> bool {
        let getter = |_: u16| max_stack;
        let current = self.current_weight(getter);
        let additional = (count as u32) * item_weight(max_stack);
        current + additional <= CAPACITY
    }

    pub fn add_item(&mut self, id: u16, count: u16, data: u16, max_stack: u16) -> bool {
        if !self.can_add(id, count, max_stack) {
            return false;
        }
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|(sid, _, sd)| *sid == id && *sd == data)
        {
            existing.1 += count;
        } else {
            self.items.push((id, count, data));
        }
        true
    }

    pub fn take_first(&mut self) -> Option<(u16, u16, u16)> {
        self.items.pop()
    }
}

impl Default for Bundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cant_overfill() {
        let mut b = Bundle::new();
        assert!(b.add_item(1, 64, 0, 64));
        assert!(!b.add_item(2, 1, 0, 64));
    }

    #[test]
    fn fifo_take() {
        let mut b = Bundle::new();
        b.add_item(1, 1, 0, 64);
        b.add_item(2, 1, 0, 64);
        assert_eq!(b.take_first().unwrap().0, 2);
    }
}
