//! Mushroom biome (MushroomFields) — special decoration rules.

/// No hostile mobs spawn in mushroom biome.
pub fn allows_hostile_spawn() -> bool {
    false
}

/// Only mooshrooms spawn passive.
pub fn passive_mob_for_biome() -> &'static [&'static str] {
    &["minecraft:mooshroom"]
}

/// Giant red mushroom spawn chance.
pub const GIANT_MUSHROOM_DENSITY: f32 = 0.01;
/// Min distance between mushroom clusters.
pub const MUSHROOM_CLUSTER_MIN_DIST: f64 = 6.0;

pub fn default_block_id() -> u16 {
    110 // Mycelium
}

pub fn top_blocks() -> &'static [u16] {
    &[110, 40, 39] // Mycelium, Red mushroom, Brown mushroom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaceful_biome() {
        assert!(!allows_hostile_spawn());
    }

    #[test]
    fn has_mooshroom() {
        assert!(passive_mob_for_biome().contains(&"minecraft:mooshroom"));
    }
}
