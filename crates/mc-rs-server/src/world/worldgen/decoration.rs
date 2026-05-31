//! Décoration riche du générateur noise — objectif : proche de Bedrock Edition.
//!
//! Placement par biome (noms Java) directement sur la grille de blocs :
//! - **arbres** : chaque biome reçoit sa **composition vanilla officielle**
//!   (sélecteur `random_selector` : espèce par défaut + alternatives à chances
//!   exactes, extraites des `configured_feature` Java). Espèces : chêne, **fancy
//!   oak** (gros chêne touffu à branches), bouleau / super bouleau, sapin, pin,
//!   méga conifère 2×2, jungle / buisson / méga jungle 2×2, chêne noir 2×2,
//!   acacia, cerisier, palétuvier. + **lianes** (jungle/palétuvier),
//! - herbe haute / fougères / fleurs (densités basées sur les `patch_*`),
//! - **aquatique** : kelp, seagrass, récifs de **corail** + sea pickles.
//!
//! Densités d'arbres = moyenne du modificateur `count` des `placed_feature`
//! vanilla (ex. plaines 0.05, jungle 50, mangrove 25). Remplace `vegetation`
//! (legacy). Les formes d'arbres restent des approximations fidèles des trunk/
//! foliage placers vanilla. Arbres en bord de chunk rognés (pas de débordement).

use super::super::block_registry::BLOCKS;
use super::super::random::Random;
use super::noise_chunk::{grid_index, MAX_Y, MIN_Y, SEA_LEVEL};

/// Palette d'IDs runtime résolus une fois.
struct Pal {
    air: u32,
    water: u32,
    grass_block: u32,
    dirt: u32,
    sand: u32,
    logs: [u32; 8],
    leaves: [u32; 8],
    vine: u32,
    kelp: u32,
    seagrass: u32,
    sea_pickle: u32,
    corals: [u32; 5],
    short_grass: u32,
    fern: u32,
    flowers: Vec<u32>,
    cactus: u32,
    deadbush: u32,
    bamboo: u32,
    sugar_cane: u32,
    lily_pad: u32,
    cherry_log: u32,
    cherry_leaves: u32,
    mangrove_log: u32,
    mangrove_leaves: u32,
    mangrove_roots: u32,
}

// Indices d'espèce dans logs/leaves.
const OAK: usize = 0;
const BIRCH: usize = 1;
const SPRUCE: usize = 2;
const JUNGLE: usize = 3;
const ACACIA: usize = 4;
const DARK_OAK: usize = 5;

impl Pal {
    fn new() -> Self {
        let g = |n: &str| BLOCKS.get(n);
        Pal {
            air: BLOCKS.air,
            water: BLOCKS.water,
            grass_block: BLOCKS.grass_block,
            dirt: BLOCKS.dirt,
            sand: BLOCKS.sand,
            logs: [
                g("minecraft:oak_log"),
                g("minecraft:birch_log"),
                g("minecraft:spruce_log"),
                g("minecraft:jungle_log"),
                g("minecraft:acacia_log"),
                g("minecraft:dark_oak_log"),
                g("minecraft:oak_log"),
                g("minecraft:oak_log"),
            ],
            leaves: [
                g("minecraft:oak_leaves"),
                g("minecraft:birch_leaves"),
                g("minecraft:spruce_leaves"),
                g("minecraft:jungle_leaves"),
                g("minecraft:acacia_leaves"),
                g("minecraft:dark_oak_leaves"),
                g("minecraft:oak_leaves"),
                g("minecraft:oak_leaves"),
            ],
            vine: g("minecraft:vine"),
            kelp: g("minecraft:kelp"),
            seagrass: g("minecraft:seagrass"),
            sea_pickle: g("minecraft:sea_pickle"),
            corals: [
                g("minecraft:tube_coral_block"),
                g("minecraft:brain_coral_block"),
                g("minecraft:bubble_coral_block"),
                g("minecraft:fire_coral_block"),
                g("minecraft:horn_coral_block"),
            ],
            short_grass: g("minecraft:short_grass"),
            fern: g("minecraft:fern"),
            flowers: vec![
                g("minecraft:dandelion"),
                g("minecraft:poppy"),
                g("minecraft:cornflower"),
                g("minecraft:oxeye_daisy"),
                g("minecraft:allium"),
                g("minecraft:azure_bluet"),
            ],
            cactus: g("minecraft:cactus"),
            deadbush: g("minecraft:deadbush"),
            bamboo: g("minecraft:bamboo"),
            sugar_cane: g("minecraft:reeds"),
            lily_pad: g("minecraft:waterlily"),
            cherry_log: g("minecraft:cherry_log"),
            cherry_leaves: g("minecraft:cherry_leaves"),
            mangrove_log: g("minecraft:mangrove_log"),
            mangrove_leaves: g("minecraft:mangrove_leaves"),
            mangrove_roots: g("minecraft:mangrove_roots"),
        }
    }
}

