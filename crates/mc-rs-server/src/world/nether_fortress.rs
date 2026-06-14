//! Forteresse du Nether — **générateur procédural** (aucune référence Bedrock ne
//! l'implémente : Nukkit/Allay/PMMP n'ont que des blocs/paquets « structure »).
//!
//! Modèle : un **plan de cellules** déterministe par région (grille spacing/separation
//! comme les structures vanilla), assemblé par croissance type BFS depuis une cellule
//! d'origine. Chaque cellule (8×8) est rendue en nether brick : ponts à rambardes
//! (`Bridge`), couloirs fermés (`Corridor`), plateforme de spawner de blaze (`Blaze`),
//! salle de nether wart (`Wart`), coffre (`Chest`). Les cellules adjacentes laissent
//! une ouverture → couloirs/ponts continus.
//!
//! Le rendu est **par chunk** : pour chaque chunk on retrouve la forteresse proche
//! dont des cellules débordent, et on écrit (via `DimGrid::set`, clippé) les blocs qui
//! tombent dans le chunk. Déterministe et cohérent d'un chunk à l'autre.
//!
//! v1 : la structure (blocs) est complète ; le **block-entity** du spawner (données
//! de spawn blaze) et le **butin** des coffres sont des suivis (nécessitent les block
//! entities en chunk).

use std::collections::HashMap;
use std::sync::LazyLock;

use super::block_registry::BLOCKS;
use super::worldgen::dim_noise::DimGrid;

const CELL: i32 = 8; // taille d'une cellule de plan (blocs)
const SPACING: i32 = 27; // chunks entre régions de forteresse
const SEPARATION: i32 = 4; // marge dans la région
const MAX_CELLS: usize = 24; // taille max d'une forteresse (cellules)
const PLAN_RADIUS: i32 = 5; // rayon max du plan (cellules) autour de l'origine
const BASE_Y: i32 = 64; // niveau du pont (deck à BASE_Y-1, on marche sur BASE_Y)

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bridge,
    Corridor,
    Blaze,
    Wart,
    Chest,
}

struct FortressBlocks {
    air: u32,
    brick: u32,
    fence: u32,
    wart: u32,
    soul_sand: u32,
    spawner: u32,
    chest: u32,
}

static FB: LazyLock<FortressBlocks> = LazyLock::new(|| FortressBlocks {
    air: BLOCKS.air,
    brick: BLOCKS.get("minecraft:nether_brick"),
    fence: BLOCKS.get("minecraft:nether_brick_fence"),
    wart: BLOCKS.get("minecraft:nether_wart"),
    soul_sand: BLOCKS.get("minecraft:soul_sand"),
    spawner: BLOCKS.get("minecraft:mob_spawner"),
    chest: BLOCKS.get("minecraft:chest"),
});

/// splitmix64 déterministe.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn hash3(a: i32, b: i32, seed: u64) -> u64 {
    mix(seed
        ^ (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
}

/// RNG splitmix local.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = mix(self.0);
        self.0
    }
    fn range(&mut self, n: i32) -> i32 {
        (self.next() % n.max(1) as u64) as i32
    }
}

/// Chunk d'origine de la forteresse de la région `(rx, rz)` (déterministe).
fn fortress_origin_chunk(rx: i32, rz: i32, seed: u64) -> (i32, i32) {
    let h = hash3(rx, rz, seed ^ 0x_F0E7_7E55);
    let span = (SPACING - SEPARATION) as u64;
    let ox = rx * SPACING + (h % span) as i32;
    let oz = rz * SPACING + ((h >> 21) % span) as i32;
    (ox, oz)
}

