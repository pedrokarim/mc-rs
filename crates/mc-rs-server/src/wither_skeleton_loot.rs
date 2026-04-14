//! Wither Skeleton loot drops.

use rand::Rng;

/// Wither skeleton drops.
pub fn drops(looting: u32) -> Vec<(&'static str, u32)> {
    let mut rng = rand::thread_rng();
    let mut drops = Vec::new();
    let coal_max = 1 + looting;
    drops.push(("minecraft:coal", rng.gen_range(0..=coal_max)));
    let bone_max = 2 + looting;
    drops.push(("minecraft:bone", rng.gen_range(0..=bone_max)));
    // Head drop chance: 2.5%, +1% per looting level.
    let head_chance = 0.025 + 0.01 * looting as f32;
    if rng.gen::<f32>() < head_chance {
        drops.push(("minecraft:wither_skeleton_skull", 1));
    }
    drops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_always_have_bone_entry() {
        let d = drops(0);
        assert!(d.iter().any(|(i, _)| *i == "minecraft:bone"));
    }
}
