//! Item frame — display item on wall, rotate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVariant {
    Normal,    // Oak item frame
    Glow,      // Glow item frame (bright)
}

#[derive(Debug, Clone)]
pub struct ItemFrame {
    pub variant: FrameVariant,
    pub facing: u8, // 0=down,1=up,2=north,3=south,4=west,5=east
    pub item: Option<(u16, u16)>, // (id, damage)
    pub rotation: u8, // 0-7 (0,45,90,... deg)
    pub is_map: bool,
}

impl ItemFrame {
    pub fn new(variant: FrameVariant, facing: u8) -> Self {
        Self {
            variant,
            facing,
            item: None,
            rotation: 0,
            is_map: false,
        }
    }

    pub fn place_item(&mut self, id: u16, damage: u16) -> Option<(u16, u16)> {
        let prev = self.item.take();
        self.item = Some((id, damage));
        self.is_map = id == Self::MAP_ID;
        prev
    }

    pub fn rotate(&mut self) {
        self.rotation = (self.rotation + 1) % 8;
    }

    pub fn clear(&mut self) -> Option<(u16, u16)> {
        self.is_map = false;
        self.item.take()
    }

    /// Redstone output based on rotation (0-15).
    /// Comparator reads item presence + rotation.
    pub fn redstone_output(&self) -> u8 {
        if self.item.is_some() {
            self.rotation * 2 + 1
        } else {
            0
        }
    }

    const MAP_ID: u16 = 358;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_returns_previous() {
        let mut f = ItemFrame::new(FrameVariant::Normal, 2);
        assert!(f.place_item(1, 0).is_none());
        assert_eq!(f.place_item(2, 0), Some((1, 0)));
    }

    #[test]
    fn rotate_cycles_8() {
        let mut f = ItemFrame::new(FrameVariant::Normal, 2);
        for _ in 0..8 {
            f.rotate();
        }
        assert_eq!(f.rotation, 0);
    }

    #[test]
    fn empty_no_redstone() {
        let f = ItemFrame::new(FrameVariant::Normal, 2);
        assert_eq!(f.redstone_output(), 0);
    }
}
