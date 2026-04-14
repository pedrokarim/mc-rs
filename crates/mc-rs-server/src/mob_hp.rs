//! Mob HP values (default max health).

pub fn mob_max_hp(entity_type: &str) -> f32 {
    match entity_type {
        // Hostile
        "minecraft:zombie" | "minecraft:husk" | "minecraft:zombie_villager" => 20.0,
        "minecraft:drowned" => 20.0,
        "minecraft:skeleton" | "minecraft:stray" | "minecraft:bogged" => 20.0,
        "minecraft:wither_skeleton" => 20.0,
        "minecraft:creeper" => 20.0,
        "minecraft:spider" => 16.0,
        "minecraft:cave_spider" => 12.0,
        "minecraft:enderman" => 40.0,
        "minecraft:ender_dragon" => 200.0,
        "minecraft:wither" => 300.0,
        "minecraft:slime" => 1.0,   // size 1
        "minecraft:magma_cube" => 1.0,
        "minecraft:blaze" => 20.0,
        "minecraft:ghast" => 10.0,
        "minecraft:guardian" => 30.0,
        "minecraft:elder_guardian" => 80.0,
        "minecraft:witch" => 26.0,
        "minecraft:vex" => 14.0,
        "minecraft:evoker" => 24.0,
        "minecraft:vindicator" => 24.0,
        "minecraft:pillager" => 24.0,
        "minecraft:ravager" => 100.0,
        "minecraft:phantom" => 20.0,
        "minecraft:shulker" => 30.0,
        "minecraft:hoglin" => 40.0,
        "minecraft:zoglin" => 40.0,
        "minecraft:piglin" | "minecraft:zombified_piglin" => 16.0,
        "minecraft:piglin_brute" => 50.0,
        "minecraft:silverfish" => 8.0,
        "minecraft:endermite" => 8.0,
        "minecraft:warden" => 500.0,
        "minecraft:breeze" => 30.0,
        // Passive
        "minecraft:pig" | "minecraft:cow" | "minecraft:mooshroom" | "minecraft:sheep" => 10.0,
        "minecraft:chicken" => 4.0,
        "minecraft:rabbit" => 3.0,
        "minecraft:wolf" => 8.0, // untamed
        "minecraft:cat" => 10.0,
        "minecraft:ocelot" => 10.0,
        "minecraft:parrot" => 6.0,
        "minecraft:bat" => 6.0,
        "minecraft:fox" => 10.0,
        "minecraft:bee" => 10.0,
        "minecraft:horse" | "minecraft:donkey" | "minecraft:mule" => 15.0,
        "minecraft:llama" => 15.0,
        "minecraft:camel" => 32.0,
        "minecraft:goat" => 10.0,
        "minecraft:frog" => 10.0,
        "minecraft:sniffer" => 14.0,
        "minecraft:allay" => 20.0,
        "minecraft:axolotl" => 14.0,
        "minecraft:dolphin" => 10.0,
        "minecraft:squid" | "minecraft:glow_squid" => 10.0,
        "minecraft:cod" | "minecraft:salmon" | "minecraft:pufferfish" | "minecraft:tropical_fish" => 3.0,
        "minecraft:turtle" => 30.0,
        "minecraft:polar_bear" => 30.0,
        "minecraft:panda" => 20.0,
        "minecraft:strider" => 20.0,
        "minecraft:iron_golem" => 100.0,
        "minecraft:snow_golem" => 4.0,
        "minecraft:armadillo" => 12.0,
        "minecraft:happy_ghast" => 20.0,
        "minecraft:creaking" => 1.0,
        // Villagers
        "minecraft:villager" => 20.0,
        "minecraft:wandering_trader" => 20.0,
        "minecraft:player" => 20.0,
        _ => 10.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warden_high_hp() {
        assert!(mob_max_hp("minecraft:warden") > mob_max_hp("minecraft:zombie"));
    }

    #[test]
    fn chicken_low_hp() {
        assert!(mob_max_hp("minecraft:chicken") < 10.0);
    }
}
