//! Stray loot — different from regular skeleton.

pub fn stray_drops() -> Vec<(&'static str, u32)> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut drops = Vec::new();
    drops.push(("minecraft:bone", rng.gen_range(0..=2)));
    // 50% chance for tipped arrow of slowness.
    if rng.gen::<f32>() < 0.5 {
        drops.push(("minecraft:tipped_arrow", rng.gen_range(0..=1)));
    }
    drops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stray_drops_bones() {
        let d = stray_drops();
        assert!(d.iter().any(|(i, _)| *i == "minecraft:bone"));
    }
}
