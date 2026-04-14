//! Trapdoor — wooden, iron, copper.

use crate::copper_oxidation::OxidationStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapdoorMaterial {
    Wood(u8), // wood type
    Iron,
    Copper(OxidationStage, bool), // (stage, waxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapdoorHalf {
    Bottom,
    Top,
}

#[derive(Debug, Clone)]
pub struct Trapdoor {
    pub material: TrapdoorMaterial,
    pub facing: u8,
    pub half: TrapdoorHalf,
    pub open: bool,
    pub powered: bool,
    pub waterlogged: bool,
}

impl Trapdoor {
    pub fn new(material: TrapdoorMaterial, facing: u8) -> Self {
        Self {
            material,
            facing,
            half: TrapdoorHalf::Bottom,
            open: false,
            powered: false,
            waterlogged: false,
        }
    }

    pub fn toggle_by_player(&mut self) -> bool {
        match self.material {
            TrapdoorMaterial::Iron => false,
            _ => {
                self.open = !self.open;
                true
            }
        }
    }

    pub fn set_powered(&mut self, powered: bool) {
        self.powered = powered;
        if powered {
            self.open = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_trapdoor_needs_redstone() {
        let mut t = Trapdoor::new(TrapdoorMaterial::Iron, 0);
        assert!(!t.toggle_by_player());
    }
}