#[inline]
fn idx_ok(lx: i32, wy: i32, lz: i32) -> Option<usize> {
    if (0..16).contains(&lx) && (0..16).contains(&lz) && (MIN_Y..MAX_Y).contains(&wy) {
        Some(grid_index(lx as usize, wy, lz as usize))
    } else {
        None
    }
}

#[inline]
fn at(grid: &[u32], lx: i32, wy: i32, lz: i32) -> u32 {
    idx_ok(lx, wy, lz).map(|i| grid[i]).unwrap_or(0)
}

/// Pose une feuille (ne remplace que l'air).
fn leaf(grid: &mut [u32], pal: &Pal, lx: i32, wy: i32, lz: i32, id: u32) {
    if let Some(i) = idx_ok(lx, wy, lz) {
        if grid[i] == pal.air {
            grid[i] = id;
        }
    }
}

/// Pose un tronc (remplace air et feuilles).
fn log(grid: &mut [u32], pal: &Pal, lx: i32, wy: i32, lz: i32, id: u32, leaf_id: u32) {
    if let Some(i) = idx_ok(lx, wy, lz) {
        if grid[i] == pal.air || grid[i] == leaf_id {
            grid[i] = id;
        }
    }
}

/// Pose une plante au sol si la case est libre.
fn plant(grid: &mut [u32], pal: &Pal, lx: i32, wy: i32, lz: i32, id: u32) {
    if let Some(i) = idx_ok(lx, wy, lz) {
        if grid[i] == pal.air {
            grid[i] = id;
        }
    }
}

/// Lianes pendantes sur les côtés exposés d'un bloc de feuilles.
fn hang_vines(grid: &mut [u32], pal: &Pal, lx: i32, wy: i32, lz: i32, rng: &mut Random) {
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if rng.next_bounded_int(3) != 0 {
            continue;
        }
        let (vx, vz) = (lx + dx, lz + dz);
        if at(grid, vx, wy, vz) != pal.air {
            continue;
        }
        let len = 1 + rng.next_bounded_int(5);
        for d in 0..len {
            let y = wy - d;
            if at(grid, vx, y, vz) == pal.air {
                if let Some(i) = idx_ok(vx, y, vz) {
                    grid[i] = pal.vine;
                }
            } else {
                break;
            }
        }
    }
}

/// Couronne de feuillage en blob (chêne/bouleau/jungle).
fn blob_leaves(
    grid: &mut [u32],
    pal: &Pal,
    lx: i32,
    top_y: i32,
    lz: i32,
    leaf_id: u32,
    rng: &mut Random,
) {
    for dy in -3i32..=0 {
        let y = top_y + dy;
        let r: i32 = if dy >= -1 { 1 } else { 2 };
        for ox in -r..=r {
            for oz in -r..=r {
                // Coins arrondis sur les couches larges.
                if ox.abs() == r && oz.abs() == r && (dy < -1) && rng.next_bounded_int(2) == 0 {
                    continue;
                }
                leaf(grid, pal, lx + ox, y, lz + oz, leaf_id);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn straight_tree(
    grid: &mut [u32],
    pal: &Pal,
    lx: i32,
    ground: i32,
    lz: i32,
    species: usize,
    min_h: i32,
    extra: i32,
    vines: bool,
    rng: &mut Random,
) {
    let log_id = pal.logs[species];
    let leaf_id = pal.leaves[species];
    let h = min_h + rng.next_bounded_int(extra.max(1));
    for dy in 0..h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    let top = ground + 1 + h;
    blob_leaves(grid, pal, lx, top, lz, leaf_id, rng);
    if vines {
        for dy in -3i32..=0 {
            hang_vines(grid, pal, lx, top + dy, lz, rng);
        }
    }
}

/// Sapin conique.
fn spruce_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.logs[SPRUCE];
    let leaf_id = pal.leaves[SPRUCE];
    let h = 6 + rng.next_bounded_int(8);
    for dy in 0..=h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    // Feuillage : rayon qui croît vers le bas par paliers, pointe en haut.
    let leaf_bottom = ground + 1 + (h / 3) + rng.next_bounded_int(2);
    let top = ground + 1 + h + 1;
    let mut r = 0i32;
    let mut step = 0;
    for y in (leaf_bottom..=top).rev() {
        for ox in -r..=r {
            for oz in -r..=r {
                if ox.abs() + oz.abs() <= r + 1 {
                    leaf(grid, pal, lx + ox, y, lz + oz, leaf_id);
                }
            }
        }
        step += 1;
        if step % 2 == 0 {
            r += 1;
        }
        if r > 2 {
            r = 1;
        }
    }
    leaf(grid, pal, lx, top, lz, leaf_id);
}

/// Acacia : tronc + segment oblique + disque plat de feuilles.
fn acacia_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.logs[ACACIA];
    let leaf_id = pal.leaves[ACACIA];
    let h = 4 + rng.next_bounded_int(3);
    for dy in 0..h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    let dir = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.next_bounded_int(4) as usize];
    let (mut cx, mut cz) = (lx, lz);
    let mut cy = ground + 1 + h;
    for _ in 0..2 {
        cx += dir.0;
        cz += dir.1;
        cy += 1;
        log(grid, pal, cx, cy, cz, log_id, leaf_id);
    }
    // Disque plat.
    for ox in -3..=3 {
        for oz in -3..=3 {
            if ox * ox + oz * oz <= 9 {
                leaf(grid, pal, cx + ox, cy + 1, cz + oz, leaf_id);
                leaf(grid, pal, cx + ox, cy, cz + oz, leaf_id);
            }
        }
    }
}

/// Tronc 2×2 (chêne noir / jungle géante) + grande couronne.
#[allow(clippy::too_many_arguments)]
fn mega_tree(
    grid: &mut [u32],
    pal: &Pal,
    lx: i32,
    ground: i32,
    lz: i32,
    species: usize,
    min_h: i32,
    extra: i32,
    vines: bool,
    rng: &mut Random,
) {
    let log_id = pal.logs[species];
    let leaf_id = pal.leaves[species];
    let h = min_h + rng.next_bounded_int(extra.max(1));
    for dy in 0..h {
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            log(
                grid,
                pal,
                lx + ox,
                ground + 1 + dy,
                lz + oz,
                log_id,
                leaf_id,
            );
        }
    }
    let top = ground + 1 + h;
    // Plusieurs couches de feuilles larges.
    for (dy, r) in [(1, 1), (0, 2), (-1, 3), (-2, 2)] {
        let y = top + dy;
        for ox in -r..=(r + 1) {
            for oz in -r..=(r + 1) {
                if ox * ox + oz * oz <= (r + 1) * (r + 1) {
                    leaf(grid, pal, lx + ox, y, lz + oz, leaf_id);
                }
            }
        }
    }
    if vines {
        for dy in -2i32..=1 {
            for (ox, oz) in [(-2, 0), (3, 0), (0, -2), (0, 3)] {
                hang_vines(grid, pal, lx + ox, top + dy, lz + oz, rng);
            }
        }
    }
}

