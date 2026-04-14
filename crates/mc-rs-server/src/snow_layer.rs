//! Snow layer — 1-8 layers, accumulates during snow.

/// Max layers.
pub const MAX_LAYERS: u8 = 8;
/// Layer height (1/8 block).
pub const LAYER_HEIGHT: f32 = 0.125;

/// Accumulation chance per snow tick.
pub const ACCUMULATION_CHANCE: f32 = 0.1;

/// Breakdown: silk_touch/shovel drops snowballs.
pub fn snowballs_dropped(layers: u8, silk_touch: bool) -> u32 {
    if silk_touch {
        layers as u32
    } else {
        layers as u32
    }
}

/// Only accumulate in cold biomes.
pub fn biome_allows_snow(temperature: f32) -> bool {
    temperature < 0.15
}

/// Torches and bright lights melt snow (light > 11).
pub const MELT_LIGHT_THRESHOLD: u8 = 11;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_biome_no_snow() {
        assert!(!biome_allows_snow(1.0));
    }

    #[test]
    fn cold_biome_snows() {
        assert!(biome_allows_snow(0.0));
    }
}
