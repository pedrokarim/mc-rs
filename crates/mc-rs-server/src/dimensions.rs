//! Dimensions — port de PMMP `src/world/DimensionIds.php` + concept world.
//!
//! Bedrock supporte 3 dimensions : Overworld, Nether, End. Chaque a une
//! hauteur max / min et des règles de génération différentes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DimensionId {
    Overworld = 0,
    Nether = 1,
    End = 2,
}

impl DimensionId {
    pub fn min_y(&self) -> i32 {
        match self {
            Self::Overworld => -64,
            Self::Nether => 0,
            Self::End => 0,
        }
    }

    pub fn max_y(&self) -> i32 {
        match self {
            Self::Overworld => 320,
            Self::Nether => 128,
            Self::End => 256,
        }
    }

    pub fn has_sky(&self) -> bool {
        matches!(self, Self::Overworld | Self::End)
    }

    pub fn has_weather(&self) -> bool {
        matches!(self, Self::Overworld)
    }

    pub fn ambient_light(&self) -> f32 {
        match self {
            Self::Overworld => 0.0,
            Self::Nether => 0.1,
            Self::End => 0.0,
        }
    }

    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:nether",
            Self::End => "minecraft:the_end",
        }
    }

    /// Numeric dimension id used on the wire and in LevelDB keys.
    pub fn as_id(&self) -> i32 {
        *self as i32
    }

    /// Map a numeric dimension id (0/1/2) back to the enum.
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Overworld),
            1 => Some(Self::Nether),
            2 => Some(Self::End),
            _ => None,
        }
    }

    /// Number of 16-block vertical sub-chunks spanning this dimension's height.
    /// Overworld = 24, Nether = 8, End = 16 (réf PMMP `ChunkSerializer::getDimensionChunkBounds`).
    pub fn sub_chunk_count(&self) -> u8 {
        ((self.max_y() - self.min_y()) / 16) as u8
    }
}

/// Portail Nether : ratio de scaling 1:8 entre Overworld et Nether.
pub fn nether_portal_coordinate(overworld_xz: f32) -> f32 {
    overworld_xz / 8.0
}

pub fn overworld_from_nether(nether_xz: f32) -> f32 {
    nether_xz * 8.0
}

/// Portail End : coordonnées fixes (entry platform 100, 50, 0).
pub const END_SPAWN: [f32; 3] = [100.0, 50.0, 0.0];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overworld_bounds() {
        assert_eq!(DimensionId::Overworld.min_y(), -64);
        assert_eq!(DimensionId::Overworld.max_y(), 320);
    }

    #[test]
    fn nether_bounds() {
        assert_eq!(DimensionId::Nether.min_y(), 0);
        assert_eq!(DimensionId::Nether.max_y(), 128);
    }

    #[test]
    fn only_overworld_has_weather() {
        assert!(DimensionId::Overworld.has_weather());
        assert!(!DimensionId::Nether.has_weather());
        assert!(!DimensionId::End.has_weather());
    }

    #[test]
    fn nether_portal_ratio() {
        assert_eq!(nether_portal_coordinate(800.0), 100.0);
        assert_eq!(overworld_from_nether(100.0), 800.0);
    }
}