/// Grappe sphérique de feuilles (centrée), un peu aplatie verticalement.
fn foliage_cluster(grid: &mut [u32], pal: &Pal, cx: i32, cy: i32, cz: i32, r: i32, leaf_id: u32) {
    for ox in -r..=r {
        for oy in -r..=r {
            for oz in -r..=r {
                // Sphère légèrement aplatie (le Y compte double).
                if ox * ox + oy * oy * 2 + oz * oz <= r * r + 1 {
                    leaf(grid, pal, cx + ox, cy + oy, cz + oz, leaf_id);
                }
            }
        }
    }
}

/// **Fancy oak / grand chêne** : tronc haut + branches obliques, chacune
/// terminée par une grappe de feuillage → grosse couronne touffue multi-lobes.
fn fancy_oak_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.logs[OAK];
    let leaf_id = pal.leaves[OAK];
    let h = 8 + rng.next_bounded_int(5); // 8..12
    for dy in 0..=h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    // Couronne sommitale.
    foliage_cluster(grid, pal, lx, ground + 1 + h, lz, 2, leaf_id);
    // Branches dans la moitié haute, chacune avec sa grappe.
    let branches = 3 + rng.next_bounded_int(3); // 3..5
    for _ in 0..branches {
        let start = ground + 1 + h / 2 + rng.next_bounded_int((h / 2).max(1));
        let (dx, dz) = loop {
            let dx = rng.next_range(-1, 1);
            let dz = rng.next_range(-1, 1);
            if dx != 0 || dz != 0 {
                break (dx, dz);
            }
        };
        let len = 2 + rng.next_bounded_int(2); // 2..3
        let (mut bx, mut by, mut bz) = (lx, start, lz);
        for _ in 0..len {
            bx += dx;
            bz += dz;
            by += 1;
            log(grid, pal, bx, by, bz, log_id, leaf_id);
        }
        foliage_cluster(grid, pal, bx, by, bz, 2, leaf_id);
    }
}

/// **Pin** : grand tronc nu, petite touffe pointue au sommet.
fn pine_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.logs[SPRUCE];
    let leaf_id = pal.leaves[SPRUCE];
    let h = 7 + rng.next_bounded_int(6); // 7..12
    for dy in 0..=h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    // Touffe sur les ~4 derniers blocs : rayon 2,2,1,1 puis pointe.
    let top = ground + 1 + h;
    for (i, &r) in [2i32, 2, 1, 1].iter().enumerate() {
        let y = top - 3 + i as i32;
        for ox in -r..=r {
            for oz in -r..=r {
                if ox.abs() + oz.abs() <= r {
                    leaf(grid, pal, lx + ox, y, lz + oz, leaf_id);
                }
            }
        }
    }
    leaf(grid, pal, lx, top + 1, lz, leaf_id);
}

