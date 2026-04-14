//! Crop growth — wheat, carrots, potatoes, beetroot, melon/pumpkin stems.

#[derive(Debug, Clone, Copy)]
pub struct CropGrowthProperties {
    pub max_stage: u8,
    pub growth_chance_base: f32,     // Per random tick
    pub needs_water: bool,
    pub needs_light_level: u8,
}

pub fn properties_for(crop: &str) -> Option<CropGrowthProperties> {
    Some(match crop {
        "minecraft:wheat" => CropGrowthProperties {
            max_stage: 7, growth_chance_base: 0.10, needs_water: true, needs_light_level: 9,
        },
        "minecraft:carrots" => CropGrowthProperties {
            max_stage: 7, growth_chance_base: 0.10, needs_water: true, needs_light_level: 9,
        },
        "minecraft:potatoes" => CropGrowthProperties {
            max_stage: 7, growth_chance_base: 0.10, needs_water: true, needs_light_level: 9,
        },
        "minecraft:beetroots" => CropGrowthProperties {
            max_stage: 3, growth_chance_base: 0.07, needs_water: true, needs_light_level: 9,
        },
        "minecraft:melon_stem" => CropGrowthProperties {
            max_stage: 7, growth_chance_base: 0.10, needs_water: true, needs_light_level: 9,
        },
        "minecraft:pumpkin_stem" => CropGrowthProperties {
            max_stage: 7, growth_chance_base: 0.10, needs_water: true, needs_light_level: 9,
        },
        "minecraft:sugar_cane" => CropGrowthProperties {
            max_stage: 16, growth_chance_base: 0.08, needs_water: true, needs_light_level: 0,
        },
        "minecraft:cactus" => CropGrowthProperties {
            max_stage: 16, growth_chance_base: 0.10, needs_water: false, needs_light_level: 0,
        },
        "minecraft:bamboo" => CropGrowthProperties {
            max_stage: 16, growth_chance_base: 0.08, needs_water: false, needs_light_level: 9,
        },
        "minecraft:nether_wart" => CropGrowthProperties {
            max_stage: 3, growth_chance_base: 0.10, needs_water: false, needs_light_level: 0,
        },
        "minecraft:cocoa" => CropGrowthProperties {
            max_stage: 2, growth_chance_base: 0.20, needs_water: false, needs_light_level: 0,
        },
        _ => return None,
    })
}

/// Bone meal applies 2-5 stages of growth.
pub fn bone_meal_stages() -> (u8, u8) { (2, 5) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheat_7_stages() {
        assert_eq!(properties_for("minecraft:wheat").unwrap().max_stage, 7);
    }

    #[test]
    fn beetroot_4_stages() {
        assert_eq!(properties_for("minecraft:beetroots").unwrap().max_stage, 3);
    }
}
