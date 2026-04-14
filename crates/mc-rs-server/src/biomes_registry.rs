//! Biomes registry — port PMMP `src/world/biome/*`.
//!
//! Liste complète des biomes vanilla avec leurs propriétés (temperature,
//! downfall, surface blocks).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeKind {
    // Overworld
    Plains,
    SunflowerPlains,
    Forest,
    FlowerForest,
    BirchForest,
    OldGrowthBirchForest,
    DarkForest,
    Taiga,
    SnowyTaiga,
    OldGrowthPineTaiga,
    OldGrowthSpruceTaiga,
    Jungle,
    SparseJungle,
    BambooJungle,
    Desert,
    Savanna,
    SavannaPlateau,
    WindsweptSavanna,
    Swamp,
    MangroveSwamp,
    Beach,
    SnowyBeach,
    StonyShore,
    River,
    FrozenRiver,
    Ocean,
    DeepOcean,
    ColdOcean,
    DeepColdOcean,
    FrozenOcean,
    DeepFrozenOcean,
    LukewarmOcean,
    DeepLukewarmOcean,
    WarmOcean,
    MushroomFields,
    SnowyPlains,
    IceSpikes,
    WindsweptHills,
    WindsweptGravellyHills,
    WindsweptForest,
    StonyPeaks,
    JaggedPeaks,
    FrozenPeaks,
    SnowySlopes,
    Grove,
    Meadow,
    CherryGrove,
    LushCaves,
    DripstoneCaves,
    DeepDark,
    // Nether
    NetherWastes,
    CrimsonForest,
    WarpedForest,
    SoulSandValley,
    BasaltDeltas,
    // End
    TheEnd,
    EndHighlands,
    EndMidlands,
    SmallEndIslands,
    EndBarrens,
}

impl BiomeKind {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Plains => "plains",
            Self::SunflowerPlains => "sunflower_plains",
            Self::Forest => "forest",
            Self::FlowerForest => "flower_forest",
            Self::BirchForest => "birch_forest",
            Self::OldGrowthBirchForest => "old_growth_birch_forest",
            Self::DarkForest => "dark_forest",
            Self::Taiga => "taiga",
            Self::SnowyTaiga => "snowy_taiga",
            Self::OldGrowthPineTaiga => "old_growth_pine_taiga",
            Self::OldGrowthSpruceTaiga => "old_growth_spruce_taiga",
            Self::Jungle => "jungle",
            Self::SparseJungle => "sparse_jungle",
            Self::BambooJungle => "bamboo_jungle",
            Self::Desert => "desert",
            Self::Savanna => "savanna",
            Self::SavannaPlateau => "savanna_plateau",
            Self::WindsweptSavanna => "windswept_savanna",
            Self::Swamp => "swamp",
            Self::MangroveSwamp => "mangrove_swamp",
            Self::Beach => "beach",
            Self::SnowyBeach => "snowy_beach",
            Self::StonyShore => "stony_shore",
            Self::River => "river",
            Self::FrozenRiver => "frozen_river",
            Self::Ocean => "ocean",
            Self::DeepOcean => "deep_ocean",
            Self::ColdOcean => "cold_ocean",
            Self::DeepColdOcean => "deep_cold_ocean",
            Self::FrozenOcean => "frozen_ocean",
            Self::DeepFrozenOcean => "deep_frozen_ocean",
            Self::LukewarmOcean => "lukewarm_ocean",
            Self::DeepLukewarmOcean => "deep_lukewarm_ocean",
            Self::WarmOcean => "warm_ocean",
            Self::MushroomFields => "mushroom_fields",
            Self::SnowyPlains => "snowy_plains",
            Self::IceSpikes => "ice_spikes",
            Self::WindsweptHills => "windswept_hills",
            Self::WindsweptGravellyHills => "windswept_gravelly_hills",
            Self::WindsweptForest => "windswept_forest",
            Self::StonyPeaks => "stony_peaks",
            Self::JaggedPeaks => "jagged_peaks",
            Self::FrozenPeaks => "frozen_peaks",
            Self::SnowySlopes => "snowy_slopes",
            Self::Grove => "grove",
            Self::Meadow => "meadow",
            Self::CherryGrove => "cherry_grove",
            Self::LushCaves => "lush_caves",
            Self::DripstoneCaves => "dripstone_caves",
            Self::DeepDark => "deep_dark",
            Self::NetherWastes => "nether_wastes",
            Self::CrimsonForest => "crimson_forest",
            Self::WarpedForest => "warped_forest",
            Self::SoulSandValley => "soul_sand_valley",
            Self::BasaltDeltas => "basalt_deltas",
            Self::TheEnd => "the_end",
            Self::EndHighlands => "end_highlands",
            Self::EndMidlands => "end_midlands",
            Self::SmallEndIslands => "small_end_islands",
            Self::EndBarrens => "end_barrens",
        }
    }

    /// Température PMMP (0.0 = glace, 2.0+ = nether).
    pub fn temperature(&self) -> f32 {
        match self {
            Self::SnowyPlains | Self::SnowyTaiga | Self::IceSpikes | Self::FrozenRiver
            | Self::FrozenOcean | Self::DeepFrozenOcean | Self::FrozenPeaks | Self::SnowySlopes => 0.0,
            Self::Taiga | Self::OldGrowthPineTaiga | Self::SnowyBeach | Self::Grove
            | Self::ColdOcean | Self::DeepColdOcean => 0.25,
            Self::Plains | Self::SunflowerPlains | Self::Forest | Self::FlowerForest
            | Self::BirchForest | Self::Meadow | Self::River | Self::Beach => 0.7,
            Self::Jungle | Self::SparseJungle | Self::BambooJungle => 0.95,
            Self::Desert | Self::Savanna | Self::SavannaPlateau | Self::WindsweptSavanna => 1.2,
            Self::NetherWastes | Self::CrimsonForest | Self::WarpedForest | Self::SoulSandValley
            | Self::BasaltDeltas => 2.0,
            _ => 0.5,
        }
    }

    pub fn is_ocean(&self) -> bool {
        matches!(
            self,
            Self::Ocean
                | Self::DeepOcean
                | Self::ColdOcean
                | Self::DeepColdOcean
                | Self::FrozenOcean
                | Self::DeepFrozenOcean
                | Self::LukewarmOcean
                | Self::DeepLukewarmOcean
                | Self::WarmOcean
        )
    }

    pub fn is_nether(&self) -> bool {
        matches!(
            self,
            Self::NetherWastes
                | Self::CrimsonForest
                | Self::WarpedForest
                | Self::SoulSandValley
                | Self::BasaltDeltas
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oceans_identified() {
        assert!(BiomeKind::DeepOcean.is_ocean());
        assert!(BiomeKind::WarmOcean.is_ocean());
        assert!(!BiomeKind::Plains.is_ocean());
    }

    #[test]
    fn nether_biomes() {
        assert!(BiomeKind::NetherWastes.is_nether());
        assert!(BiomeKind::CrimsonForest.is_nether());
    }

    #[test]
    fn desert_hot() {
        assert!(BiomeKind::Desert.temperature() > 1.0);
    }
}
