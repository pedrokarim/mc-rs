//! Recherche de chemin au sol — port de `FlatAStarRouteFinder` +
//! `GroundPosEvaluator`/`WalkingPosEvaluator` d'Allay.
//!
//! A\* 2D sur le plan XZ pour mobs terrestres : 8 voisins (cardinaux +
//! diagonales avec anti-corner-cutting), franchissement d'**une marche** vers le
//! haut et **chute** jusqu'à `max_fall` blocs, heuristique **octile** (admissible
//! et consistante pour un déplacement 8-directionnel). Si la cible exacte est
//! inatteignable, renvoie un chemin partiel vers le nœud le plus proche.
//!
//! Le critère « walkable » est fourni par l'appelant via un prédicat
//! `walkable(x, y, z)` (le mob peut se tenir debout, pieds en `y`) — ce qui rend
//! le finder testable sans `ChunkCache` réel. `super::route::walkable_ground`
//! construit ce prédicat depuis le `ChunkCache` (sol solide + pieds/tête libres).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::mob_entities::is_supporting_block;
use crate::world::chunk_cache::ChunkCache;

/// `sqrt(2) - 1`, coût diagonal supplémentaire de l'heuristique octile.
const SQRT2_MINUS_1: f64 = std::f64::consts::SQRT_2 - 1.0;

/// Voisins plats (dx, dz) : 4 cardinaux puis 4 diagonaux.
const FLAT_NEIGHBORS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// Paramètres par défaut (port `FlatAStarRouteFinder` : 100 nœuds, chute 3).
pub const DEFAULT_MAX_EXPANDED: usize = 100;
pub const DEFAULT_MAX_FALL: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Pos {
    x: i32,
    y: i32,
    z: i32,
}

/// Entrée de file de priorité (min-heap sur `f = g + h`).
struct Frontier {
    f: f64,
    pos: Pos,
}
impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f
    }
}
impl Eq for Frontier {}
impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap : on inverse pour que le plus petit `f` sorte en premier.
        other.f.total_cmp(&self.f)
    }
}
impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heuristique octile (horizontale uniquement).
fn heuristic(a: Pos, goal: Pos) -> f64 {
    let dx = (a.x - goal.x).abs() as f64;
    let dz = (a.z - goal.z).abs() as f64;
    dx.max(dz) + SQRT2_MINUS_1 * dx.min(dz)
}

/// Distance réelle entre deux nœuds adjacents (inclut la composante verticale).
fn step_cost(a: Pos, b: Pos) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    let dz = (a.z - b.z) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Y praticable le plus proche en `(x, z)` autour de `cy` : descend de +1 à
/// `-max_fall`. Renvoie `Some(ny)` au premier niveau walkable.
fn walkable_y<F: FnMut(i32, i32, i32) -> bool>(
    x: i32,
    cy: i32,
    z: i32,
    max_fall: i32,
    walkable: &mut F,
) -> Option<i32> {
    let mut dy = 1;
    while dy >= -max_fall {
        if walkable(x, cy + dy, z) {
            return Some(cy + dy);
        }
        dy -= 1;
    }
    None
}

/// A\* sol. `start`/`target` en coordonnées monde ; renvoie la liste des
/// waypoints (centres de blocs, `y` = niveau des pieds) **hors** nœud de départ.
/// Vide si déjà arrivé ou aucun progrès possible.
pub fn find_ground_path<F: FnMut(i32, i32, i32) -> bool>(
    start: [f32; 3],
    target: [f64; 3],
    max_expanded: usize,
    max_fall: i32,
    mut walkable: F,
) -> Vec<[f64; 3]> {
    let start_pos = Pos {
        x: start[0].floor() as i32,
        y: start[1].floor() as i32,
        z: start[2].floor() as i32,
    };
    let goal = Pos {
        x: target[0].floor() as i32,
        y: target[1].floor() as i32,
        z: target[2].floor() as i32,
    };

    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<Pos, f64> = HashMap::new();
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    let mut closed: HashSet<Pos> = HashSet::new();

    g_score.insert(start_pos, 0.0);
    open.push(Frontier {
        f: heuristic(start_pos, goal),
        pos: start_pos,
    });

    // Meilleur nœud atteint (h minimale) pour un chemin partiel.
    let mut best = start_pos;
    let mut best_h = heuristic(start_pos, goal);

    let mut expanded = 0;
    while let Some(Frontier { pos: current, .. }) = open.pop() {
        if closed.contains(&current) {
            continue;
        }
        if is_close_enough(current, goal, max_fall) {
            return reconstruct(&came_from, current, start_pos);
        }
        closed.insert(current);
        expanded += 1;
        if expanded >= max_expanded {
            break;
        }

        let h = heuristic(current, goal);
        if h < best_h {
            best_h = h;
            best = current;
        }

        let cur_g = *g_score.get(&current).unwrap_or(&f64::MAX);
        for (dx, dz) in FLAT_NEIGHBORS {
            let nx = current.x + dx;
            let nz = current.z + dz;

            // Anti-corner-cutting : pour une diagonale, au moins un cardinal
            // adjacent doit être walkable.
            if dx != 0 && dz != 0 {
                let card_x = walkable_y(
                    current.x + dx,
                    current.y,
                    current.z,
                    max_fall,
                    &mut walkable,
                )
                .is_some();
                let card_z = walkable_y(
                    current.x,
                    current.y,
                    current.z + dz,
                    max_fall,
                    &mut walkable,
                )
                .is_some();
                if !card_x && !card_z {
                    continue;
                }
            }

            let Some(ny) = walkable_y(nx, current.y, nz, max_fall, &mut walkable) else {
                continue;
            };
            let neighbor = Pos {
                x: nx,
                y: ny,
                z: nz,
            };
            if closed.contains(&neighbor) {
                continue;
            }

            let tentative = cur_g + step_cost(current, neighbor);
            if tentative < *g_score.get(&neighbor).unwrap_or(&f64::MAX) {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative);
                open.push(Frontier {
                    f: tentative + heuristic(neighbor, goal),
                    pos: neighbor,
                });
            }
        }
    }

    // Cible inatteignable : chemin partiel vers le nœud le plus proche.
    if best != start_pos {
        return reconstruct(&came_from, best, start_pos);
    }
    Vec::new()
}

