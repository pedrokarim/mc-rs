//! Entity identifiers — mapping ID runtime ↔ network name.
//! Port PMMP `src/network/mcpe/protocol/types/entity/EntityIds.php`.

use crate::mob_ai::MobKind;
use crate::passive_entities::*;
use crate::projectiles::ProjectileKind;
use crate::vehicles::VehicleKind;

/// Tous les identifiants réseau d'entités que le serveur peut spawn.
pub fn all_entity_network_ids() -> Vec<&'static str> {
    let mut out = vec![
        "minecraft:player",
        "minecraft:item",
        "minecraft:xp_orb",
        "minecraft:tnt",
        "minecraft:falling_block",
        "minecraft:painting",
        "minecraft:lightning_bolt",
        "minecraft:end_crystal",
        "minecraft:eye_of_ender_signal",
        "minecraft:area_effect_cloud",
        "minecraft:armor_stand",
        "minecraft:ender_chest_minecart", // variants
    ];
    // Mobs.
    for m in all_mob_kinds() {
        out.push(m.network_identifier());
    }
    // Projectiles.
    for p in all_projectile_kinds() {
        out.push(p.network_identifier());
    }
    out
}

pub fn all_mob_kinds() -> Vec<MobKind> {
    vec![
        MobKind::Chicken,
        MobKind::Cow,
        MobKind::Pig,
        MobKind::Sheep,
        MobKind::Rabbit,
        MobKind::Squid,
        MobKind::Villager,
        MobKind::Horse,
        MobKind::Donkey,
        MobKind::Llama,
        MobKind::Cat,
        MobKind::Wolf,
        MobKind::Ocelot,
        MobKind::Parrot,
        MobKind::Fox,
        MobKind::Panda,
        MobKind::Turtle,
        MobKind::Dolphin,
        MobKind::Zombie,
        MobKind::Skeleton,
        MobKind::Creeper,
        MobKind::Spider,
        MobKind::CaveSpider,
        MobKind::Enderman,
        MobKind::Witch,
        MobKind::Blaze,
        MobKind::Ghast,
        MobKind::MagmaCube,
        MobKind::Slime,
        MobKind::Drowned,
        MobKind::Husk,
        MobKind::Stray,
        MobKind::ZombieVillager,
        MobKind::WitherSkeleton,
        MobKind::ZombiePigman,
        MobKind::Wither,
        MobKind::EnderDragon,
        MobKind::Ravager,
        MobKind::Vindicator,
        MobKind::Pillager,
        MobKind::IronGolem,
        MobKind::SnowGolem,
    ]
}

pub fn all_projectile_kinds() -> Vec<ProjectileKind> {
    vec![
        ProjectileKind::Arrow,
        ProjectileKind::Egg,
        ProjectileKind::Snowball,
        ProjectileKind::EnderPearl,
        ProjectileKind::ExperienceBottle,
        ProjectileKind::FishingHook,
        ProjectileKind::SplashPotion,
        ProjectileKind::LingeringPotion,
        ProjectileKind::Trident,
        ProjectileKind::FireCharge,
        ProjectileKind::WitherSkull,
        ProjectileKind::FireworkRocket,
        ProjectileKind::ShulkerBullet,
    ]
}

/// Compte total d'entités vanilla exposables.
pub fn total_entity_count() -> usize {
    all_entity_network_ids().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_50_entity_kinds_known() {
        assert!(total_entity_count() > 50);
    }

    #[test]
    fn mob_list_not_empty() {
        assert!(!all_mob_kinds().is_empty());
    }
}
