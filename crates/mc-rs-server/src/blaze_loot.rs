//! Blaze loot drops.

use rand::Rng;

pub fn drops(looting: u32, killed_by_player: bool) -> Vec<(&'static str, u32)> {
    let mut drops = Vec::new();
    if !killed_by_player {
        return drops;
    }
    let mut rng = rand::thread_rng();
    // Blaze rod drops only on player kill (50% + looting bonus).
    let rod_chance = 0.5 + 0.1 * looting as f32;
    if rng.gen::<f32>() < rod_chance {
        let count = 1 + rng.gen_range(0..=(looting / 2));
        drops.push(("minecraft:blaze_rod", count));
    }
    // Glowstone dust on kill (rare).
    if rng.gen::<f32>() < 0.15 {
        drops.push(("minecraft:glowstone_dust", rng.gen_range(0..=2)));
    }
    drops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_player_no_rod() {
        let d = drops(0, false);
        assert!(d.is_empty());
    }
}
