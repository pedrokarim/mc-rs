//! Bamboo jungle biome variants.

pub const BIOME_ID: u8 = 168;
pub const BIOME_HILLS_ID: u8 = 169;

pub fn unique_features() -> &'static [&'static str] {
    &[
        "minecraft:bamboo",
        "minecraft:jungle_log",
        "minecraft:melon",
        "minecraft:cocoa",
        "minecraft:vine",
    ]
}

/// Panda spawn exclusive to bamboo jungle.
pub fn panda_spawns_here() -> bool { true }

#[cfg(test)]
mod tests {
    #[test]
    fn has_bamboo() {
        assert!(super::unique_features().contains(&"minecraft:bamboo"));
    }
}
