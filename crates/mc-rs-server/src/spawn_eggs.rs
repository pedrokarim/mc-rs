//! Spawn eggs — mapping item → mob type.

use crate::mob_ai::MobKind;

/// Lookup spawn egg → mob à faire spawn.
pub fn mob_for_egg(item_network_id: i32) -> Option<MobKind> {
    use crate::item_registry::network_id;
    let table: &[(&str, MobKind)] = &[
        ("minecraft:chicken_spawn_egg", MobKind::Chicken),
        ("minecraft:cow_spawn_egg", MobKind::Cow),
        ("minecraft:pig_spawn_egg", MobKind::Pig),
        ("minecraft:sheep_spawn_egg", MobKind::Sheep),
        ("minecraft:rabbit_spawn_egg", MobKind::Rabbit),
        ("minecraft:squid_spawn_egg", MobKind::Squid),
        ("minecraft:villager_spawn_egg", MobKind::Villager),
        ("minecraft:horse_spawn_egg", MobKind::Horse),
        ("minecraft:donkey_spawn_egg", MobKind::Donkey),
        ("minecraft:llama_spawn_egg", MobKind::Llama),
        ("minecraft:cat_spawn_egg", MobKind::Cat),
        ("minecraft:wolf_spawn_egg", MobKind::Wolf),
        ("minecraft:ocelot_spawn_egg", MobKind::Ocelot),
        ("minecraft:parrot_spawn_egg", MobKind::Parrot),
        ("minecraft:fox_spawn_egg", MobKind::Fox),
        ("minecraft:panda_spawn_egg", MobKind::Panda),
        ("minecraft:turtle_spawn_egg", MobKind::Turtle),
        ("minecraft:dolphin_spawn_egg", MobKind::Dolphin),
        ("minecraft:zombie_spawn_egg", MobKind::Zombie),
        ("minecraft:skeleton_spawn_egg", MobKind::Skeleton),
        ("minecraft:creeper_spawn_egg", MobKind::Creeper),
        ("minecraft:spider_spawn_egg", MobKind::Spider),
        ("minecraft:cave_spider_spawn_egg", MobKind::CaveSpider),
        ("minecraft:enderman_spawn_egg", MobKind::Enderman),
        ("minecraft:witch_spawn_egg", MobKind::Witch),
        ("minecraft:blaze_spawn_egg", MobKind::Blaze),
        ("minecraft:ghast_spawn_egg", MobKind::Ghast),
        ("minecraft:magma_cube_spawn_egg", MobKind::MagmaCube),
        ("minecraft:slime_spawn_egg", MobKind::Slime),
        ("minecraft:drowned_spawn_egg", MobKind::Drowned),
        ("minecraft:husk_spawn_egg", MobKind::Husk),
        ("minecraft:stray_spawn_egg", MobKind::Stray),
        (
            "minecraft:wither_skeleton_spawn_egg",
            MobKind::WitherSkeleton,
        ),
        ("minecraft:zombie_pigman_spawn_egg", MobKind::ZombiePigman),
        ("minecraft:ravager_spawn_egg", MobKind::Ravager),
        ("minecraft:vindicator_spawn_egg", MobKind::Vindicator),
        ("minecraft:pillager_spawn_egg", MobKind::Pillager),
    ];
    for (name, kind) in table {
        if network_id(name) == Some(item_network_id) {
            return Some(*kind);
        }
    }
    None
}
