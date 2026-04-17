//! Torch — regular / soul / redstone.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorchKind {
    Normal,
    Soul,
    Redstone,
    ColoredRedstone, // Bedrock extension
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorchPlacement {
    Standing, // On ground
    Wall(u8), // facing direction
}

#[derive(Debug, Clone)]
pub struct Torch {
    pub kind: TorchKind,
    pub placement: TorchPlacement,
    pub lit: bool,
}

pub fn light_emission(kind: TorchKind, lit: bool) -> u8 {
    if !lit {
        return 0;
    }
    match kind {
        TorchKind::Normal => 14,
        TorchKind::Soul => 10,
        TorchKind::Redstone => 7,
        TorchKind::ColoredRedstone => 7,
    }
}

impl Torch {
    pub fn new(kind: TorchKind, placement: TorchPlacement) -> Self {
        Self {
            kind,
            placement,
            lit: true,
        }
    }

    /// Extinguish (water/wind charge).
    pub fn extinguish(&mut self) {
        self.lit = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soul_less_bright() {
        assert!(light_emission(TorchKind::Soul, true) < light_emission(TorchKind::Normal, true));
    }
}
