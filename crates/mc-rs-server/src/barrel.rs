//! Barrel — chest-like but can be placed anywhere.

#[derive(Debug, Clone)]
pub struct Barrel {
    pub inventory: Vec<Option<(u16, u16)>>, // 27 slots
    pub facing: u8,
    pub open: bool,
    pub viewers: u32,
}

pub const SLOTS: usize = 27;

impl Barrel {
    pub fn new(facing: u8) -> Self {
        Self {
            inventory: vec![None; SLOTS],
            facing,
            open: false,
            viewers: 0,
        }
    }

    pub fn add_viewer(&mut self) {
        self.viewers += 1;
        if self.viewers > 0 {
            self.open = true;
        }
    }

    pub fn remove_viewer(&mut self) {
        self.viewers = self.viewers.saturating_sub(1);
        if self.viewers == 0 {
            self.open = false;
        }
    }

    /// Barrels don't care about blocks above them (unlike chests).
    pub fn can_open_with_block_above() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_with_viewer() {
        let mut b = Barrel::new(0);
        b.add_viewer();
        assert!(b.open);
    }
}
