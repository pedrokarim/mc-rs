//! Obsidian pillars (End) — contain crystals; destroyed to weaken dragon.

#[derive(Debug, Clone, Copy)]
pub struct PillarLocation {
    pub x: i32,
    pub z: i32,
    pub height: u8,
    pub crystal_iron_cage: bool,
}

/// Vanilla 10 pillar layout.
pub fn vanilla_pillar_locations() -> Vec<PillarLocation> {
    let radius = 43.0;
    let heights = [43, 50, 41, 47, 55, 45, 57, 51, 49, 53];
    (0..10)
        .map(|i| {
            let angle = (i as f64) * std::f64::consts::TAU / 10.0;
            PillarLocation {
                x: (angle.cos() * radius) as i32,
                z: (angle.sin() * radius) as i32,
                height: heights[i],
                crystal_iron_cage: heights[i] > 50,
            }
        })
        .collect()
}

/// Cage blocks prevent eye access (iron bars on top 4 sides).
pub const CAGE_BARS_FACES: [[i32; 3]; 4] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_pillars() {
        assert_eq!(vanilla_pillar_locations().len(), 10);
    }

    #[test]
    fn pillars_distinct_positions() {
        let pillars = vanilla_pillar_locations();
        for i in 0..pillars.len() {
            for j in (i + 1)..pillars.len() {
                assert_ne!(
                    (pillars[i].x, pillars[i].z),
                    (pillars[j].x, pillars[j].z)
                );
            }
        }
    }
}
