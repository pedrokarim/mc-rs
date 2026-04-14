//! Furnace recipes (smelting).

pub fn smelt_to(input: &str) -> Option<(&'static str, f32)> {
    Some(match input {
        "minecraft:iron_ore" => ("minecraft:iron_ingot", 0.7),
        "minecraft:gold_ore" => ("minecraft:gold_ingot", 1.0),
        "minecraft:copper_ore" => ("minecraft:copper_ingot", 0.7),
        "minecraft:raw_iron" => ("minecraft:iron_ingot", 0.7),
        "minecraft:raw_gold" => ("minecraft:gold_ingot", 1.0),
        "minecraft:raw_copper" => ("minecraft:copper_ingot", 0.7),
        "minecraft:ancient_debris" => ("minecraft:netherite_scrap", 2.0),
        "minecraft:sand" => ("minecraft:glass", 0.1),
        "minecraft:red_sand" => ("minecraft:glass", 0.1),
        "minecraft:clay" | "minecraft:clay_ball" => ("minecraft:brick", 0.3),
        "minecraft:clay_block" => ("minecraft:terracotta", 0.35),
        "minecraft:cactus" => ("minecraft:green_dye", 0.2),
        "minecraft:kelp" => ("minecraft:dried_kelp", 0.1),
        "minecraft:raw_beef" => ("minecraft:cooked_beef", 0.35),
        "minecraft:raw_chicken" => ("minecraft:cooked_chicken", 0.35),
        "minecraft:raw_porkchop" => ("minecraft:cooked_porkchop", 0.35),
        "minecraft:raw_mutton" => ("minecraft:cooked_mutton", 0.35),
        "minecraft:raw_rabbit" => ("minecraft:cooked_rabbit", 0.35),
        "minecraft:raw_cod" => ("minecraft:cooked_cod", 0.35),
        "minecraft:raw_salmon" => ("minecraft:cooked_salmon", 0.35),
        "minecraft:potato" => ("minecraft:baked_potato", 0.35),
        "minecraft:cobblestone" => ("minecraft:stone", 0.1),
        "minecraft:stone" => ("minecraft:smooth_stone", 0.1),
        "minecraft:sandstone" => ("minecraft:smooth_sandstone", 0.1),
        "minecraft:red_sandstone" => ("minecraft:smooth_red_sandstone", 0.1),
        "minecraft:quartz_block" => ("minecraft:smooth_quartz", 0.1),
        "minecraft:netherrack" => ("minecraft:nether_brick", 0.1),
        "minecraft:wet_sponge" => ("minecraft:sponge", 0.15),
        "minecraft:nether_quartz_ore" => ("minecraft:quartz", 0.2),
        "minecraft:lapis_ore" => ("minecraft:lapis_lazuli", 0.2),
        "minecraft:diamond_ore" => ("minecraft:diamond", 1.0),
        "minecraft:emerald_ore" => ("minecraft:emerald", 1.0),
        "minecraft:redstone_ore" => ("minecraft:redstone", 0.7),
        "minecraft:coal_ore" => ("minecraft:coal", 0.1),
        _ => return None,
    })
}

/// Default smelt time (200 ticks = 10s).
pub const DEFAULT_SMELT_TIME: u32 = 200;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iron_smelts() {
        assert!(smelt_to("minecraft:iron_ore").is_some());
    }
}