/// Construit le plan d'une forteresse (cellules absolues + leur type). L'origine est
/// la cellule contenant le chunk d'origine.
fn plan(origin_cx: i32, origin_cz: i32, seed: u64) -> HashMap<(i32, i32), Kind> {
    let mut cells: HashMap<(i32, i32), Kind> = HashMap::new();
    let mut order: Vec<(i32, i32)> = Vec::new();
    let mut rng = Rng(hash3(origin_cx, origin_cz, seed ^ 0x000B_12E5));
    cells.insert((0, 0), Kind::Bridge);
    order.push((0, 0));
    let mut frontier = vec![(0, 0)];
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

    while cells.len() < MAX_CELLS {
        if frontier.is_empty() {
            break;
        }
        let idx = rng.range(frontier.len() as i32) as usize;
        let (cx, cz) = frontier[idx];
        let branches = 1 + rng.range(3); // 1..3
        let mut grew = false;
        for _ in 0..branches {
            let (dx, dz) = DIRS[rng.range(4) as usize];
            let (nx, nz) = (cx + dx, cz + dz);
            if nx * nx + nz * nz > PLAN_RADIUS * PLAN_RADIUS || cells.contains_key(&(nx, nz)) {
                continue;
            }
            // Majoritairement ponts/couloirs ; les salles spéciales sont assignées après.
            let kind = if rng.range(2) == 0 {
                Kind::Bridge
            } else {
                Kind::Corridor
            };
            cells.insert((nx, nz), kind);
            order.push((nx, nz));
            frontier.push((nx, nz));
            grew = true;
            if cells.len() >= MAX_CELLS {
                break;
            }
        }
        if !grew {
            frontier.swap_remove(idx);
        }
    }

    // Salles spéciales : la cellule la plus éloignée = blaze, la 2e = wart, une proche = chest.
    if order.len() >= 4 {
        let mut by_dist = order.clone();
        by_dist.sort_by_key(|&(x, z)| -(x * x + z * z));
        if let Some(&c) = by_dist.first() {
            cells.insert(c, Kind::Blaze);
        }
        if let Some(&c) = by_dist.get(1) {
            cells.insert(c, Kind::Wart);
        }
        // Coffre : une cellule proche du centre (autre que l'origine).
        if let Some(&c) =
            order.get(1 + rng.range((order.len() as i32 - 1).max(1)) as usize % order.len())
        {
            if c != (0, 0) && cells.get(&c) == Some(&Kind::Bridge) {
                cells.insert(c, Kind::Chest);
            }
        }
    }
    cells
}

/// Carve les forteresses qui débordent dans ce chunk. Appelé après la décoration.
pub fn carve_into_chunk(grid: &mut DimGrid, base_x: i32, base_z: i32, seed: u64) {
    // Régions candidates : celle du chunk + 8 voisines (une forteresse peut déborder).
    let crx = base_x.div_euclid(SPACING * 16);
    let crz = base_z.div_euclid(SPACING * 16);
    for drx in -1..=1 {
        for drz in -1..=1 {
            let (ocx, ocz) = fortress_origin_chunk(crx + drx, crz + drz, seed);
            let (ox, oz) = (ocx * 16 + 8, ocz * 16 + 8); // centre monde de l'origine
                                                         // Cull : la forteresse s'étend de ±(PLAN_RADIUS+1)*CELL autour de l'origine.
            let reach = (PLAN_RADIUS + 1) * CELL;
            if (base_x - ox).abs() > reach + 16 || (base_z - oz).abs() > reach + 16 {
                continue;
            }
            carve_fortress(grid, base_x, base_z, ox, oz, seed);
        }
    }
}

/// Rend une forteresse (origine monde `ox,oz`) dans le chunk courant.
fn carve_fortress(grid: &mut DimGrid, base_x: i32, base_z: i32, ox: i32, oz: i32, seed: u64) {
    // Cellule d'origine = celle contenant (ox,oz).
    let origin_cx = ox.div_euclid(CELL);
    let origin_cz = oz.div_euclid(CELL);
    let cells = plan(origin_cx, origin_cz, seed);
    let occupied = |cx: i32, cz: i32| cells.contains_key(&(origin_cx + cx, origin_cz + cz));

    for (&(rcx, rcz), &kind) in &cells {
        // Coords monde du coin de la cellule.
        let wx0 = (origin_cx + rcx) * CELL;
        let wz0 = (origin_cz + rcz) * CELL;
        // Cull cellule vs chunk.
        if wx0 + CELL <= base_x || wx0 >= base_x + 16 || wz0 + CELL <= base_z || wz0 >= base_z + 16
        {
            continue;
        }
        let conn = [
            occupied(rcx + 1, rcz), // est  (x+)
            occupied(rcx - 1, rcz), // ouest(x-)
            occupied(rcx, rcz + 1), // sud  (z+)
            occupied(rcx, rcz - 1), // nord (z-)
        ];
        render_cell(grid, wx0, wz0, kind, conn);
    }
}

