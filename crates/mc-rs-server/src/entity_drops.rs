//! Entity drops — mob-specific loot tables.

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Drop {
    pub item: &'static str,
    pub min: u32,
    pub max: u32,
    pub looting_bonus: u32,
}

/// Zombie drops.
pub fn zombie_drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let mut drops = Vec::new();
    // Rotten flesh (0-2, +1 per looting).
    let max = 2 + looting;
    drops.push(("minecraft:rotten_flesh", rng.gen_range(0..=max)));
    // Rare: iron ingot, carrot, potato (~3% each).
    if rng.gen::<f32>() < 0.03 + 0.01 * looting as f32 {
        drops.push(("minecraft:iron_ingot", 1));
    }
    drops
}

/// Skeleton drops.
pub fn skeleton_drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let mut drops = Vec::new();
    let bone_max = 2 + looting;
    drops.push(("minecraft:bone", rng.gen_range(0..=bone_max)));
    let arrow_max = 2 + looting;
    drops.push(("minecraft:arrow", rng.gen_range(0..=arrow_max)));
    drops
}

/// Creeper drops.
pub fn creeper_drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let max = 2 + looting;
    vec![("minecraft:gunpowder", rng.gen_range(0..=max))]
}

/// Spider drops.
pub fn spider_drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let mut drops = Vec::new();
    let string_max = 2 + looting;
    drops.push(("minecraft:string", rng.gen_range(0..=string_max)));
    if rng.gen::<f32>() < 1.0 / 3.0 {
        drops.push(("minecraft:spider_eye", 1));
    }
    drops
}

/// Enderman drops.
pub fn enderman_drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let max = 1 + looting;
    vec![("minecraft:ender_pearl", rng.gen_range(0..=max))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombie_drops_at_least_rotten() {
        let drops = zombie_drops(0);
        assert!(drops.iter().any(|(i, _)| *i == "minecraft:rotten_flesh"));
    }
}
