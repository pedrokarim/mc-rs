//! Village structure types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillageBiomeType {
    Plains,
    Desert,
    Savanna,
    Taiga,
    Snowy,
}

impl VillageBiomeType {
    pub fn from_biome_id(biome: u8) -> Option<Self> {
        match biome {
            1 => Some(Self::Plains),
            2 | 17 | 130 => Some(Self::Desert),
            35 | 36 => Some(Self::Savanna),
            5 | 32 | 133 => Some(Self::Taiga),
            12 | 13 | 30 | 31 | 158 => Some(Self::Snowy),
            _ => None,
        }
    }

    pub fn bell_count_per_village() -> u32 {
        1
    }
    pub fn max_radius_blocks() -> f64 {
        64.0
    }
}

/// Village structures (buildings).
pub fn village_buildings(biome: VillageBiomeType) -> &'static [&'static str] {
    match biome {
        VillageBiomeType::Plains => &[
            "village/plains/houses/plains_small_house_1",
            "village/plains/houses/plains_small_house_2",
            "village/plains/houses/plains_medium_house_1",
            "village/plains/houses/plains_big_house_1",
            "village/plains/houses/plains_meeting_point_1",
            "village/plains/houses/plains_animal_pen_1",
            "village/plains/houses/plains_butcher_shop_1",
            "village/plains/houses/plains_blacksmith_1",
            "village/plains/houses/plains_fisher_cottage_1",
            "village/plains/houses/plains_weaponsmith_1",
            "village/plains/houses/plains_library_1",
            "village/plains/houses/plains_cartographer_1",
            "village/plains/houses/plains_toolsmith_1",
            "village/plains/houses/plains_armorer_1",
            "village/plains/houses/plains_shepherd_1",
            "village/plains/houses/plains_tannery_1",
            "village/plains/houses/plains_stable_1",
            "village/plains/houses/plains_masons_house_1",
            "village/plains/houses/plains_temple_1",
        ],
        VillageBiomeType::Desert => &[
            "village/desert/houses/desert_small_house_1",
            "village/desert/houses/desert_medium_house_1",
            "village/desert/houses/desert_meeting_point_1",
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plains_has_houses() {
        assert!(!village_buildings(VillageBiomeType::Plains).is_empty());
    }
}
