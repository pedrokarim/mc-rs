//! XP orbs dropped by mobs.

pub fn mob_xp_drop(entity_type: &str) -> (u32, u32) {
    match entity_type {
        // Hostile
        "minecraft:zombie"
        | "minecraft:skeleton"
        | "minecraft:creeper"
        | "minecraft:stray"
        | "minecraft:bogged"
        | "minecraft:wither_skeleton"
        | "minecraft:husk"
        | "minecraft:drowned"
        | "minecraft:zombified_piglin"
        | "minecraft:zoglin"
        | "minecraft:phantom"
        | "minecraft:vex"
        | "minecraft:piglin" => (5, 5),
        "minecraft:spider" | "minecraft:cave_spider" => (5, 5),
        "minecraft:silverfish" | "minecraft:endermite" => (5, 5),
        "minecraft:enderman" => (5, 5),
        "minecraft:guardian" => (10, 10),
        "minecraft:elder_guardian" => (10, 10),
        "minecraft:blaze" => (10, 10),
        "minecraft:ghast" => (5, 5),
        "minecraft:magma_cube" => (1, 4),
        "minecraft:slime" => (1, 4),
        "minecraft:witch" => (5, 5),
        "minecraft:evoker" => (10, 10),
        "minecraft:vindicator" | "minecraft:pillager" | "minecraft:ravager" => (5, 5),
        "minecraft:piglin_brute" => (20, 20),
        "minecraft:hoglin" => (1, 3),
        "minecraft:shulker" => (5, 5),
        "minecraft:warden" => (5, 5),
        "minecraft:ender_dragon" => (12000, 12000),
        "minecraft:wither" => (50, 50),
        "minecraft:breeze" => (10, 10),
        // Passive/breeding
        "minecraft:pig"
        | "minecraft:cow"
        | "minecraft:mooshroom"
        | "minecraft:sheep"
        | "minecraft:chicken"
        | "minecraft:rabbit"
        | "minecraft:wolf"
        | "minecraft:cat"
        | "minecraft:ocelot"
        | "minecraft:fox"
        | "minecraft:parrot"
        | "minecraft:turtle"
        | "minecraft:bee"
        | "minecraft:polar_bear"
        | "minecraft:panda"
        | "minecraft:axolotl"
        | "minecraft:strider"
        | "minecraft:goat"
        | "minecraft:frog"
        | "minecraft:sniffer"
        | "minecraft:camel"
        | "minecraft:horse"
        | "minecraft:donkey"
        | "minecraft:mule"
        | "minecraft:llama"
        | "minecraft:allay" => (1, 3),
        _ => (0, 0),
    }
}

/// Breeding XP (successful pair = 1-7 XP).
pub const BREEDING_XP_MIN: u32 = 1;
pub const BREEDING_XP_MAX: u32 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_gives_12000() {
        assert_eq!(mob_xp_drop("minecraft:ender_dragon"), (12000, 12000));
    }
}
