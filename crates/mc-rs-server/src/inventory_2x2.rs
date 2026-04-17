//! Player 2x2 crafting grid (survival inventory).

#[derive(Debug, Clone)]
pub struct PlayerCraftingGrid {
    pub slots: [Option<(u16, u16)>; 4],
    pub output: Option<(u16, u16)>,
}

impl PlayerCraftingGrid {
    pub fn new() -> Self {
        Self {
            slots: [None; 4],
            output: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Return all items to inventory when closing (player drops).
    pub fn take_all(&mut self) -> Vec<(u16, u16)> {
        let mut out = Vec::new();
        for slot in self.slots.iter_mut() {
            if let Some(item) = slot.take() {
                out.push(item);
            }
        }
        out
    }
}

impl Default for PlayerCraftingGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_all_empties_grid() {
        let mut g = PlayerCraftingGrid::new();
        g.slots[0] = Some((1, 2));
        let items = g.take_all();
        assert_eq!(items.len(), 1);
        assert!(g.is_empty());
    }
}
