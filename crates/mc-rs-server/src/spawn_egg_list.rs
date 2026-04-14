//! Spawn egg list — all mob types with eggs.

pub fn all_spawn_eggs() -> &'static [(&'static str, &'static str)] {
    &[
        ("minecraft:allay_spawn_egg", "minecraft:allay"),
        ("minecraft:axolotl_spawn_egg", "minecraft:axolotl"),
        ("minecraft:bat_spawn_egg", "minecraft:bat"),
        ("minecraft:bee_spawn_egg", "minecraft:bee"),
        ("minecraft:blaze_spawn_egg", "minecraft:blaze"),
        ("minecraft:camel_spawn_egg", "minecraft:camel"),
        ("minecraft:cat_spawn_egg", "minecraft:cat"),
        ("minecraft:cave_spider_spawn_egg", "minecraft:cave_spider"),
        ("minecraft:chicken_spawn_egg", "minecraft:chicken"),
        ("minecraft:cod_spawn_egg", "minecraft:cod"),
        ("minecraft:cow_spawn_egg", "minecraft:cow"),
        ("minecraft:creeper_spawn_egg", "minecraft:creeper"),
        ("minecraft:dolphin_spawn_egg", "minecraft:dolphin"),
        ("minecraft:donkey_spawn_egg", "minecraft:donkey"),
        ("minecraft:drowned_spawn_egg", "minecraft:drowned"),
        ("minecraft:elder_guardian_spawn_egg", "minecraft:elder_guardian"),
        ("minecraft:ender_dragon_spawn_egg", "minecraft:ender_dragon"),
        ("minecraft:enderman_spawn_egg", "minecraft:enderman"),
        ("minecraft:endermite_spawn_egg", "minecraft:endermite"),
        ("minecraft:evoker_spawn_egg", "minecraft:evoker"),
        ("minecraft:fox_spawn_egg", "minecraft:fox"),
        ("minecraft:frog_spawn_egg", "minecraft:frog"),
        ("minecraft:ghast_spawn_egg", "minecraft:ghast"),
        ("minecraft:glow_squid_spawn_egg", "minecraft:glow_squid"),
        ("minecraft:goat_spawn_egg", "minecraft:goat"),
        ("minecraft:guardian_spawn_egg", "minecraft:guardian"),
        ("minecraft:happy_ghast_spawn_egg", "minecraft:happy_ghast"),
        ("minecraft:hoglin_spawn_egg", "minecraft:hoglin"),
        ("minecraft:horse_spawn_egg", "minecraft:horse"),
        ("minecraft:husk_spawn_egg", "minecraft:husk"),
        ("minecraft:iron_golem_spawn_egg", "minecraft:iron_golem"),
        ("minecraft:llama_spawn_egg", "minecraft:llama"),
        ("minecraft:magma_cube_spawn_egg", "minecraft:magma_cube"),
        ("minecraft:mooshroom_spawn_egg", "minecraft:mooshroom"),
        ("minecraft:mule_spawn_egg", "minecraft:mule"),
        ("minecraft:ocelot_spawn_egg", "minecraft:ocelot"),
        ("minecraft:panda_spawn_egg", "minecraft:panda"),
        ("minecraft:parrot_spawn_egg", "minecraft:parrot"),
        ("minecraft:phantom_spawn_egg", "minecraft:phantom"),
        ("minecraft:pig_spawn_egg", "minecraft:pig"),
        ("minecraft:piglin_spawn_egg", "minecraft:piglin"),
        ("minecraft:piglin_brute_spawn_egg", "minecraft:piglin_brute"),
        ("minecraft:pillager_spawn_egg", "minecraft:pillager"),
        ("minecraft:polar_bear_spawn_egg", "minecraft:polar_bear"),
        ("minecraft:pufferfish_spawn_egg", "minecraft:pufferfish"),
        ("minecraft:rabbit_spawn_egg", "minecraft:rabbit"),
        ("minecraft:ravager_spawn_egg", "minecraft:ravager"),
        ("minecraft:salmon_spawn_egg", "minecraft:salmon"),
        ("minecraft:sheep_spawn_egg", "minecraft:sheep"),
        ("minecraft:shulker_spawn_egg", "minecraft:shulker"),
        ("minecraft:silverfish_spawn_egg", "minecraft:silverfish"),
        ("minecraft:skeleton_spawn_egg", "minecraft:skeleton"),
        ("minecraft:skeleton_horse_spawn_egg", "minecraft:skeleton_horse"),
        ("minecraft:slime_spawn_egg", "minecraft:slime"),
        ("minecraft:sniffer_spawn_egg", "minecraft:sniffer"),
        ("minecraft:snow_golem_spawn_egg", "minecraft:snow_golem"),
        ("minecraft:spider_spawn_egg", "minecraft:spider"),
        ("minecraft:squid_spawn_egg", "minecraft:squid"),
        ("minecraft:stray_spawn_egg", "minecraft:stray"),
        ("minecraft:strider_spawn_egg", "minecraft:strider"),
        ("minecraft:tadpole_spawn_egg", "minecraft:tadpole"),
        ("minecraft:trader_llama_spawn_egg", "minecraft:trader_llama"),
        ("minecraft:tropical_fish_spawn_egg", "minecraft:tropical_fish"),
        ("minecraft:turtle_spawn_egg", "minecraft:turtle"),
        ("minecraft:vex_spawn_egg", "minecraft:vex"),
        ("minecraft:villager_spawn_egg", "minecraft:villager"),
        ("minecraft:vindicator_spawn_egg", "minecraft:vindicator"),
        ("minecraft:wandering_trader_spawn_egg", "minecraft:wandering_trader"),
        ("minecraft:warden_spawn_egg", "minecraft:warden"),
        ("minecraft:witch_spawn_egg", "minecraft:witch"),
        ("minecraft:wither_spawn_egg", "minecraft:wither"),
        ("minecraft:wither_skeleton_spawn_egg", "minecraft:wither_skeleton"),
        ("minecraft:wolf_spawn_egg", "minecraft:wolf"),
        ("minecraft:zoglin_spawn_egg", "minecraft:zoglin"),
        ("minecraft:zombie_spawn_egg", "minecraft:zombie"),
        ("minecraft:zombie_horse_spawn_egg", "minecraft:zombie_horse"),
        ("minecraft:zombie_villager_spawn_egg", "minecraft:zombie_villager"),
        ("minecraft:zombified_piglin_spawn_egg", "minecraft:zombified_piglin"),
        ("minecraft:armadillo_spawn_egg", "minecraft:armadillo"),
        ("minecraft:breeze_spawn_egg", "minecraft:breeze"),
        ("minecraft:bogged_spawn_egg", "minecraft:bogged"),
        ("minecraft:creaking_spawn_egg", "minecraft:creaking"),
    ]
}

pub fn entity_for_egg(egg: &str) -> Option<&'static str> {
    all_spawn_eggs().iter().find(|(e, _)| *e == egg).map(|(_, ent)| *ent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_egg_spawns_zombie() {
        assert_eq!(entity_for_egg("minecraft:zombie_spawn_egg"), Some("minecraft:zombie"));
    }
}