/// Rend une cellule 8×8 à `(wx0, wz0)`. `conn = [est, ouest, sud, nord]`.
fn render_cell(grid: &mut DimGrid, wx0: i32, wz0: i32, kind: Kind, conn: [bool; 4]) {
    let fb = &*FB;
    let (x1, z1) = (wx0 + CELL - 1, wz0 + CELL - 1); // bord inclus (7)
    let floor = BASE_Y - 1;
    let enclosed = !matches!(kind, Kind::Bridge | Kind::Chest);
    let ceiling = BASE_Y + 3;

    // Sol + piliers + intérieur dégagé.
    for x in wx0..=x1 {
        for z in wz0..=z1 {
            grid.set(x, floor, z, fb.brick);
            // Dégage l'air au-dessus du sol (sur la hauteur de la pièce).
            let top = if enclosed { ceiling - 1 } else { BASE_Y + 4 };
            for y in BASE_Y..=top {
                grid.set(x, y, z, fb.air);
            }
        }
    }
    // Piliers de support aux 4 coins (descendent sous le pont).
    for &(px, pz) in &[(wx0, wz0), (x1, wz0), (wx0, z1), (x1, z1)] {
        for y in floor - 6..floor {
            grid.set(px, y, pz, fb.brick);
        }
    }

    // Murs / rambardes le long des bords NON connectés.
    // Bord = est(x==x1), ouest(x==wx0), sud(z==z1), nord(z==wz0).
    let edge_open = |x: i32, z: i32| -> Option<usize> {
        if x == x1 {
            Some(0)
        } else if x == wx0 {
            Some(1)
        } else if z == z1 {
            Some(2)
        } else if z == wz0 {
            Some(3)
        } else {
            None
        }
    };
    for x in wx0..=x1 {
        for z in wz0..=z1 {
            if let Some(e) = edge_open(x, z) {
                if conn[e] {
                    continue; // bord connecté : on laisse l'ouverture
                }
                if enclosed {
                    // Mur plein du sol au plafond.
                    for y in BASE_Y..=ceiling {
                        grid.set(x, y, z, fb.brick);
                    }
                } else {
                    // Pont : rambarde de fence (2 de haut) + ligne de brick au sol.
                    grid.set(x, BASE_Y, z, fb.fence);
                    grid.set(x, BASE_Y + 1, z, fb.fence);
                }
            }
        }
    }
    if enclosed {
        // Plafond.
        for x in wx0..=x1 {
            for z in wz0..=z1 {
                grid.set(x, ceiling, z, fb.brick);
            }
        }
    }

    // Contenu spécifique.
    let (mx, mz) = (wx0 + CELL / 2, wz0 + CELL / 2); // centre
    match kind {
        Kind::Blaze => {
            // Petite plateforme surélevée + spawner + fence autour.
            for dx in -1..=1 {
                for dz in -1..=1 {
                    grid.set(mx + dx, BASE_Y, mz + dz, fb.brick);
                }
            }
            grid.set(mx, BASE_Y + 1, mz, fb.spawner);
            for &(dx, dz) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                grid.set(mx + dx, BASE_Y + 1, mz + dz, fb.fence);
            }
        }
        Kind::Wart => {
            // Rangées de soul sand + nether wart sur le sol.
            for x in (wx0 + 1)..=(x1 - 1) {
                for z in (wz0 + 1)..=(z1 - 1) {
                    if (x + z) % 2 == 0 {
                        grid.set(x, floor, z, fb.soul_sand);
                        grid.set(x, BASE_Y, z, fb.wart);
                    }
                }
            }
        }
        Kind::Chest => {
            grid.set(mx, BASE_Y, mz, fb.chest);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le plan d'une forteresse contient plusieurs cellules dont une blaze + une wart.
    #[test]
    fn plan_has_special_rooms() {
        let cells = plan(0, 0, 42);
        assert!(cells.len() >= 4, "forteresse non vide attendue");
        assert!(
            cells.values().any(|&k| k == Kind::Blaze),
            "salle blaze attendue"
        );
        assert!(
            cells.values().any(|&k| k == Kind::Wart),
            "salle wart attendue"
        );
    }

    /// Un chunk proche de l'origine reçoit bien de la nether brick + un spawner quelque part.
    #[test]
    fn carve_places_bricks_and_spawner() {
        let fb = &*FB;
        let seed = 42u64;
        // Trouve l'origine de la région (0,0).
        let (ocx, ocz) = fortress_origin_chunk(0, 0, seed);
        let mut bricks = 0usize;
        let mut spawners = 0usize;
        // Carve une grille de chunks autour de l'origine.
        for cx in (ocx - 5)..(ocx + 6) {
            for cz in (ocz - 5)..(ocz + 6) {
                let (bx, bz) = (cx * 16, cz * 16);
                let mut grid =
                    DimGrid::test_floor(bx, bz, 0, 128, BLOCKS.get("minecraft:netherrack"), 80);
                carve_into_chunk(&mut grid, bx, bz, seed);
                bricks += grid.count(fb.brick);
                spawners += grid.count(fb.spawner);
            }
        }
        assert!(
            bricks > 100,
            "nether brick de forteresse attendue (trouvé {bricks})"
        );
        assert!(spawners >= 1, "au moins un spawner de blaze attendu");
    }
}
