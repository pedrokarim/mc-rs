//! Door — 2-block height, open/close, hinge side, powered.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorMaterial {
    Oak,
    Birch,
    Spruce,
    Jungle,
    Acacia,
    DarkOak,
    Mangrove,
    Cherry,
    Bamboo,
    Iron,
    Nether,
    Copper(crate::copper_oxidation::OxidationStage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hinge {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Door {
    pub material: DoorMaterial,
    pub facing: u8, // 0=north...
    pub hinge: Hinge,
    pub open: bool,
    pub powered: bool,
}

impl Door {
    pub fn new(material: DoorMaterial, facing: u8) -> Self {
        Self {
            material,
            facing,
            hinge: Hinge::Left,
            open: false,
            powered: false,
        }
    }

    pub fn toggle(&mut self, by_player: bool) -> bool {
        if !self.can_be_opened_by_player() && by_player {
            return false;
        }
        self.open = !self.open;
        true
    }

    /// Iron doors only open with redstone.
    pub fn can_be_opened_by_player(&self) -> bool {
        !matches!(self.material, DoorMaterial::Iron)
    }

    pub fn set_powered(&mut self, powered: bool) {
        let was_open = self.open;
        self.powered = powered;
        self.open = powered;
        if was_open != powered {
            // Trigger sound event.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_needs_redstone() {
        let mut d = Door::new(DoorMaterial::Iron, 0);
        assert!(!d.toggle(true));
        d.set_powered(true);
        assert!(d.open);
    }

    #[test]
    fn oak_opens_by_player() {
        let mut d = Door::new(DoorMaterial::Oak, 0);
        assert!(d.toggle(true));
    }
}
