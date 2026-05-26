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

    /// Résout un identifiant ("village", "minecraft:village") vers un StructureKind.
    pub fn parse(name: &str) -> Option<Self> {
        let n = name.strip_prefix("minecraft:").unwrap_or(name);
        Some(match n {
            "village" => Self::Village,
            "stronghold" => Self::Stronghold,
            "mineshaft" => Self::MineshaftNormal,
            "mineshaft_mesa" => Self::MineshaftMesa,
            "monument" | "ocean_monument" => Self::OceanMonument,
            "fortress" | "nether_fortress" => Self::NetherFortress,
            "bastion_remnant" => Self::BastionRemnant,
            "endcity" | "end_city" => Self::EndCity,
            "mansion" | "woodland_mansion" => Self::WoodlandMansion,
            "ocean_ruin_cold" => Self::OceanRuinCold,
            "ocean_ruin_warm" => Self::OceanRuinWarm,
            "shipwreck" | "shipwreck_buried" => Self::ShipwreckBuried,
            "shipwreck_beached" => Self::ShipwreckBeached,
            "desert_pyramid" => Self::DesertPyramid,
            "jungle_pyramid" | "jungle_temple" => Self::JungleTemple,
            "swamp_hut" | "witch_hut" => Self::SwampHut,
            "igloo" => Self::Igloo,
            "ruined_portal" => Self::RuinedPortalOverworld,
            "ruined_portal_nether" => Self::RuinedPortalNether,
            "pillager_outpost" => Self::PillagerOutpost,
            "buried_treasure" => Self::BuriedTreasure,
            "ancient_city" => Self::AncientCity,
            "trail_ruins" | "trailruin" => Self::Trailruin,
            _ => return None,
        })
    }
}

/// Localise la structure la plus proche du point donné via une approximation
/// grid-based (séparation moyenne en chunks). PMMP ne fait pas de /locate ;
/// vanilla Bedrock fait une recherche réelle dans les chunks générés. Ici on
/// donne juste la position de la cellule la plus proche dans la grille de
/// séparation — utilisable comme indicateur, pas comme garantie absolue.
pub fn locate_nearest(kind: StructureKind, from: [f32; 3]) -> [i32; 3] {
    let sep = kind.average_separation_chunks() as i32;
    let cx = (from[0] / 16.0).floor() as i32;
    let cz = (from[2] / 16.0).floor() as i32;
    // Snap au multiple de sep le plus proche.
    let snap_x = ((cx as f32 / sep as f32).round() as i32) * sep;
    let snap_z = ((cz as f32 / sep as f32).round() as i32) * sep;
    [snap_x * 16 + 8, 64, snap_z * 16 + 8]
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
