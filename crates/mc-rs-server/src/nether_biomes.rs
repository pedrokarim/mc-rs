//! Nether biomes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetherBiome {
    NetherWastes, // Vanilla
    SoulSandValley,
    CrimsonForest,
    WarpedForest,
    BasaltDeltas,
}

impl NetherBiome {
    pub fn id(&self) -> u8 {
        match self {
            Self::NetherWastes => 8,
            Self::SoulSandValley => 170,
            Self::CrimsonForest => 171,
            Self::WarpedForest => 172,
            Self::BasaltDeltas => 173,
        }
    }

    pub fn native_mobs(&self) -> &'static [&'static str] {
        match self {
            Self::NetherWastes => &[
                "minecraft:ghast",
                "minecraft:magma_cube",
                "minecraft:zombified_piglin",
                "minecraft:piglin",
            ],
            Self::SoulSandValley => &[
                "minecraft:skeleton",
                "minecraft:ghast",
                "minecraft:endermite",
                "minecraft:strider",
            ],
            Self::CrimsonForest => &[
                "minecraft:piglin",
                "minecraft:hoglin",
                "minecraft:zombified_piglin",
                "minecraft:strider",
            ],
            Self::WarpedForest => &["minecraft:enderman", "minecraft:strider"],
            Self::BasaltDeltas => &[
                "minecraft:magma_cube",
                "minecraft:ghast",
                "minecraft:strider",
            ],
        }
    }

    pub fn ambient_fog_color(&self) -> u32 {
        match self {
            Self::NetherWastes => 0x330808,
            Self::SoulSandValley => 0x1B4745,
            Self::CrimsonForest => 0x330303,
            Self::WarpedForest => 0x1A051A,
            Self::BasaltDeltas => 0x685F70,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crimson_has_hoglin() {
        assert!(NetherBiome::CrimsonForest
            .native_mobs()
            .contains(&"minecraft:hoglin"));
    }
}
