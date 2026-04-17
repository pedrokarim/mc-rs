//! Spawn rules — port conceptuel PMMP (pas implémenté) + vanilla.
//! Définit quand/où les mobs peuvent spawn naturellement.

use crate::biomes_registry::BiomeKind;
use crate::mob_ai::MobKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnLightRequirement {
    NightOnly,
    DayOnly,
    Any,
    DarkOnly, // light_level < 7
}

#[derive(Debug, Clone)]
pub struct SpawnRule {
    pub mob: MobKind,
    pub biomes: Vec<BiomeKind>,
    pub light: SpawnLightRequirement,
    pub min_y: i32,
    pub max_y: i32,
    pub weight: u32,
}

/// Règles de spawn vanilla simplifiées.
pub fn vanilla_spawn_rules() -> Vec<SpawnRule> {
    vec![
        SpawnRule {
            mob: MobKind::Cow,
            biomes: vec![BiomeKind::Plains, BiomeKind::Forest, BiomeKind::Savanna],
            light: SpawnLightRequirement::DayOnly,
            min_y: 60,
            max_y: 256,
            weight: 8,
        },
        SpawnRule {
            mob: MobKind::Sheep,
            biomes: vec![BiomeKind::Plains, BiomeKind::Meadow],
            light: SpawnLightRequirement::DayOnly,
            min_y: 60,
            max_y: 256,
            weight: 12,
        },
        SpawnRule {
            mob: MobKind::Pig,
            biomes: vec![BiomeKind::Plains, BiomeKind::Forest],
            light: SpawnLightRequirement::DayOnly,
            min_y: 60,
            max_y: 256,
            weight: 10,
        },
        SpawnRule {
            mob: MobKind::Chicken,
            biomes: vec![BiomeKind::Plains, BiomeKind::Forest],
            light: SpawnLightRequirement::DayOnly,
            min_y: 60,
            max_y: 256,
            weight: 10,
        },
        SpawnRule {
            mob: MobKind::Zombie,
            biomes: vec![], // tous overworld hostile
            light: SpawnLightRequirement::DarkOnly,
            min_y: -64,
            max_y: 256,
            weight: 100,
        },
        SpawnRule {
            mob: MobKind::Skeleton,
            biomes: vec![],
            light: SpawnLightRequirement::DarkOnly,
            min_y: -64,
            max_y: 256,
            weight: 100,
        },
        SpawnRule {
            mob: MobKind::Creeper,
            biomes: vec![],
            light: SpawnLightRequirement::DarkOnly,
            min_y: -64,
            max_y: 256,
            weight: 100,
        },
        SpawnRule {
            mob: MobKind::Spider,
            biomes: vec![],
            light: SpawnLightRequirement::DarkOnly,
            min_y: -64,
            max_y: 256,
            weight: 100,
        },
        SpawnRule {
            mob: MobKind::Enderman,
            biomes: vec![],
            light: SpawnLightRequirement::DarkOnly,
            min_y: -64,
            max_y: 256,
            weight: 10,
        },
        SpawnRule {
            mob: MobKind::Squid,
            biomes: vec![
                BiomeKind::Ocean,
                BiomeKind::DeepOcean,
                BiomeKind::ColdOcean,
                BiomeKind::DeepColdOcean,
                BiomeKind::WarmOcean,
                BiomeKind::LukewarmOcean,
                BiomeKind::DeepLukewarmOcean,
                BiomeKind::River,
            ],
            light: SpawnLightRequirement::Any,
            min_y: 46,
            max_y: 63,
            weight: 5,
        },
        SpawnRule {
            mob: MobKind::Blaze,
            biomes: vec![BiomeKind::NetherWastes],
            light: SpawnLightRequirement::Any,
            min_y: 0,
            max_y: 128,
            weight: 10,
        },
        SpawnRule {
            mob: MobKind::Ghast,
            biomes: vec![
                BiomeKind::NetherWastes,
                BiomeKind::BasaltDeltas,
                BiomeKind::SoulSandValley,
            ],
            light: SpawnLightRequirement::Any,
            min_y: 0,
            max_y: 128,
            weight: 50,
        },
        SpawnRule {
            mob: MobKind::Enderman,
            biomes: vec![BiomeKind::TheEnd],
            light: SpawnLightRequirement::Any,
            min_y: 0,
            max_y: 255,
            weight: 80,
        },
    ]
}

/// Check if a mob can spawn at the given conditions.
pub fn can_spawn_at(
    rule: &SpawnRule,
    biome: BiomeKind,
    light_level: u8,
    is_night: bool,
    y: i32,
) -> bool {
    if y < rule.min_y || y > rule.max_y {
        return false;
    }
    if !rule.biomes.is_empty() && !rule.biomes.contains(&biome) {
        return false;
    }
    match rule.light {
        SpawnLightRequirement::DayOnly => !is_night && light_level >= 9,
        SpawnLightRequirement::NightOnly => is_night,
        SpawnLightRequirement::DarkOnly => light_level < 7,
        SpawnLightRequirement::Any => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_spawns_in_dark() {
        let rules = vanilla_spawn_rules();
        let zombie_rule = rules.iter().find(|r| r.mob == MobKind::Zombie).unwrap();
        assert!(can_spawn_at(zombie_rule, BiomeKind::Plains, 5, true, 64));
        assert!(!can_spawn_at(zombie_rule, BiomeKind::Plains, 15, false, 64));
    }

    #[test]
    fn cow_spawns_in_daylight_grass_biomes() {
        let rules = vanilla_spawn_rules();
        let cow = rules.iter().find(|r| r.mob == MobKind::Cow).unwrap();
        assert!(can_spawn_at(cow, BiomeKind::Plains, 15, false, 64));
        assert!(!can_spawn_at(cow, BiomeKind::Desert, 15, false, 64));
    }
}