/// **Buisson de jungle** : 1 bûche + petite grappe de feuilles.
fn jungle_bush(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32) {
    log(
        grid,
        pal,
        lx,
        ground + 1,
        lz,
        pal.logs[JUNGLE],
        pal.leaves[JUNGLE],
    );
    foliage_cluster(grid, pal, lx, ground + 2, lz, 2, pal.leaves[JUNGLE]);
}

/// **Méga conifère 2×2** (méga sapin/pin) : tronc épais très haut + feuillage
/// conique en paliers.
fn mega_conifer(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.logs[SPRUCE];
    let leaf_id = pal.leaves[SPRUCE];
    let h = 13 + rng.next_bounded_int(8); // 13..20
    for dy in 0..h {
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            log(
                grid,
                pal,
                lx + ox,
                ground + 1 + dy,
                lz + oz,
                log_id,
                leaf_id,
            );
        }
    }
    // Feuillage conique : large en bas, pointe en haut.
    let leaf_bottom = ground + 1 + h / 3;
    let top = ground + 1 + h + 1;
    let mut r = 1i32;
    let mut step = 0;
    for y in (leaf_bottom..=top).rev() {
        for ox in -r..=(r + 1) {
            for oz in -r..=(r + 1) {
                if ox * ox + oz * oz <= (r + 1) * (r + 1) {
                    leaf(grid, pal, lx + ox, y, lz + oz, leaf_id);
                }
            }
        }
        step += 1;
        if step % 2 == 0 {
            r += 1;
        }
        if r > 3 {
            r = 1;
        }
    }
}

/// **Cerisier** : petit tronc + branches + grappes de feuilles roses.
fn cherry_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    let log_id = pal.cherry_log;
    let leaf_id = pal.cherry_leaves;
    let h = 5 + rng.next_bounded_int(3);
    for dy in 0..=h {
        log(grid, pal, lx, ground + 1 + dy, lz, log_id, leaf_id);
    }
    foliage_cluster(grid, pal, lx, ground + 1 + h, lz, 3, leaf_id);
    for _ in 0..2 {
        let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.next_bounded_int(4) as usize];
        let by = ground + 1 + h - 1 - rng.next_bounded_int(2);
        foliage_cluster(grid, pal, lx + dx * 2, by + 1, lz + dz * 2, 2, leaf_id);
        log(grid, pal, lx + dx, by, lz + dz, log_id, leaf_id);
    }
}

/// **Palétuvier** : racines au sol, tronc, feuillage + lianes.
fn mangrove_tree(grid: &mut [u32], pal: &Pal, lx: i32, ground: i32, lz: i32, rng: &mut Random) {
    // Racines.
    for (ox, oz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
        if let Some(i) = idx_ok(lx + ox, ground, lz + oz) {
            if grid[i] == pal.dirt || grid[i] == pal.grass_block {
                grid[i] = pal.mangrove_roots;
            }
        }
    }
    let h = 4 + rng.next_bounded_int(4);
    for dy in 0..=h {
        log(
            grid,
            pal,
            lx,
            ground + 1 + dy,
            lz,
            pal.mangrove_log,
            pal.mangrove_leaves,
        );
    }
    let top = ground + 1 + h;
    foliage_cluster(grid, pal, lx, top, lz, 3, pal.mangrove_leaves);
    for dy in -2i32..=0 {
        hang_vines(grid, pal, lx + 2, top + dy, lz, rng);
        hang_vines(grid, pal, lx - 2, top + dy, lz, rng);
        hang_vines(grid, pal, lx, top + dy, lz + 2, rng);
    }
}

#[derive(Clone, Copy)]
enum Species {
    Oak,
    FancyOak,
    Birch,
    SuperBirch,
    Spruce,
    Pine,
    MegaSpruce,
    JungleTree,
    JungleBush,
    MegaJungle,
    DarkOak,
    Acacia,
    Cherry,
    Mangrove,
}

