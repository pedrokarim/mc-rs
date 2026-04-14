//! Biome color variations — grass, foliage, water, sky.

#[derive(Debug, Clone, Copy)]
pub struct BiomeColors {
    pub grass: [u8; 3],
    pub foliage: [u8; 3],
    pub water: [u8; 3],
    pub sky: [u8; 3],
    pub fog: [u8; 3],
    pub dry_foliage: [u8; 3],
}

/// Default Plains biome colors (vanilla).
pub const PLAINS_COLORS: BiomeColors = BiomeColors {
    grass: [145, 189, 89],
    foliage: [119, 171, 47],
    water: [63, 118, 228],
    sky: [120, 167, 255],
    fog: [192, 216, 255],
    dry_foliage: [118, 142, 81],
};

/// Jungle colors (saturated green).
pub const JUNGLE_COLORS: BiomeColors = BiomeColors {
    grass: [89, 201, 60],
    foliage: [48, 187, 10],
    water: [63, 118, 228],
    sky: [120, 167, 255],
    fog: [192, 216, 255],
    dry_foliage: [118, 142, 81],
};

/// Swamp colors (muddy).
pub const SWAMP_COLORS: BiomeColors = BiomeColors {
    grass: [106, 112, 57],
    foliage: [106, 112, 57],
    water: [97, 123, 100],
    sky: [120, 167, 255],
    fog: [192, 216, 255],
    dry_foliage: [118, 142, 81],
};

/// Mushroom fields colors (magenta-ish).
pub const MUSHROOM_COLORS: BiomeColors = BiomeColors {
    grass: [85, 201, 63],
    foliage: [43, 187, 15],
    water: [63, 118, 228],
    sky: [120, 167, 255],
    fog: [192, 216, 255],
    dry_foliage: [118, 142, 81],
};

/// Snowy biomes.
pub const SNOWY_COLORS: BiomeColors = BiomeColors {
    grass: [128, 180, 151],
    foliage: [96, 161, 123],
    water: [62, 149, 200],
    sky: [129, 184, 208],
    fog: [192, 216, 255],
    dry_foliage: [118, 142, 81],
};

pub fn colors_for_biome(biome_id: u8) -> BiomeColors {
    match biome_id {
        0 | 7 | 24 | 48 => BiomeColors { // Ocean variants
            grass: [141, 179, 96], foliage: [113, 167, 77],
            water: [63, 118, 228], sky: [120, 167, 255],
            fog: [192, 216, 255], dry_foliage: [118, 142, 81],
        },
        6 | 134 => SWAMP_COLORS,
        11..=30 => SNOWY_COLORS,
        14 | 15 => MUSHROOM_COLORS,
        21..=23 | 149 | 151 => JUNGLE_COLORS,
        _ => PLAINS_COLORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swamp_has_different_water() {
        assert_ne!(SWAMP_COLORS.water, PLAINS_COLORS.water);
    }
}
