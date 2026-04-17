//! Painting — port PMMP `src/entity/object/Painting.php` + `PaintingMotive.php`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintingMotive {
    // 1x1
    Kebab,
    Aztec,
    Alban,
    Aztec2,
    Bomb,
    Plant,
    Wasteland,
    // 2x1
    Pool,
    Courbet,
    Sea,
    Sunset,
    Creebet,
    // 1x2
    Wanderer,
    Graham,
    // 2x2
    Match,
    Bust,
    Stage,
    Void,
    SkullAndRoses,
    Wither,
    // 4x2
    Fighters,
    // 4x4
    Pointer,
    Pigscene,
    BurningSkull,
    Skeleton,
    DonkeyKong,
    // 4x3
    Earth,
    Wind,
    Water,
    Fire,
}

impl PaintingMotive {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Kebab => "Kebab",
            Self::Aztec => "Aztec",
            Self::Alban => "Alban",
            Self::Aztec2 => "Aztec2",
            Self::Bomb => "Bomb",
            Self::Plant => "Plant",
            Self::Wasteland => "Wasteland",
            Self::Pool => "Pool",
            Self::Courbet => "Courbet",
            Self::Sea => "Sea",
            Self::Sunset => "Sunset",
            Self::Creebet => "Creebet",
            Self::Wanderer => "Wanderer",
            Self::Graham => "Graham",
            Self::Match => "Match",
            Self::Bust => "Bust",
            Self::Stage => "Stage",
            Self::Void => "Void",
            Self::SkullAndRoses => "SkullAndRoses",
            Self::Wither => "Wither",
            Self::Fighters => "Fighters",
            Self::Pointer => "Pointer",
            Self::Pigscene => "Pigscene",
            Self::BurningSkull => "BurningSkull",
            Self::Skeleton => "Skeleton",
            Self::DonkeyKong => "DonkeyKong",
            Self::Earth => "Earth",
            Self::Wind => "Wind",
            Self::Water => "Water",
            Self::Fire => "Fire",
        }
    }

    /// Taille en blocs (width, height).
    pub fn size(&self) -> (u8, u8) {
        match self {
            Self::Kebab
            | Self::Aztec
            | Self::Alban
            | Self::Aztec2
            | Self::Bomb
            | Self::Plant
            | Self::Wasteland => (1, 1),
            Self::Pool | Self::Courbet | Self::Sea | Self::Sunset | Self::Creebet => (2, 1),
            Self::Wanderer | Self::Graham => (1, 2),
            Self::Match
            | Self::Bust
            | Self::Stage
            | Self::Void
            | Self::SkullAndRoses
            | Self::Wither => (2, 2),
            Self::Fighters => (4, 2),
            Self::Pointer | Self::Pigscene | Self::BurningSkull => (4, 4),
            Self::Skeleton | Self::DonkeyKong => (4, 3),
            Self::Earth | Self::Wind | Self::Water | Self::Fire => (4, 3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_is_1x1() {
        assert_eq!(PaintingMotive::Kebab.size(), (1, 1));
    }

    #[test]
    fn pointer_is_4x4() {
        assert_eq!(PaintingMotive::Pointer.size(), (4, 4));
    }
}