/// Composition d'arbres d'un biome : `(densité arbres/chunk, espèce par défaut,
/// alternatives (chance, espèce))`. **Données officielles vanilla** : densité =
/// moyenne du `count` des `placed_feature` ; espèces/chances = `random_selector`
/// des `configured_feature` (`trees_<biome>` → sélecteur `default` + features).
fn tree_plan(biome: &str) -> (f64, Species, &'static [(f64, Species)]) {
    use Species::*;
    match biome {
        // Forêt : chêne (déf.) + 20 % bouleau + 10 % fancy oak.
        "minecraft:forest" => (10.0, Oak, &[(0.20, Birch), (0.10, FancyOak)]),
        "minecraft:flower_forest" => (6.0, Oak, &[(0.20, Birch), (0.10, FancyOak)]),
        "minecraft:birch_forest" => (10.0, Birch, &[]),
        "minecraft:old_growth_birch_forest" => (10.0, SuperBirch, &[]),
        "minecraft:dark_forest" => (16.0, DarkOak, &[(0.05, Birch), (0.05, Oak)]),
        // Taïga : sapin (déf.) + 33 % pin.
        "minecraft:taiga" | "minecraft:snowy_taiga" => (10.0, Spruce, &[(0.333, Pine)]),
        "minecraft:old_growth_pine_taiga" => (
            10.0,
            Spruce,
            &[(0.026, MegaSpruce), (0.308, MegaSpruce), (0.333, Pine)],
        ),
        "minecraft:old_growth_spruce_taiga" => {
            (10.0, Spruce, &[(0.333, MegaSpruce), (0.333, Pine)])
        }
        "minecraft:grove" => (10.0, Spruce, &[(0.333, Pine)]),
        "minecraft:snowy_slopes" => (0.1, Spruce, &[(0.333, Pine)]),
        // Jungle : arbre (déf.) + 10 % fancy oak + 50 % buisson + 33 % méga.
        "minecraft:jungle" => (
            50.0,
            JungleTree,
            &[(0.10, FancyOak), (0.50, JungleBush), (0.333, MegaJungle)],
        ),
        "minecraft:bamboo_jungle" => (
            30.0,
            JungleTree,
            &[(0.10, FancyOak), (0.50, JungleBush), (0.333, MegaJungle)],
        ),
        "minecraft:sparse_jungle" => (2.0, JungleTree, &[(0.10, FancyOak), (0.50, JungleBush)]),
        // Savane : chêne (déf.) + 80 % acacia.
        "minecraft:savanna" | "minecraft:savanna_plateau" | "minecraft:windswept_savanna" => {
            (1.1, Oak, &[(0.80, Acacia)])
        }
        "minecraft:swamp" => (0.1, Oak, &[(0.10, FancyOak)]),
        "minecraft:mangrove_swamp" => (25.0, Mangrove, &[]),
        // Plaines : chêne (déf.) + 1/3 fancy oak (parmi ses très rares arbres).
        "minecraft:plains" | "minecraft:sunflower_plains" => (0.05, Oak, &[(0.333, FancyOak)]),
        "minecraft:meadow" => (0.05, SuperBirch, &[(0.50, FancyOak)]),
        // Windswept : chêne (déf.) + 66 % sapin + 10 % fancy oak.
        "minecraft:windswept_forest" => (8.0, Oak, &[(0.666, Spruce), (0.10, FancyOak)]),
        "minecraft:windswept_hills" | "minecraft:windswept_gravelly_hills" => {
            (0.1, Oak, &[(0.666, Spruce), (0.10, FancyOak)])
        }
        "minecraft:cherry_grove" => (10.0, Cherry, &[]),
        _ => (0.0, Oak, &[]),
    }
}

/// Sélection d'espèce selon la sémantique vanilla `random_selector` : on teste
/// chaque alternative dans l'ordre (`rng < chance`), sinon l'espèce par défaut.
fn pick_tree(default: Species, alts: &[(f64, Species)], rng: &mut Random) -> Species {
    for (chance, species) in alts {
        if rng.next_float() < *chance {
            return *species;
        }
    }
    default
}

fn place_tree(
    grid: &mut [u32],
    pal: &Pal,
    species: Species,
    lx: i32,
    ground: i32,
    lz: i32,
    rng: &mut Random,
) {
    match species {
        Species::Oak => straight_tree(grid, pal, lx, ground, lz, OAK, 4, 3, false, rng),
        Species::FancyOak => fancy_oak_tree(grid, pal, lx, ground, lz, rng),
        Species::Birch => straight_tree(grid, pal, lx, ground, lz, BIRCH, 5, 3, false, rng),
        Species::SuperBirch => straight_tree(grid, pal, lx, ground, lz, BIRCH, 8, 4, false, rng),
        Species::Spruce => spruce_tree(grid, pal, lx, ground, lz, rng),
        Species::Pine => pine_tree(grid, pal, lx, ground, lz, rng),
        Species::MegaSpruce => mega_conifer(grid, pal, lx, ground, lz, rng),
        // Lianes gérées par la feature `vines` (decorate_vines), pas ici.
        Species::JungleTree => straight_tree(grid, pal, lx, ground, lz, JUNGLE, 4, 8, false, rng),
        Species::JungleBush => jungle_bush(grid, pal, lx, ground, lz),
        Species::MegaJungle => mega_tree(grid, pal, lx, ground, lz, JUNGLE, 10, 11, true, rng),
        Species::DarkOak => mega_tree(grid, pal, lx, ground, lz, DARK_OAK, 6, 3, false, rng),
        Species::Acacia => acacia_tree(grid, pal, lx, ground, lz, rng),
        Species::Cherry => cherry_tree(grid, pal, lx, ground, lz, rng),
        Species::Mangrove => mangrove_tree(grid, pal, lx, ground, lz, rng),
    }
}

