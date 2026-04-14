//! Campfire recipes — food cooking without fuel.

pub fn campfire_cook(input: &str) -> Option<&'static str> {
    Some(match input {
        "minecraft:raw_beef" => "minecraft:cooked_beef",
        "minecraft:raw_chicken" => "minecraft:cooked_chicken",
        "minecraft:raw_porkchop" => "minecraft:cooked_porkchop",
        "minecraft:raw_mutton" => "minecraft:cooked_mutton",
        "minecraft:raw_rabbit" => "minecraft:cooked_rabbit",
        "minecraft:raw_cod" => "minecraft:cooked_cod",
        "minecraft:raw_salmon" => "minecraft:cooked_salmon",
        "minecraft:potato" => "minecraft:baked_potato",
        "minecraft:kelp" => "minecraft:dried_kelp",
        _ => return None,
    })
}

/// Cook time (600 ticks = 30s, no fuel needed).
pub const COOK_TIME: u32 = 600;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beef_cooks() {
        assert_eq!(campfire_cook("minecraft:raw_beef"), Some("minecraft:cooked_beef"));
    }
}
