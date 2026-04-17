//! Rail — normal/powered/detector/activator.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailKind {
    Normal,
    Powered,   // Boosts minecart when powered
    Detector,  // Emits redstone when minecart on top
    Activator, // Toggles minecart on
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailShape {
    NorthSouth,
    EastWest,
    AscendingEast,
    AscendingWest,
    AscendingNorth,
    AscendingSouth,
    SouthEastCurve,
    SouthWestCurve,
    NorthWestCurve,
    NorthEastCurve,
}

#[derive(Debug, Clone)]
pub struct Rail {
    pub kind: RailKind,
    pub shape: RailShape,
    pub powered: bool,
}

impl Rail {
    pub fn new(kind: RailKind, shape: RailShape) -> Self {
        Self {
            kind,
            shape,
            powered: false,
        }
    }

    pub fn can_curve(&self) -> bool {
        self.kind == RailKind::Normal
    }

    /// Booster/powered rail speed multiplier.
    pub fn speed_multiplier(&self) -> f32 {
        if self.kind == RailKind::Powered && self.powered {
            1.8
        } else {
            1.0
        }
    }

    /// Detector emits redstone when cart on top.
    pub fn emits_redstone(&self, cart_on_top: bool) -> bool {
        self.kind == RailKind::Detector && cart_on_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powered_boosts() {
        let mut r = Rail::new(RailKind::Powered, RailShape::NorthSouth);
        r.powered = true;
        assert!(r.speed_multiplier() > 1.0);
    }

    #[test]
    fn only_normal_curves() {
        assert!(Rail::new(RailKind::Normal, RailShape::NorthSouth).can_curve());
        assert!(!Rail::new(RailKind::Powered, RailShape::NorthSouth).can_curve());
    }
}
