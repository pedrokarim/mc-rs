//! Structures generation — port conceptuel (PMMP n'implémente pas complètement).
//! Définit les structures vanilla (village, stronghold, mineshaft, etc.) avec
//! leur densité d'apparition. Placement effectif dans worldgen.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructureKind {
    Village,
    Stronghold,
    MineshaftNormal,
    MineshaftMesa,
    OceanMonument,
    NetherFortress,
    BastionRemnant,
    EndCity,
    WoodlandMansion,
    OceanRuinCold,
    OceanRuinWarm,
    ShipwreckBuried,
    ShipwreckBeached,
    DesertPyramid,
    JungleTemple,
    SwampHut,
    IglooNormal,
    IglooIce,
    Igloo,
    RuinedPortalOverworld,
    RuinedPortalNether,
    PillagerOutpost,
    BuriedTreasure,
    AncientCity,
    Trailruin,
}

impl StructureKind {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Village => "minecraft:village",
            Self::Stronghold => "minecraft:stronghold",
            Self::MineshaftNormal => "minecraft:mineshaft",
            Self::MineshaftMesa => "minecraft:mineshaft_mesa",
            Self::OceanMonument => "minecraft:monument",
            Self::NetherFortress => "minecraft:fortress",
            Self::BastionRemnant => "minecraft:bastion_remnant",
            Self::EndCity => "minecraft:endcity",
            Self::WoodlandMansion => "minecraft:mansion",
            Self::OceanRuinCold => "minecraft:ocean_ruin_cold",
            Self::OceanRuinWarm => "minecraft:ocean_ruin_warm",
            Self::ShipwreckBuried => "minecraft:shipwreck_buried",
            Self::ShipwreckBeached => "minecraft:shipwreck_beached",
            Self::DesertPyramid => "minecraft:desert_pyramid",
            Self::JungleTemple => "minecraft:jungle_pyramid",
            Self::SwampHut => "minecraft:swamp_hut",
            Self::IglooNormal | Self::Igloo => "minecraft:igloo",
            Self::IglooIce => "minecraft:igloo_ice",
            Self::RuinedPortalOverworld => "minecraft:ruined_portal",
            Self::RuinedPortalNether => "minecraft:ruined_portal_nether",
            Self::PillagerOutpost => "minecraft:pillager_outpost",
            Self::BuriedTreasure => "minecraft:buried_treasure",
            Self::AncientCity => "minecraft:ancient_city",
            Self::Trailruin => "minecraft:trail_ruins",
        }
    }

    /// Rayon moyen (chunks) entre structures de ce type. Vanilla values.
    pub fn average_separation_chunks(&self) -> u32 {
        match self {
            Self::Village => 32,
            Self::Stronghold => 128, // ~3 strongholds close-ish
            Self::MineshaftNormal | Self::MineshaftMesa => 1, // mineshafts très fréquents
            Self::OceanMonument => 32,
            Self::NetherFortress => 27,
            Self::BastionRemnant => 27,
            Self::EndCity => 20,
            Self::WoodlandMansion => 80,
            Self::OceanRuinCold | Self::OceanRuinWarm => 20,
            Self::ShipwreckBeached | Self::ShipwreckBuried => 24,
            Self::DesertPyramid
            | Self::JungleTemple
            | Self::SwampHut
            | Self::IglooNormal
            | Self::Igloo
            | Self::IglooIce => 32,
            Self::RuinedPortalOverworld => 40,
            Self::RuinedPortalNether => 25,
            Self::PillagerOutpost => 32,
            Self::BuriedTreasure => 1, // toujours dans les beach chunks
            Self::AncientCity => 24,
            Self::Trailruin => 34,
        }
    }

    /// Dimension dans laquelle la structure génère.
    pub fn dimension(&self) -> &'static str {
        match self {
            Self::NetherFortress | Self::BastionRemnant | Self::RuinedPortalNether => "nether",
            Self::EndCity => "the_end",
            _ => "overworld",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn village_in_overworld() {
        assert_eq!(StructureKind::Village.dimension(), "overworld");
    }

    #[test]
    fn fortress_in_nether() {
        assert_eq!(StructureKind::NetherFortress.dimension(), "nether");
    }

    #[test]
    fn end_city_in_end() {
        assert_eq!(StructureKind::EndCity.dimension(), "the_end");
    }
}
