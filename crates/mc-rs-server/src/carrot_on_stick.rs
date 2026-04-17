//! Carrot on stick / Warped fungus on stick — control mounts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlStickKind {
    Carrot,       // Pig
    WarpedFungus, // Strider
}

#[derive(Debug, Clone)]
pub struct ControlStick {
    pub kind: ControlStickKind,
    pub durability: u16,
}

/// Max durability.
pub const CARROT_DURABILITY: u16 = 25;
pub const WARPED_FUNGUS_DURABILITY: u16 = 100;

/// Boost duration per use.
pub const BOOST_TICKS: u32 = 100;

impl ControlStick {
    pub fn new_carrot() -> Self {
        Self {
            kind: ControlStickKind::Carrot,
            durability: CARROT_DURABILITY,
        }
    }

    pub fn new_warped_fungus() -> Self {
        Self {
            kind: ControlStickKind::WarpedFungus,
            durability: WARPED_FUNGUS_DURABILITY,
        }
    }

    /// Use stick on mount → reduces durability.
    pub fn use_stick(&mut self) -> bool {
        if self.durability == 0 {
            return false;
        }
        self.durability -= 1;
        true
    }

    pub fn is_broken(&self) -> bool {
        self.durability == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_breaks_stick() {
        let mut c = ControlStick::new_carrot();
        c.durability = 1;
        assert!(c.use_stick());
        assert!(c.is_broken());
    }
}
