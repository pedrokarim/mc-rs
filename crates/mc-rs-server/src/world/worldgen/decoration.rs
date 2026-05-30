//! Décoration riche du générateur noise — objectif : proche de Bedrock Edition.
//!
//! Placement par biome (noms Java) directement sur la grille de blocs :
//! - arbres variés par espèce et taille (chêne petit/géant, bouleau, sapin,
//!   jungle petite/géante 2×2, chêne noir 2×2, acacia) + **lianes**,
//! - herbe haute / fougères / fleurs,
//! - **aquatique** : kelp, seagrass, récifs de **corail** + sea pickles.
//!
//! Remplace l'usage du module `vegetation` (legacy) pour ce générateur. Les
//! formes d'arbres sont des approximations fidèles des features vanilla ; le
//! portage 100 % data-driven (`placed_feature`/`configured_feature`) viendra
//! ensuite. Les arbres en bord de chunk sont rognés (pas de débordement).

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

#[derive(Clone, Copy)]
enum Species {
    OakSmall,
    OakLarge,
    Birch,
    Spruce,
    JungleSmall,
    JungleGiant,
    DarkOak,
    Acacia,
}

/// (densité = arbres/chunk, espèces pondérées) par biome Java.
///
/// Densités = valeurs **officielles vanilla** (moyenne du modificateur `count`
/// des `placed_feature` Java : `trees_<biome>`, etc.). Ex. plaines `{0:19,1:1}`
/// → 0.05 ; jungle `{50:9,51:1}` → 50.1 ; mangrove `count 25`.
fn tree_plan(biome: &str) -> (f64, &'static [(Species, i32)]) {
    use Species::*;
    match biome {
        "minecraft:forest" => (10.0, &[(OakSmall, 6), (Birch, 4), (OakLarge, 1)]),
        "minecraft:flower_forest" => (6.0, &[(OakSmall, 6), (Birch, 4), (OakLarge, 1)]),
        "minecraft:birch_forest" => (10.0, &[(Birch, 1)]),
        "minecraft:old_growth_birch_forest" => (10.0, &[(Birch, 1)]),
        "minecraft:dark_forest" => (16.0, &[(DarkOak, 6), (OakSmall, 3), (Birch, 1)]),
        "minecraft:taiga" | "minecraft:snowy_taiga" => (10.0, &[(Spruce, 1)]),
        "minecraft:old_growth_pine_taiga" | "minecraft:old_growth_spruce_taiga" => {
            (10.0, &[(Spruce, 1)])
        }
        "minecraft:grove" => (10.0, &[(Spruce, 1)]),
        "minecraft:snowy_slopes" => (0.1, &[(Spruce, 1)]),
        "minecraft:jungle" => (50.0, &[(JungleSmall, 10), (JungleGiant, 2), (OakLarge, 1)]),
        "minecraft:bamboo_jungle" => (30.0, &[(JungleSmall, 4), (JungleGiant, 1)]),
        "minecraft:sparse_jungle" => (2.0, &[(JungleSmall, 2), (OakSmall, 1)]),
        "minecraft:savanna" | "minecraft:savanna_plateau" | "minecraft:windswept_savanna" => {
            (1.1, &[(Acacia, 4), (OakSmall, 1)])
        }
        "minecraft:swamp" => (0.1, &[(OakSmall, 1)]),
        // Mangrove : dense (count 25). Approximé en chêne + lianes faute de
        // blocs de palétuvier dédiés.
        "minecraft:mangrove_swamp" => (25.0, &[(OakSmall, 1)]),
        "minecraft:plains" | "minecraft:sunflower_plains" => {
            (0.05, &[(OakSmall, 4), (OakLarge, 1)])
        }
        "minecraft:meadow" => (0.05, &[(OakSmall, 1)]),
        "minecraft:windswept_forest" => (3.0, &[(Spruce, 2), (OakSmall, 1)]),
        "minecraft:windswept_hills" | "minecraft:windswept_gravelly_hills" => {
            (0.1, &[(Spruce, 2), (OakSmall, 1)])
        }
        "minecraft:cherry_grove" => (10.0, &[(OakSmall, 1)]),
        _ => (0.0, &[]),
    }
}

fn pick_species(plan: &[(Species, i32)], rng: &mut Random) -> Species {
    let total: i32 = plan.iter().map(|(_, w)| *w).sum();
    let mut r = rng.next_bounded_int(total.max(1));
    for (s, w) in plan {
        r -= *w;
        if r < 0 {
            return *s;
        }
    }
    plan[0].0
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
        Species::OakSmall => straight_tree(grid, pal, lx, ground, lz, OAK, 4, 3, false, rng),
        Species::OakLarge => straight_tree(grid, pal, lx, ground, lz, OAK, 7, 5, false, rng),
        Species::Birch => straight_tree(grid, pal, lx, ground, lz, BIRCH, 5, 3, false, rng),
        Species::Spruce => spruce_tree(grid, pal, lx, ground, lz, rng),
        Species::JungleSmall => straight_tree(grid, pal, lx, ground, lz, JUNGLE, 5, 6, true, rng),
        Species::JungleGiant => mega_tree(grid, pal, lx, ground, lz, JUNGLE, 11, 9, true, rng),
        Species::DarkOak => mega_tree(grid, pal, lx, ground, lz, DARK_OAK, 6, 3, false, rng),
        Species::Acacia => acacia_tree(grid, pal, lx, ground, lz, rng),
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
        let (_, plan) = tree_plan(biome_at(lx as usize, lz as usize));
        if plan.is_empty() {
            continue;
        }
        let species = pick_species(plan, &mut rng);
        place_tree(grid, &pal, species, lx, ground, lz, &mut rng);
    }

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