/// Densité d'herbe/fleurs (tentatives par chunk).
///
/// Basée sur les `patch_grass_*` vanilla (jungle `count 25`, forêt `count 2`).
/// En vanilla les plaines/savanes utilisent `noise_threshold_count` (densité
/// variable selon un bruit) — approximé ici par une valeur fixe modérée.
fn grass_density(biome: &str) -> i32 {
    match biome {
        "minecraft:jungle" | "minecraft:bamboo_jungle" => 25,
        "minecraft:sparse_jungle" => 12,
        "minecraft:plains" | "minecraft:sunflower_plains" | "minecraft:meadow" => 10,
        "minecraft:savanna" | "minecraft:savanna_plateau" | "minecraft:windswept_savanna" => 8,
        "minecraft:forest" | "minecraft:flower_forest" => 2,
        "minecraft:taiga" | "minecraft:snowy_taiga" | "minecraft:grove" => 4,
        "minecraft:swamp" | "minecraft:mangrove_swamp" => 5,
        "minecraft:birch_forest" | "minecraft:old_growth_birch_forest" => 3,
        "minecraft:dark_forest" => 2,
        _ => 1,
    }
}

fn is_ocean(biome: &str) -> bool {
    matches!(
        biome,
        "minecraft:ocean"
            | "minecraft:deep_ocean"
            | "minecraft:cold_ocean"
            | "minecraft:deep_cold_ocean"
            | "minecraft:lukewarm_ocean"
            | "minecraft:deep_lukewarm_ocean"
            | "minecraft:warm_ocean"
            | "minecraft:frozen_ocean"
            | "minecraft:deep_frozen_ocean"
    )
}

fn is_jungle(biome: &str) -> bool {
    matches!(
        biome,
        "minecraft:jungle" | "minecraft:bamboo_jungle" | "minecraft:sparse_jungle"
    )
}

#[inline]
fn is_vine_anchor(pal: &Pal, block: u32) -> bool {
    // Face solide d'accroche pour une liane (pas air/eau/liane/plante fine).
    block != pal.air
        && block != pal.water
        && block != pal.vine
        && block != pal.short_grass
        && block != pal.fern
        && block != pal.kelp
        && block != pal.seagrass
}

/// Feature vanilla `vines` (jungle) : **count 127**, y ∈ [64, 100]. À une
/// position d'air adjacente à une face solide, accroche une liane et la fait
/// pendre tant qu'il reste de l'air le long de la même face.
fn decorate_vines(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    rng: &mut Random,
) {
    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };
    for _ in 0..127 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        if !is_jungle(biome_at(lx as usize, lz as usize)) {
            continue;
        }
        let wy = 64 + rng.next_bounded_int(37); // 64..=100
        if at(grid, lx, wy, lz) != pal.air {
            continue;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if !is_vine_anchor(pal, at(grid, lx + dx, wy, lz + dz)) {
                continue;
            }
            let len = 1 + rng.next_bounded_int(7); // 1..=7
            for d in 0..len {
                let y = wy - d;
                if at(grid, lx, y, lz) != pal.air
                    || !is_vine_anchor(pal, at(grid, lx + dx, y, lz + dz))
                {
                    break;
                }
                if let Some(i) = idx_ok(lx, y, lz) {
                    grid[i] = pal.vine;
                }
            }
            break;
        }
    }
}

/// Point d'entrée : décore un chunk déjà terrassé + habillé en surface.
pub fn decorate(
    grid: &mut [u32],
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
) {
    let pal = Pal::new();
    let mut rng = Random::new(
        0x9e37_79b9_i64
            .wrapping_mul(chunk_x as i64)
            .wrapping_add((chunk_z as i64) << 16)
            ^ seed as i64,
    );

    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };

    // ── Arbres ── (densité moyenne du chunk = moyenne des densités officielles
    // par colonne ; arrondi probabiliste pour les biomes clairsemés < 1/chunk).
    let tree_attempts: i32 = {
        let sum: f64 = (0..16)
            .flat_map(|x| (0..16).map(move |z| (x, z)))
            .map(|(x, z)| tree_plan(biome_at(x, z)).0)
            .sum();
        let mean = sum / 256.0;
        mean.floor() as i32
            + if rng.next_float() < mean.fract() {
                1
            } else {
                0
            }
    };
    for _ in 0..tree_attempts {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        if ground <= SEA_LEVEL {
            continue;
        }
        let below = at(grid, lx, ground, lz);
        if below != pal.grass_block && below != pal.dirt {
            continue;
        }
        if at(grid, lx, ground + 1, lz) != pal.air {
            continue;
        }
        let (density, default, alts) = tree_plan(biome_at(lx as usize, lz as usize));
        if density <= 0.0 {
            continue;
        }
        // Le palétuvier pousse aussi sur les racines déjà posées.
        let species = pick_tree(default, alts, &mut rng);
        place_tree(grid, &pal, species, lx, ground, lz, &mut rng);
    }

    // ── Lianes (après les arbres) : feature vanilla `vines`, count 127 ──
    decorate_vines(grid, &pal, biome_idx, biome_names, &mut rng);

    // ── Herbe / fougères / fleurs ──
    let grass_attempts: i32 = {
        let sum: i32 = (0..16)
            .flat_map(|x| (0..16).map(move |z| (x, z)))
            .map(|(x, z)| grass_density(biome_at(x, z)))
            .sum();
        sum / 16
    };
    for _ in 0..grass_attempts {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        if ground <= SEA_LEVEL || at(grid, lx, ground, lz) != pal.grass_block {
            continue;
        }
        let b = biome_at(lx as usize, lz as usize);
        let roll = rng.next_bounded_int(10);
        let id = if roll == 0 && !pal.flowers.is_empty() {
            pal.flowers[rng.next_bounded_int(pal.flowers.len() as i32) as usize]
        } else if b.contains("taiga") || b.contains("grove") {
            pal.fern
        } else {
            pal.short_grass
        };
        plant(grid, &pal, lx, ground + 1, lz, id);
    }

    // ── Spécial : cactus / arbustes morts / canne à sucre / bambou / nénuphars ──
    decorate_special(grid, &pal, biome_idx, biome_names, surfaces, &mut rng);

    // ── Aquatique : kelp / seagrass / coraux ──
    decorate_aquatic(grid, &pal, biome_idx, biome_names, surfaces, &mut rng);
}

