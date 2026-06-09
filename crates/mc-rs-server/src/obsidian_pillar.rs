//! End spikes (piliers d'obsidienne) — spec vanilla `SpikeFeature`.
//!
//! 10 piliers à coordonnées **fixes** sur un cercle de rayon 42 autour du portail
//! de sortie (0,0). Chaque pilier a un rayon (3..6), une hauteur (76..103, pas de 3)
//! et un flag `guarded` (cage de barreaux de fer autour du cristal). Le mapping
//! position→(rayon,hauteur,cage) est mélangé par la seed (comme vanilla).
//!
//! Réf : minecraft.wiki/w/Obsidian_pillar + `net.minecraft...SpikeFeature`. La
//! colonne d'obsidienne va de Y=0 à `height`, coiffée d'un bloc de bedrock ; le
//! **cristal de l'End se pose à `height + 1`** (sur la bedrock) — c'est une entité
//! spawnée par `dragon_fight_manager`, pas par le terrain.

use crate::world::random::Random;

/// Les 10 positions fixes des piliers (indépendantes de la seed).
const POSITIONS: [(i32, i32); 10] = [
    (-42, -1),
    (-34, -25),
    (-34, 24),
    (-13, -40),
    (-13, 39),
    (12, -40),
    (12, 39),
    (33, -25),
    (33, 24),
    (42, 0),
];

#[derive(Debug, Clone, Copy)]
pub struct EndSpike {
    pub x: i32,
    pub z: i32,
    /// Rayon de la colonne d'obsidienne (3..6).
    pub radius: i32,
    /// Y du sommet (bloc de bedrock ici ; le cristal est à `height + 1`).
    pub height: i32,
    /// Cage de barreaux de fer autour du cristal (2 plus courts piliers gardés).
    pub guarded: bool,
}

impl EndSpike {
    /// Y où se pose le cristal de l'End (sur la bedrock du sommet).
    pub fn crystal_y(&self) -> i32 {
        self.height + 1
    }
}

/// Les 10 end spikes pour une seed donnée. Le rang (0..9) attribué à chaque
/// position est mélangé par la seed ; rang → rayon `3 + rang/3`, hauteur
/// `76 + rang*3`, gardé si rang ∈ {1,2} (2e et 3e plus courts).
pub fn end_spikes(seed: u64) -> Vec<EndSpike> {
    let mut ranks: [usize; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut rng = Random::new(seed as i64);
    // Fisher-Yates déterministe (mélange position→rang par seed).
    for i in (1..10).rev() {
        let j = rng.next_bounded_int(i as i32 + 1) as usize;
        ranks.swap(i, j);
    }
    POSITIONS
        .iter()
        .enumerate()
        .map(|(i, &(x, z))| {
            let l = ranks[i] as i32;
            EndSpike {
                x,
                z,
                radius: 3 + l / 3,  // 3,3,3,4,4,4,5,5,5,6
                height: 76 + l * 3, // 76..103
                guarded: l == 1 || l == 2,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_spikes_with_vanilla_specs() {
        let spikes = end_spikes(42);
        assert_eq!(spikes.len(), 10);
        // L'ensemble des hauteurs = {76,79,...,103} (peu importe la seed).
        let mut heights: Vec<i32> = spikes.iter().map(|s| s.height).collect();
        heights.sort();
        assert_eq!(heights, vec![76, 79, 82, 85, 88, 91, 94, 97, 100, 103]);
        // Exactement 2 piliers gardés (cages de fer).
        assert_eq!(spikes.iter().filter(|s| s.guarded).count(), 2);
        // Rayons dans 3..=6.
        assert!(spikes.iter().all(|s| (3..=6).contains(&s.radius)));
        // Le cristal se pose au-dessus de la bedrock.
        assert!(spikes.iter().all(|s| s.crystal_y() == s.height + 1));
    }

    #[test]
    fn positions_are_fixed_and_distinct() {
        let a = end_spikes(1);
        let b = end_spikes(999);
        // Mêmes positions quelle que soit la seed (seul le mapping rang change).
        let pa: Vec<_> = a.iter().map(|s| (s.x, s.z)).collect();
        let pb: Vec<_> = b.iter().map(|s| (s.x, s.z)).collect();
        assert_eq!(pa, pb);
        for i in 0..a.len() {
            for j in (i + 1)..a.len() {
                assert_ne!((a[i].x, a[i].z), (a[j].x, a[j].z));
            }
        }
    }

    #[test]
    fn guarded_are_the_two_shortest_above_min() {
        // Les piliers gardés sont les hauteurs 79 et 82 (2e et 3e plus courts).
        let spikes = end_spikes(7);
        let guarded_heights: Vec<i32> = spikes
            .iter()
            .filter(|s| s.guarded)
            .map(|s| s.height)
            .collect();
        let mut gh = guarded_heights;
        gh.sort();
        assert_eq!(gh, vec![79, 82]);
    }
}
