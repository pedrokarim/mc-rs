//! Chunk random ticking — port PMMP `World::doChunkRandomTick`.
//! À chaque tick, sélectionne N positions aléatoires dans chaque chunk chargé
//! et tick le bloc dessus. Utilisé pour grow crops, melt ice, spread fire, etc.

use rand::Rng;

/// Nombre de random ticks par section (16³) selon game rule randomtickspeed.
pub fn ticks_per_section(random_tick_speed: u32) -> u32 {
    random_tick_speed
}

/// Générer N positions aléatoires dans une section 16×16×16.
pub fn random_positions_in_section(count: u32) -> Vec<(u8, u8, u8)> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| {
            (
                rng.gen_range(0..16),
                rng.gen_range(0..16),
                rng.gen_range(0..16),
            )
        })
        .collect()
}

/// Chance qu'un bloc grass se propage : besoin de lumière ≥ 4.
pub fn grass_spread_chance(light_level: u8) -> f32 {
    if light_level >= 4 {
        0.25
    } else {
        0.0
    }
}

/// Chance de melt snow_layer (température biome + light).
pub fn snow_melt_chance(temperature: f32, light_level: u8) -> f32 {
    if temperature < 0.15 || light_level < 10 {
        0.0
    } else {
        0.1
    }
}

/// Chance de freeze water (froid + no light).
pub fn water_freeze_chance(temperature: f32) -> f32 {
    if temperature < 0.15 {
        0.1
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_tick_speed_3_default() {
        assert_eq!(ticks_per_section(3), 3);
    }

    #[test]
    fn grass_no_spread_in_dark() {
        assert_eq!(grass_spread_chance(3), 0.0);
        assert!(grass_spread_chance(4) > 0.0);
    }

    #[test]
    fn snow_melts_in_warm_biome() {
        assert!(snow_melt_chance(0.7, 15) > 0.0);
        assert_eq!(snow_melt_chance(0.0, 15), 0.0);
    }
}