fn decorate_special(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
    rng: &mut Random,
) {
    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };
    for _ in 0..6 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        if ground <= SEA_LEVEL {
            continue;
        }
        let b = biome_at(lx as usize, lz as usize);
        let surf = at(grid, lx, ground, lz);
        if (b.contains("desert") || b == "minecraft:badlands") && surf == pal.sand {
            let h = 1 + rng.next_bounded_int(3);
            for dy in 0..h {
                plant(grid, pal, lx, ground + 1 + dy, lz, pal.cactus);
            }
        } else if (b == "minecraft:bamboo_jungle" || b == "minecraft:jungle")
            && surf == pal.grass_block
            && rng.next_bounded_int(2) == 0
        {
            let h = 6 + rng.next_bounded_int(10);
            for dy in 0..h {
                plant(grid, pal, lx, ground + 1 + dy, lz, pal.bamboo);
            }
        } else if surf == pal.sand && b.contains("desert") {
            plant(grid, pal, lx, ground + 1, lz, pal.deadbush);
        }
    }

    // Canne à sucre au bord de l'eau.
    for _ in 0..4 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        if !(SEA_LEVEL - 1..=SEA_LEVEL + 2).contains(&ground) {
            continue;
        }
        let surf = at(grid, lx, ground, lz);
        if surf != pal.grass_block && surf != pal.sand && surf != pal.dirt {
            continue;
        }
        // À côté d'eau ?
        let near_water = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dz)| at(grid, lx + dx, ground, lz + dz) == pal.water);
        if !near_water {
            continue;
        }
        let h = 1 + rng.next_bounded_int(3);
        for dy in 0..h {
            plant(grid, pal, lx, ground + 1 + dy, lz, pal.sugar_cane);
        }
    }

    // Nénuphars en marais.
    for _ in 0..4 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        let b = biome_at(lx as usize, lz as usize);
        if !b.contains("swamp") {
            continue;
        }
        if at(grid, lx, SEA_LEVEL, lz) == pal.water && at(grid, lx, SEA_LEVEL + 1, lz) == pal.air {
            plant(grid, pal, lx, SEA_LEVEL + 1, lz, pal.lily_pad);
        }
        let _ = ground;
    }
}

