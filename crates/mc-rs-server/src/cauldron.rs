//! Cauldron — port PMMP `src/block/Cauldron.php`.
//! Peut contenir eau/lava/potion avec level 0-3.

use crate::brewing::PotionType;

#[derive(Debug, Clone)]
pub enum CauldronContent {
    Empty,
    Water { level: u8 }, // 0 = empty, 3 = full
    Lava { level: u8 },
    PowderedSnow { level: u8 },
    Potion { kind: PotionType, level: u8 },
}

impl CauldronContent {
    pub fn level(&self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Water { level }
            | Self::Lava { level }
            | Self::PowderedSnow { level }
            | Self::Potion { level, .. } => *level,
        }
    }

    pub fn is_full(&self) -> bool {
        self.level() >= 3
    }

    /// Can dye an item ? Only water + dye → dyed item.
    pub fn can_dye(&self) -> bool {
        matches!(self, Self::Water { level: 1.. })
    }

    /// Can wash away banner patterns ? Water cauldron.
    pub fn can_wash_banner(&self) -> bool {
        matches!(self, Self::Water { level: 1.. })
    }

    /// Can fill bottle with water/potion ?
    pub fn can_fill_bottle(&self) -> bool {
        matches!(
            self,
            Self::Water { level: 1.. } | Self::Potion { level: 1.., .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_level_0() {
        assert_eq!(CauldronContent::Empty.level(), 0);
    }

    #[test]
    fn water_level_3_full() {
        let c = CauldronContent::Water { level: 3 };
        assert!(c.is_full());
    }
}
