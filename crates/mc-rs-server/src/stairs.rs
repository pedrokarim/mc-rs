//! Stairs — shape orientation and connection.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairShape {
    Straight,
    InnerLeft,
    InnerRight,
    OuterLeft,
    OuterRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairHalf {
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct Stairs {
    pub material: u16,
    pub facing: u8,
    pub half: StairHalf,
    pub shape: StairShape,
    pub waterlogged: bool,
}

impl Stairs {
    pub fn new(material: u16, facing: u8) -> Self {
        Self {
            material,
            facing,
            half: StairHalf::Bottom,
            shape: StairShape::Straight,
            waterlogged: false,
        }
    }

    /// Upside-down stairs placed when clicking top half of block.
    pub fn flip_upside_down(&mut self) {
        self.half = match self.half {
            StairHalf::Bottom => StairHalf::Top,
            StairHalf::Top => StairHalf::Bottom,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_upside() {
        let mut s = Stairs::new(1, 0);
        s.flip_upside_down();
        assert_eq!(s.half, StairHalf::Top);
    }
}