fn decorate_aquatic(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
    rng: &mut Random,
) {
    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            let floor = surfaces[lx as usize][lz as usize];
            // Colonne sous-marine : sol solide sous le niveau de la mer + eau dessus.
            if floor >= SEA_LEVEL || at(grid, lx, floor + 1, lz) != pal.water {
                continue;
            }
            let b = biome_at(lx as usize, lz as usize);
            if !is_ocean(b) && !b.contains("river") {
                continue;
            }

            if b == "minecraft:warm_ocean" {
                // Récif de corail + sea pickles (océan chaud).
                if rng.next_bounded_int(4) == 0 {
                    let coral = pal.corals[rng.next_bounded_int(5) as usize];
                    let h = 1 + rng.next_bounded_int(3);
                    for dy in 0..h {
                        if at(grid, lx, floor + 1 + dy, lz) == pal.water {
                            if let Some(i) = idx_ok(lx, floor + 1 + dy, lz) {
                                grid[i] = coral;
                            }
                        }
                    }
                    if rng.next_bounded_int(2) == 0 {
                        plant(grid, pal, lx, floor + 1 + h, lz, pal.sea_pickle);
                    }
                    continue;
                }
            }

            // Kelp (océans non gelés) : colonne vers la surface.
            if !b.contains("frozen") && rng.next_bounded_int(6) == 0 {
                let max = SEA_LEVEL - 1;
                let height = 3 + rng.next_bounded_int((max - floor).max(1));
                for dy in 0..height {
                    let y = floor + 1 + dy;
                    if y >= max || at(grid, lx, y, lz) != pal.water {
                        break;
                    }
                    if let Some(i) = idx_ok(lx, y, lz) {
                        grid[i] = pal.kelp;
                    }
                }
            } else if rng.next_bounded_int(4) == 0 {
                // Seagrass sur le sol.
                if let Some(i) = idx_ok(lx, floor + 1, lz) {
                    if grid[i] == pal.water {
                        grid[i] = pal.seagrass;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise_chunk::GRID_LEN;
    use super::*;

    #[allow(clippy::type_complexity)]
    fn flat_chunk(ground: i32, biome: u16) -> (Box<[u32]>, [[u16; 16]; 16], [[i32; 16]; 16]) {
        let pal = Pal::new();
        let mut grid = vec![pal.air; GRID_LEN].into_boxed_slice();
        for lx in 0..16usize {
            for lz in 0..16usize {
                for wy in MIN_Y..ground {
                    grid[grid_index(lx, wy, lz)] = pal.dirt;
                }
                grid[grid_index(lx, ground, lz)] = pal.grass_block;
            }
        }
        (grid, [[biome; 16]; 16], [[ground; 16]; 16])
    }

    #[test]
    fn forest_places_logs_and_leaves() {
        let names = vec!["minecraft:forest".to_string()];
        let (mut grid, idx, surf) = flat_chunk(70, 0);
        decorate(&mut grid, 42, 0, 0, &idx, &names, &surf);
        let pal = Pal::new();
        let logs = grid.iter().filter(|&&b| pal.logs.contains(&b)).count();
        let leaves = grid.iter().filter(|&&b| pal.leaves.contains(&b)).count();
        assert!(logs > 0, "forêt sans tronc");
        assert!(leaves > 0, "forêt sans feuilles");
    }

    #[test]
    fn fancy_oak_has_more_foliage_than_oak() {
        let pal = Pal::new();
        let count_leaves = |species: Species| -> usize {
            let mut grid = vec![pal.air; GRID_LEN].into_boxed_slice();
            let mut rng = Random::new(123);
            place_tree(&mut grid, &pal, species, 8, 70, 8, &mut rng);
            grid.iter().filter(|&&b| b == pal.leaves[OAK]).count()
        };
        let oak = count_leaves(Species::Oak);
        let fancy = count_leaves(Species::FancyOak);
        assert!(
            fancy > oak * 2,
            "le fancy oak doit avoir bien plus de feuillage (oak={oak}, fancy={fancy})"
        );
    }

    #[test]
    fn cherry_and_mangrove_use_own_blocks() {
        let pal = Pal::new();
        let mut grid = vec![pal.air; GRID_LEN].into_boxed_slice();
        for lx in 0..16usize {
            for lz in 0..16usize {
                grid[grid_index(lx, 69, lz)] = pal.dirt;
                grid[grid_index(lx, 70, lz)] = pal.grass_block;
            }
        }
        let mut rng = Random::new(5);
        place_tree(&mut grid, &pal, Species::Cherry, 4, 70, 4, &mut rng);
        place_tree(&mut grid, &pal, Species::Mangrove, 11, 70, 11, &mut rng);
        assert!(
            grid.contains(&pal.cherry_leaves),
            "pas de feuilles de cerisier"
        );
        assert!(
            grid.contains(&pal.mangrove_log),
            "pas de bûche de palétuvier"
        );
    }

    #[test]
    fn jungle_places_many_vines() {
        // Jungle dense → la feature vines (127) doit couvrir les arbres.
        let names = vec!["minecraft:jungle".to_string()];
        let (mut grid, idx, surf) = flat_chunk(75, 0);
        decorate(&mut grid, 99, 0, 0, &idx, &names, &surf);
        let pal = Pal::new();
        let vines = grid.iter().filter(|&&b| b == pal.vine).count();
        assert!(vines > 20, "jungle trop pauvre en lianes: {vines}");
    }

    #[test]
    fn warm_ocean_places_coral() {
        let names = vec!["minecraft:warm_ocean".to_string()];
        let pal = Pal::new();
        // Sol marin à y=40, eau jusqu'au niveau de la mer.
        let mut grid = vec![pal.air; GRID_LEN].into_boxed_slice();
        for lx in 0..16usize {
            for lz in 0..16usize {
                for wy in MIN_Y..=40 {
                    grid[grid_index(lx, wy, lz)] = pal.dirt;
                }
                for wy in 41..=SEA_LEVEL {
                    grid[grid_index(lx, wy, lz)] = pal.water;
                }
            }
        }
        let idx = [[0u16; 16]; 16];
        let surf = [[40i32; 16]; 16];
        decorate(&mut grid, 7, 0, 0, &idx, &names, &surf);
        let coral = grid
            .iter()
            .filter(|&&b| pal.corals.contains(&b) || b == pal.kelp || b == pal.seagrass)
            .count();
        assert!(coral > 0, "océan chaud sans corail/kelp/seagrass");
    }
}