fn is_close_enough(a: Pos, goal: Pos, max_fall: i32) -> bool {
    let dx = (a.x - goal.x) as f64;
    let dz = (a.z - goal.z) as f64;
    dx * dx + dz * dz < 1.0 && (a.y - goal.y).abs() <= max_fall
}

/// Reconstruit le chemin du départ vers `end`, **sans** le nœud de départ.
fn reconstruct(came_from: &HashMap<Pos, Pos>, end: Pos, start: Pos) -> Vec<[f64; 3]> {
    let mut chain = vec![end];
    let mut cur = end;
    while let Some(&prev) = came_from.get(&cur) {
        chain.push(prev);
        cur = prev;
        if cur == start {
            break;
        }
    }
    chain.reverse();
    // Drop le nœud de départ (le mob y est déjà).
    chain
        .into_iter()
        .filter(|p| *p != start)
        .map(|p| [p.x as f64 + 0.5, p.y as f64, p.z as f64 + 0.5])
        .collect()
}

/// Construit le prédicat « walkable » depuis le `ChunkCache` : le mob peut se
/// tenir debout (pieds en `y`) si le bloc en `y-1` est un support solide et que
/// les blocs en `y` (pieds) et `y+1` (tête) sont franchissables. Port simplifié
/// de `WalkingPosEvaluator` (sol solide + AABB libre ; lave/eau exclues via
/// `is_supporting_block`).
pub fn walkable_ground(cache: &mut ChunkCache, x: i32, y: i32, z: i32) -> bool {
    if !is_supporting_block(cache.get_block(x, y - 1, z)) {
        return false; // pas de sol solide en dessous
    }
    // Pieds + tête doivent être franchissables (non solides).
    !is_supporting_block(cache.get_block(x, y, z))
        && !is_supporting_block(cache.get_block(x, y + 1, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monde de test : ensemble de blocs solides ; walkable = sol solide +
    /// pieds/tête libres.
    fn make_walkable(solids: HashSet<(i32, i32, i32)>) -> impl Fn(i32, i32, i32) -> bool {
        move |x, y, z| {
            solids.contains(&(x, y - 1, z))
                && !solids.contains(&(x, y, z))
                && !solids.contains(&(x, y + 1, z))
        }
    }

    /// Plan plein à `y = ground_y` sur `[-r, r]²`.
    fn flat(ground_y: i32, r: i32) -> HashSet<(i32, i32, i32)> {
        let mut s = HashSet::new();
        for x in -r..=r {
            for z in -r..=r {
                s.insert((x, ground_y, z));
            }
        }
        s
    }

    #[test]
    fn straight_line_on_flat_ground() {
        let walkable = make_walkable(flat(63, 10));
        let path = find_ground_path([0.5, 64.0, 0.5], [4.0, 64.0, 0.0], 200, 3, walkable);
        assert!(!path.is_empty(), "un chemin doit exister sur terrain plat");
        // Dernier waypoint proche de la cible en X.
        let last = *path.last().unwrap();
        assert!(
            (last[0] - 4.5).abs() < 1.5,
            "fin proche de la cible, got {last:?}"
        );
    }

    #[test]
    fn steps_up_one_block() {
        // Sol à y=63 partout, sauf une marche à y=64 en x=2 (sol y=64 → debout y=65).
        let mut solids = flat(63, 6);
        solids.insert((2, 64, 0)); // bloc surélevé : crée une marche
        let walkable = make_walkable(solids);
        let path = find_ground_path([0.5, 64.0, 0.5], [4.0, 64.0, 0.0], 300, 3, walkable);
        assert!(!path.is_empty(), "doit franchir la marche d'1 bloc");
    }

    #[test]
    fn routes_around_a_wall() {
        // Mur solide (3 de haut) sur la colonne x=1 pour z in -1..=1, forçant un détour.
        let mut solids = flat(63, 6);
        for z in -1..=1 {
            for y in 64..=66 {
                solids.insert((1, y, z));
            }
        }
        let walkable = make_walkable(solids);
        let path = find_ground_path([0.5, 64.0, 0.5], [3.0, 64.0, 0.0], 500, 3, walkable);
        assert!(!path.is_empty(), "un détour autour du mur doit exister");
        // Aucun waypoint ne traverse la colonne du mur en x=1, z in -1..=1.
        for wp in &path {
            let (wx, wz) = (wp[0].floor() as i32, wp[2].floor() as i32);
            assert!(
                !(wx == 1 && (-1..=1).contains(&wz)),
                "waypoint {wp:?} traverse le mur"
            );
        }
    }

    #[test]
    fn no_path_when_fully_boxed_in() {
        // Mob enfermé : un seul bloc de sol, entouré de murs hauts.
        let mut solids: HashSet<(i32, i32, i32)> = HashSet::new();
        solids.insert((0, 63, 0)); // sol unique
        for (dx, dz) in FLAT_NEIGHBORS {
            for y in 63..=66 {
                solids.insert((dx, y, dz));
            }
        }
        let walkable = make_walkable(solids);
        let path = find_ground_path([0.5, 64.0, 0.5], [5.0, 64.0, 5.0], 200, 3, walkable);
        assert!(path.is_empty(), "aucun mouvement possible → chemin vide");
    }
}
