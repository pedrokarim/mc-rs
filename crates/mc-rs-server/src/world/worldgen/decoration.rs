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
pub(super) struct Pal {
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
    brown_mushroom: u32,
    red_mushroom: u32,
    pumpkin: u32,
    melon: u32,
    glow_lichen: u32,
    leaf_litter: u32,
    bush: u32,
    berry_bush: u32,
    snow_layer: u32,
    ice: u32,
    stone: u32,
    deepslate: u32,
    lava: u32,
    pointed_dripstone: u32,
    dripstone_block: u32,
    amethyst_block: u32,
    budding_amethyst: u32,
    amethyst_cluster: u32,
    calcite: u32,
    smooth_basalt: u32,
    // Lush caves
    moss_block: u32,
    moss_carpet: u32,
    cave_vines: u32,
    spore_blossom: u32,
    azalea: u32,
    flowering_azalea: u32,
    hanging_roots: u32,
    clay: u32,
    // Deep dark
    sculk: u32,
    sculk_vein: u32,
    sculk_sensor: u32,
    sculk_shrieker: u32,
    sculk_catalyst: u32,
    // Structures
    cobblestone: u32,
    mossy_cobblestone: u32,
    mob_spawner: u32,
    chest: u32,
    sandstone: u32,
    cut_sandstone: u32,
    chiseled_sandstone: u32,
}

// Indices d'espèce dans logs/leaves.
const OAK: usize = 0;
const BIRCH: usize = 1;
const SPRUCE: usize = 2;
const JUNGLE: usize = 3;
const ACACIA: usize = 4;
const DARK_OAK: usize = 5;

impl Pal {
    pub(super) fn new() -> Self {
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
            brown_mushroom: g("minecraft:brown_mushroom"),
            red_mushroom: g("minecraft:red_mushroom"),
            pumpkin: g("minecraft:pumpkin"),
            melon: g("minecraft:melon_block"),
            glow_lichen: g("minecraft:glow_lichen"),
            leaf_litter: g("minecraft:leaf_litter"),
            bush: g("minecraft:bush"),
            berry_bush: g("minecraft:sweet_berry_bush"),
            snow_layer: g("minecraft:snow_layer"),
            ice: g("minecraft:ice"),
            stone: BLOCKS.stone,
            deepslate: g("minecraft:deepslate"),
            lava: g("minecraft:lava"),
            pointed_dripstone: g("minecraft:pointed_dripstone"),
            dripstone_block: g("minecraft:dripstone_block"),
            amethyst_block: g("minecraft:amethyst_block"),
            budding_amethyst: g("minecraft:budding_amethyst"),
            amethyst_cluster: g("minecraft:amethyst_cluster"),
            calcite: g("minecraft:calcite"),
            smooth_basalt: g("minecraft:smooth_basalt"),
            moss_block: g("minecraft:moss_block"),
            moss_carpet: g("minecraft:moss_carpet"),
            cave_vines: g("minecraft:cave_vines_body_with_berries"),
            spore_blossom: g("minecraft:spore_blossom"),
            azalea: g("minecraft:azalea"),
            flowering_azalea: g("minecraft:flowering_azalea"),
            hanging_roots: g("minecraft:hanging_roots"),
            clay: g("minecraft:clay"),
            sculk: g("minecraft:sculk"),
            sculk_vein: g("minecraft:sculk_vein"),
            sculk_sensor: g("minecraft:sculk_sensor"),
            sculk_shrieker: g("minecraft:sculk_shrieker"),
            sculk_catalyst: g("minecraft:sculk_catalyst"),
            cobblestone: g("minecraft:cobblestone"),
            mossy_cobblestone: g("minecraft:mossy_cobblestone"),
            mob_spawner: g("minecraft:mob_spawner"),
            chest: g("minecraft:chest"),
            sandstone: g("minecraft:sandstone"),
            cut_sandstone: g("minecraft:cut_sandstone"),
            chiseled_sandstone: g("minecraft:chiseled_sandstone"),
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

/// Petit bruit de valeur lisse (bilinéaire) ∈ [-1, 1] — tient lieu du bruit
/// basse fréquence vanilla qui pilote la densité d'herbe/fleurs
/// (`noise_threshold_count`, échantillonné à ~1/200 bloc).
fn veg_noise(seed: u64, fx: f64, fz: f64) -> f64 {
    let corner = |a: i64, b: i64| -> f64 {
        let h = (a.wrapping_mul(0x1f1f_1f1f))
            .wrapping_add(b.wrapping_mul(0x27d4_eb2f))
            .wrapping_add(seed as i64) as u64;
        let h = h.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ((h >> 40) as f64 / (1u64 << 24) as f64) * 2.0 - 1.0
    };
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let (x0, z0) = (fx.floor() as i64, fz.floor() as i64);
    let (tx, tz) = (smooth(fx - x0 as f64), smooth(fz - z0 as f64));
    let a = lerp(corner(x0, z0), corner(x0 + 1, z0), tx);
    let b = lerp(corner(x0, z0 + 1), corner(x0 + 1, z0 + 1), tx);
    lerp(a, b, tz)
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
pub(super) enum Species {
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
pub(super) fn tree_plan(biome: &str) -> (f64, Species, &'static [(f64, Species)]) {
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
pub(super) fn pick_tree(default: Species, alts: &[(f64, Species)], rng: &mut Random) -> Species {
    for (chance, species) in alts {
        if rng.next_float() < *chance {
            return *species;
        }
    }
    default
}

pub(super) fn place_tree(
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

/// Nombre de tentatives d'herbe pour un biome, selon les `patch_grass_*`
/// vanilla. Les biomes en `noise_threshold_count` (plaines, savanes…) varient
/// selon `noise` (≥ -0.8 → `above_noise`, sinon `below_noise`) ; les autres ont
/// un `count` fixe (jungle 25, forêt 2…).
fn grass_count(biome: &str, noise: f64) -> i32 {
    match biome {
        "minecraft:jungle" | "minecraft:bamboo_jungle" => 25,
        "minecraft:sparse_jungle" => 12,
        // noise_threshold_count : above 10 / below 5.
        "minecraft:plains" | "minecraft:sunflower_plains" | "minecraft:meadow" => {
            if noise >= -0.8 {
                10
            } else {
                5
            }
        }
        "minecraft:savanna" | "minecraft:savanna_plateau" | "minecraft:windswept_savanna" => {
            if noise >= -0.8 {
                8
            } else {
                4
            }
        }
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

/// Biomes froids (neige/glace en surface) — même ensemble que les surface rules.
fn is_cold(biome: &str) -> bool {
    matches!(
        biome,
        "minecraft:snowy_plains"
            | "minecraft:ice_spikes"
            | "minecraft:snowy_taiga"
            | "minecraft:snowy_beach"
            | "minecraft:grove"
            | "minecraft:snowy_slopes"
            | "minecraft:frozen_peaks"
            | "minecraft:jagged_peaks"
            | "minecraft:frozen_ocean"
            | "minecraft:deep_frozen_ocean"
            | "minecraft:frozen_river"
    )
}

/// Feature vanilla `glow_lichen` : count 104-157, accroché sur une face de
/// pierre/deepslate exposée (grottes), toute la hauteur.
fn decorate_glow_lichen(grid: &mut [u32], pal: &Pal, rng: &mut Random) {
    let count = 104 + rng.next_bounded_int(54);
    for _ in 0..count {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 4 + rng.next_bounded_int(180); // ~ -60..120
        if at(grid, lx, wy, lz) != pal.air {
            continue;
        }
        for (dx, dy, dz) in [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let nb = at(grid, lx + dx, wy + dy, lz + dz);
            if nb == pal.stone || nb == pal.deepslate {
                if let Some(i) = idx_ok(lx, wy, lz) {
                    grid[i] = pal.glow_lichen;
                }
                break;
            }
        }
    }
}

/// `freeze_top_layer` : neige au sol des biomes froids + glace sur l'eau.
fn decorate_snow(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
) {
    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            if !is_cold(biome_at(lx as usize, lz as usize)) {
                continue;
            }
            let ground = surfaces[lx as usize][lz as usize];
            if ground > SEA_LEVEL {
                // Neige sur le sol exposé.
                if at(grid, lx, ground + 1, lz) == pal.air {
                    if let Some(i) = idx_ok(lx, ground + 1, lz) {
                        grid[i] = pal.snow_layer;
                    }
                }
            } else if at(grid, lx, SEA_LEVEL, lz) == pal.water
                && at(grid, lx, SEA_LEVEL + 1, lz) == pal.air
            {
                // Surface d'eau gelée.
                if let Some(i) = idx_ok(lx, SEA_LEVEL, lz) {
                    grid[i] = pal.ice;
                }
            }
        }
    }
}

#[inline]
fn is_rock(pal: &Pal, b: u32) -> bool {
    b == pal.stone || b == pal.deepslate
}

/// Dripstone : `pointed_dripstone` (stalactites/stalagmites) +
/// `dripstone_cluster` (blocs de dripstone au sol/plafond des grottes).
///
/// NB : faute de support des états de bloc (`dripstone_thickness`
/// tip/frustum/base), on pose des **pointes courtes** (1 bloc, parfois 2) plutôt
/// que de longues colonnes uniformes (qui rendaient mal). Densité réduite.
fn decorate_dripstone(grid: &mut [u32], pal: &Pal, rng: &mut Random) {
    let pointed = 64 + rng.next_bounded_int(64);
    for _ in 0..pointed {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 4 + rng.next_bounded_int(160); // ~ -60..100
        if at(grid, lx, wy, lz) != pal.air {
            continue;
        }
        let ceiling = is_rock(pal, at(grid, lx, wy + 1, lz));
        let floor = is_rock(pal, at(grid, lx, wy - 1, lz));
        // Pointe courte (surtout 1 bloc).
        let len = if rng.next_bounded_int(4) == 0 { 2 } else { 1 };
        let dir = if ceiling {
            -1
        } else if floor {
            1
        } else {
            continue;
        };
        for d in 0..len {
            let y = wy + dir * d;
            if at(grid, lx, y, lz) != pal.air {
                break;
            }
            if let Some(i) = idx_ok(lx, y, lz) {
                grid[i] = pal.pointed_dripstone;
            }
        }
    }

    let clusters = 48 + rng.next_bounded_int(48);
    for _ in 0..clusters {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 4 + rng.next_bounded_int(160);
        // Sol de grotte : air avec roche dessous → bloc de dripstone.
        if at(grid, lx, wy, lz) == pal.air && is_rock(pal, at(grid, lx, wy - 1, lz)) {
            if let Some(i) = idx_ok(lx, wy - 1, lz) {
                grid[i] = pal.dripstone_block;
            }
        }
    }
}

/// Géode d'améthyste : `amethyst_geode` (1/24, y ∈ [-58, 30]). Sphère creuse :
/// coquille `smooth_basalt`, `calcite`, `amethyst_block`, centre creux avec
/// `budding_amethyst` + `amethyst_cluster`.
fn decorate_geode(grid: &mut [u32], pal: &Pal, rng: &mut Random) {
    if rng.next_bounded_int(24) != 0 {
        return;
    }
    let cx = rng.next_bounded_int(16);
    let cz = rng.next_bounded_int(16);
    let cy = -58 + rng.next_bounded_int(89); // [-58, 30]
    for dx in -6..=6i32 {
        for dy in -6..=6i32 {
            for dz in -6..=6i32 {
                let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                let cur = at(grid, x, y, z);
                // On ne creuse que la roche (ou l'air déjà présent dans la coquille).
                if !is_rock(pal, cur) && cur != pal.air {
                    continue;
                }
                let d2 = dx * dx + dy * dy + dz * dz;
                let block = if d2 <= 9 {
                    // Centre creux : améthyste bourgeonnante par endroits, sinon air.
                    if d2 >= 7 && rng.next_bounded_int(3) == 0 {
                        pal.budding_amethyst
                    } else if d2 >= 7 && rng.next_bounded_int(4) == 0 {
                        pal.amethyst_cluster
                    } else {
                        pal.air
                    }
                } else if d2 <= 16 {
                    pal.amethyst_block
                } else if d2 <= 20 {
                    pal.calcite
                } else if d2 <= 25 {
                    pal.smooth_basalt
                } else {
                    continue;
                };
                if is_rock(pal, cur) || (cur == pal.air && block != pal.air) {
                    if let Some(i) = idx_ok(x, y, z) {
                        grid[i] = block;
                    }
                }
            }
        }
    }
}

/// Lac de lave souterrain (`lake_lava_underground`, 1/9) : petite cuvette de
/// lave dans une grotte.
fn decorate_lava_lake(grid: &mut [u32], pal: &Pal, rng: &mut Random) {
    if rng.next_bounded_int(9) != 0 {
        return;
    }
    // Cherche un sol de grotte souterrain.
    for _ in 0..32 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 6 + rng.next_bounded_int(60); // profond (~ -58..2)
        if at(grid, lx, wy, lz) == pal.air && is_rock(pal, at(grid, lx, wy - 1, lz)) {
            // Cuvette : lave en bas, air au-dessus, dans un petit ellipsoïde.
            for dx in -4..=4i32 {
                for dz in -4..=4i32 {
                    for dy in -2..=2i32 {
                        if dx * dx + dz * dz + dy * dy * 4 > 16 {
                            continue;
                        }
                        let (x, y, z) = (lx + dx, wy + dy, lz + dz);
                        let cur = at(grid, x, y, z);
                        if !is_rock(pal, cur) && cur != pal.air {
                            continue;
                        }
                        if let Some(i) = idx_ok(x, y, z) {
                            grid[i] = if dy <= 0 { pal.lava } else { pal.air };
                        }
                    }
                }
            }
            return;
        }
    }
}

/// Biome 3D (cellule 4×4×4) à une position locale.
fn cave_biome<'a>(
    biome3d: &[[[u16; 4]; 4]],
    names: &'a [String],
    lx: i32,
    wy: i32,
    lz: i32,
) -> &'a str {
    let sub = (((wy - MIN_Y) / 16).clamp(0, biome3d.len() as i32 - 1)) as usize;
    let cx = (lx.clamp(0, 15) / 4) as usize;
    let cz = (lz.clamp(0, 15) / 4) as usize;
    &names[biome3d[sub][cx][cz] as usize]
}

/// Lush caves : mousse, azalées, lianes de grotte (baies), spore blossom,
/// racines, dans le biome `lush_caves`.
fn decorate_lush(
    grid: &mut [u32],
    pal: &Pal,
    biome3d: &[[[u16; 4]; 4]],
    names: &[String],
    rng: &mut Random,
) {
    for _ in 0..140 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 8 + rng.next_bounded_int(110); // ~ -56..62
        if at(grid, lx, wy, lz) != pal.air
            || cave_biome(biome3d, names, lx, wy, lz) != "minecraft:lush_caves"
        {
            continue;
        }
        let floor = is_rock(pal, at(grid, lx, wy - 1, lz));
        let ceiling = is_rock(pal, at(grid, lx, wy + 1, lz));
        if floor {
            if let Some(i) = idx_ok(lx, wy - 1, lz) {
                grid[i] = pal.moss_block;
            }
            match rng.next_bounded_int(6) {
                0 => plant(grid, pal, lx, wy, lz, pal.azalea),
                1 => plant(grid, pal, lx, wy, lz, pal.flowering_azalea),
                2 => plant(grid, pal, lx, wy, lz, pal.moss_carpet),
                3 => {
                    if let Some(i) = idx_ok(lx, wy - 1, lz) {
                        grid[i] = pal.clay;
                    }
                }
                _ => {}
            }
        } else if ceiling {
            if let Some(i) = idx_ok(lx, wy + 1, lz) {
                grid[i] = pal.moss_block;
            }
            match rng.next_bounded_int(4) {
                0 => {
                    // Lianes de grotte (baies) pendantes.
                    let len = 1 + rng.next_bounded_int(8);
                    for d in 0..len {
                        if at(grid, lx, wy - d, lz) != pal.air {
                            break;
                        }
                        if let Some(i) = idx_ok(lx, wy - d, lz) {
                            grid[i] = pal.cave_vines;
                        }
                    }
                }
                1 => plant(grid, pal, lx, wy, lz, pal.spore_blossom),
                _ => plant(grid, pal, lx, wy, lz, pal.hanging_roots),
            }
        }
    }
}

/// Deep dark : sculk au sol + capteurs/hurleurs/catalyseurs, dans `deep_dark`.
fn decorate_deep_dark(
    grid: &mut [u32],
    pal: &Pal,
    biome3d: &[[[u16; 4]; 4]],
    names: &[String],
    rng: &mut Random,
) {
    for _ in 0..120 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let wy = MIN_Y + 6 + rng.next_bounded_int(40); // profond (~ -58..-18)
        if at(grid, lx, wy, lz) != pal.air
            || cave_biome(biome3d, names, lx, wy, lz) != "minecraft:deep_dark"
        {
            continue;
        }
        if is_rock(pal, at(grid, lx, wy - 1, lz)) {
            if let Some(i) = idx_ok(lx, wy - 1, lz) {
                grid[i] = pal.sculk;
            }
            match rng.next_bounded_int(12) {
                0 => plant(grid, pal, lx, wy, lz, pal.sculk_sensor),
                1 => plant(grid, pal, lx, wy, lz, pal.sculk_shrieker),
                2 => plant(grid, pal, lx, wy, lz, pal.sculk_catalyst),
                3 => plant(grid, pal, lx, wy, lz, pal.sculk_vein),
                _ => {}
            }
        }
    }
}

/// Donjon (monster room) : petite salle de cobblestone/mossy avec spawner +
/// coffres, sous terre. ~1/3 des chunks tentent d'en placer un.
fn decorate_dungeon(grid: &mut [u32], pal: &Pal, rng: &mut Random) {
    if rng.next_bounded_int(3) != 0 {
        return;
    }
    for _ in 0..16 {
        let lx = 3 + rng.next_bounded_int(10);
        let lz = 3 + rng.next_bounded_int(10);
        let cy = MIN_Y + 8 + rng.next_bounded_int(50);
        if !is_rock(pal, at(grid, lx, cy - 1, lz)) {
            continue;
        }
        let half = 1 + rng.next_bounded_int(2); // intérieur 3×3 ou 5×5
        for dx in -(half + 1)..=(half + 1) {
            for dz in -(half + 1)..=(half + 1) {
                for dy in -1..=3 {
                    let (x, y, z) = (lx + dx, cy + dy, lz + dz);
                    let edge = dx.abs() == half + 1 || dz.abs() == half + 1 || dy == -1 || dy == 3;
                    if let Some(i) = idx_ok(x, y, z) {
                        grid[i] = if edge {
                            if rng.next_bounded_int(4) == 0 {
                                pal.mossy_cobblestone
                            } else {
                                pal.cobblestone
                            }
                        } else {
                            pal.air
                        };
                    }
                }
            }
        }
        // Spawner au centre.
        if let Some(i) = idx_ok(lx, cy, lz) {
            grid[i] = pal.mob_spawner;
        }
        // 1-2 coffres au sol (intérieur).
        for _ in 0..(1 + rng.next_bounded_int(2)) {
            let px = lx + rng.next_range(-half, half);
            let pz = lz + rng.next_range(-half, half);
            if at(grid, px, cy, pz) == pal.air && !(px == lx && pz == lz) {
                if let Some(i) = idx_ok(px, cy, pz) {
                    grid[i] = pal.chest;
                }
            }
        }
        return;
    }
}

/// Puits du désert : structure de grès avec bassin d'eau + piliers, en surface
/// de désert. Très rare (~1/1000).
fn decorate_desert_well(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
    rng: &mut Random,
) {
    if rng.next_bounded_int(1000) != 0 {
        return;
    }
    for _ in 0..16 {
        let lx = 3 + rng.next_bounded_int(10);
        let lz = 3 + rng.next_bounded_int(10);
        if !biome_names[biome_idx[lx as usize][lz as usize] as usize].contains("desert") {
            continue;
        }
        let g = surfaces[lx as usize][lz as usize];
        if g <= SEA_LEVEL {
            continue;
        }
        // Plateforme de grès 5×5 + bassin d'eau 3×3.
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                if let Some(i) = idx_ok(lx + dx, g, lz + dz) {
                    grid[i] = pal.sandstone;
                }
            }
        }
        for dx in -1..=1i32 {
            for dz in -1..=1i32 {
                if let Some(i) = idx_ok(lx + dx, g, lz + dz) {
                    grid[i] = pal.water;
                }
            }
        }
        // 4 piliers de grès taillé + toit de grès ciselé.
        for (px, pz) in [(-2, -2), (-2, 2), (2, -2), (2, 2)] {
            for dy in 1..=3 {
                if let Some(i) = idx_ok(lx + px, g + dy, lz + pz) {
                    grid[i] = pal.cut_sandstone;
                }
            }
        }
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                if let Some(i) = idx_ok(lx + dx, g + 4, lz + dz) {
                    grid[i] = pal.chiseled_sandstone;
                }
            }
        }
        return;
    }
}

/// Point d'entrée : décore un chunk déjà terrassé + habillé en surface.
#[allow(clippy::too_many_arguments)]
pub fn decorate(
    grid: &mut [u32],
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
    biome3d: &[[[u16; 4]; 4]],
) {
    let pal = Pal::new();
    let mut rng = Random::new(
        0x9e37_79b9_i64
            .wrapping_mul(chunk_x as i64)
            .wrapping_add((chunk_z as i64) << 16)
            ^ seed as i64,
    );

    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };

    // ── Arbres : déplacés dans `noise_chunk` (passe de population CROSS-CHUNK
    // sur le voisinage 3×3, pour que les canopées débordant d'un chunk à l'autre
    // soient cohérentes et non coupées aux frontières). Ils sont posés AVANT cet
    // appel à `decorate`, donc les lianes ci-dessous les voient déjà.

    // ── Lianes (après les arbres) : feature vanilla `vines`, count 127 ──
    decorate_vines(grid, &pal, biome_idx, biome_names, &mut rng);

    // ── Herbe / fougères (densité pilotée par bruit, `noise_threshold_count`) ──
    let veg_n = veg_noise(
        seed,
        (chunk_x * 16) as f64 / 200.0,
        (chunk_z * 16) as f64 / 200.0,
    );
    let grass_attempts: i32 = {
        let sum: i32 = (0..16)
            .flat_map(|x| (0..16).map(move |z| (x, z)))
            .map(|(x, z)| grass_count(biome_at(x, z), veg_n))
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
        let id = if b.contains("taiga") || b.contains("grove") {
            pal.fern
        } else {
            pal.short_grass
        };
        plant(grid, &pal, lx, ground + 1, lz, id);
    }

    // ── Fleurs (rarity-gated par biome, set vanilla) ──
    decorate_flowers(
        grid,
        &pal,
        biome_idx,
        biome_names,
        surfaces,
        &mut rng,
        veg_n,
    );

    // ── Spécial : cactus / arbustes morts / canne à sucre / bambou / nénuphars ──
    decorate_special(grid, &pal, biome_idx, biome_names, surfaces, &mut rng);

    // ── Aquatique : kelp / seagrass / coraux ──
    decorate_aquatic(grid, &pal, biome_idx, biome_names, surfaces, &mut rng);

    // ── Features de grottes : glow lichen, dripstone, géodes, lacs de lave ──
    decorate_glow_lichen(grid, &pal, &mut rng);
    decorate_dripstone(grid, &pal, &mut rng);
    decorate_geode(grid, &pal, &mut rng);
    decorate_lava_lake(grid, &pal, &mut rng);

    // ── Biomes 3D de grottes : lush caves (mousse/azalée/lianes) & deep dark (sculk) ──
    decorate_lush(grid, &pal, biome3d, biome_names, &mut rng);
    decorate_deep_dark(grid, &pal, biome3d, biome_names, &mut rng);

    // ── Structures : donjons (spawner + coffres), puits du désert ──
    decorate_dungeon(grid, &pal, &mut rng);
    decorate_desert_well(grid, &pal, biome_idx, biome_names, surfaces, &mut rng);

    // ── Neige/glace (biomes froids), en dernier (top layer) ──
    decorate_snow(grid, &pal, biome_idx, biome_names, surfaces);
}

/// Fleurs : rarité officielle par biome + set vanilla. Plaines 1/32 (4 ou 15
/// selon le bruit, dandelion-dominant) ; jungle/warm 1/16 (poppy/dandelion) ;
/// flower_forest dense et varié ; autres forêts/savanes ~1/8 (poppy/dandelion).
#[allow(clippy::too_many_arguments)]
fn decorate_flowers(
    grid: &mut [u32],
    pal: &Pal,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
    rng: &mut Random,
    noise: f64,
) {
    let biome_at = |lx: usize, lz: usize| -> &str { &biome_names[biome_idx[lx][lz] as usize] };
    let put = |grid: &mut [u32], lx: i32, lz: i32, id: u32| {
        let ground = surfaces[lx as usize][lz as usize];
        if ground > SEA_LEVEL
            && at(grid, lx, ground, lz) == pal.grass_block
            && at(grid, lx, ground + 1, lz) == pal.air
        {
            plant(grid, pal, lx, ground + 1, lz, id);
        }
    };
    let f = &pal.flowers; // [dandelion, poppy, cornflower, oxeye, allium, azure_bluet]
    let is_plains = |b: &str| {
        matches!(
            b,
            "minecraft:plains" | "minecraft:sunflower_plains" | "minecraft:meadow"
        )
    };
    let present =
        |pred: &dyn Fn(&str) -> bool| (0..16).any(|x| (0..16).any(|z| pred(biome_at(x, z))));

    // Plaines : 1/32, count bruit (4 / 15), dandelion par défaut, 1/3 → autre.
    if present(&is_plains) && rng.next_bounded_int(32) == 0 {
        let n = if noise >= -0.8 { 4 } else { 15 };
        for _ in 0..n {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if !is_plains(biome_at(lx as usize, lz as usize)) {
                continue;
            }
            let id = if rng.next_bounded_int(3) == 0 {
                f[[1usize, 3, 5][rng.next_bounded_int(3) as usize]]
            } else {
                f[0]
            };
            put(grid, lx, lz, id);
        }
    }

    // Jungle / warm : 1/16, poppy(2)/dandelion(1).
    if present(&is_jungle) && rng.next_bounded_int(16) == 0 {
        for _ in 0..(2 + rng.next_bounded_int(5)) {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if !is_jungle(biome_at(lx as usize, lz as usize)) {
                continue;
            }
            let id = if rng.next_bounded_int(3) < 2 {
                f[1]
            } else {
                f[0]
            };
            put(grid, lx, lz, id);
        }
    }

    // Flower forest : dense et varié (pas de rarity).
    if present(&|b| b == "minecraft:flower_forest") {
        for _ in 0..40 {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if biome_at(lx as usize, lz as usize) != "minecraft:flower_forest" {
                continue;
            }
            let id = f[rng.next_bounded_int(f.len() as i32) as usize];
            put(grid, lx, lz, id);
        }
    }

    // Autres forêts / savanes (flower_default) : ~1/8, poppy/dandelion.
    let is_other =
        |b: &str| (b.contains("forest") && b != "minecraft:flower_forest") || b.contains("savanna");
    if present(&is_other) && rng.next_bounded_int(8) == 0 {
        for _ in 0..(2 + rng.next_bounded_int(4)) {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if !is_other(biome_at(lx as usize, lz as usize)) {
                continue;
            }
            let id = if rng.next_bounded_int(3) < 2 {
                f[1]
            } else {
                f[0]
            };
            put(grid, lx, lz, id);
        }
    }
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

    // Place un bloc sur une colonne herbeuse aléatoire (helper local).
    let on_grass = |grid: &mut [u32], rng: &mut Random, id: u32| {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        let ground = surfaces[lx as usize][lz as usize];
        if ground > SEA_LEVEL
            && at(grid, lx, ground, lz) == pal.grass_block
            && at(grid, lx, ground + 1, lz) == pal.air
        {
            plant(grid, pal, lx, ground + 1, lz, id);
        }
    };

    // Champignons (officiel : brun 1/256, rouge 1/512).
    if rng.next_bounded_int(256) == 0 {
        on_grass(grid, rng, pal.brown_mushroom);
    }
    if rng.next_bounded_int(512) == 0 {
        on_grass(grid, rng, pal.red_mushroom);
    }

    // Citrouilles (officiel : 1/300) — petit patch.
    if rng.next_bounded_int(300) == 0 {
        for _ in 0..(1 + rng.next_bounded_int(4)) {
            on_grass(grid, rng, pal.pumpkin);
        }
    }

    // Melons (jungle, officiel : 1/6) — petit patch.
    let has_jungle = (0..16).any(|x| (0..16).any(|z| is_jungle(biome_at(x, z))));
    if has_jungle && rng.next_bounded_int(6) == 0 {
        for _ in 0..(2 + rng.next_bounded_int(5)) {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if !is_jungle(biome_at(lx as usize, lz as usize)) {
                continue;
            }
            let ground = surfaces[lx as usize][lz as usize];
            if ground > SEA_LEVEL
                && at(grid, lx, ground, lz) == pal.grass_block
                && at(grid, lx, ground + 1, lz) == pal.air
            {
                plant(grid, pal, lx, ground + 1, lz, pal.melon);
            }
        }
    }

    // Leaf litter au sol des forêts (sous les arbres).
    let has_forest = (0..16).any(|x| (0..16).any(|z| biome_at(x, z).contains("forest")));
    if has_forest {
        for _ in 0..24 {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            if !biome_at(lx as usize, lz as usize).contains("forest") {
                continue;
            }
            on_grass(grid, rng, pal.leaf_litter);
        }
    }

    // Buissons (`patch_bush`, officiel 1/4) — petit patch sur l'herbe.
    if rng.next_bounded_int(4) == 0 {
        for _ in 0..(2 + rng.next_bounded_int(4)) {
            on_grass(grid, rng, pal.bush);
        }
    }

    // Buissons de baies (taïga).
    for _ in 0..3 {
        let lx = rng.next_bounded_int(16);
        let lz = rng.next_bounded_int(16);
        if !biome_at(lx as usize, lz as usize).contains("taiga") {
            continue;
        }
        on_grass(grid, rng, pal.berry_bush);
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

            // Kelp (océans non gelés) : colonne, bien plus clairsemée qu'avant.
            if is_ocean(b) && !b.contains("frozen") && rng.next_bounded_int(20) == 0 {
                let max = SEA_LEVEL - 1;
                // Hauteur modérée (ne remplit pas toute la colonne d'eau).
                let height = 2 + rng.next_bounded_int(((max - floor) / 2).max(1));
                for dy in 0..height {
                    let y = floor + 1 + dy;
                    if y >= max || at(grid, lx, y, lz) != pal.water {
                        break;
                    }
                    if let Some(i) = idx_ok(lx, y, lz) {
                        grid[i] = pal.kelp;
                    }
                }
            } else if rng.next_bounded_int(10) == 0 {
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
        // Les arbres sont posés par `noise_chunk` (passe cross-chunk), mais la
        // composition + le placement vivent ici : on vérifie qu'un arbre de la
        // forêt produit bien tronc + feuilles.
        let pal = Pal::new();
        let (_, default, _) = tree_plan("minecraft:forest");
        let mut grid = vec![pal.air; GRID_LEN].into_boxed_slice();
        let mut rng = Random::new(42);
        place_tree(&mut grid, &pal, default, 8, 70, 8, &mut rng);
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
        decorate(
            &mut grid,
            99,
            0,
            0,
            &idx,
            &names,
            &surf,
            &[[[0u16; 4]; 4]; 1],
        );
        let pal = Pal::new();
        let vines = grid.iter().filter(|&&b| b == pal.vine).count();
        assert!(vines > 20, "jungle trop pauvre en lianes: {vines}");
    }

    #[test]
    fn cold_biome_gets_snow_layer() {
        let names = vec!["minecraft:snowy_plains".to_string()];
        let (mut grid, idx, surf) = flat_chunk(80, 0);
        decorate(
            &mut grid,
            1,
            0,
            0,
            &idx,
            &names,
            &surf,
            &[[[0u16; 4]; 4]; 1],
        );
        let pal = Pal::new();
        assert!(
            grid.contains(&pal.snow_layer),
            "pas de neige en biome froid"
        );
    }

    #[test]
    fn dripstone_in_caves() {
        let pal = Pal::new();
        let mut grid = vec![pal.stone; GRID_LEN].into_boxed_slice();
        // Couche de grotte (air) à y∈[10,14).
        for lx in 0..16usize {
            for lz in 0..16usize {
                for y in 10..14 {
                    grid[grid_index(lx, y, lz)] = pal.air;
                }
            }
        }
        let mut rng = Random::new(3);
        decorate_dripstone(&mut grid, &pal, &mut rng);
        assert!(
            grid.contains(&pal.pointed_dripstone) || grid.contains(&pal.dripstone_block),
            "pas de dripstone dans la grotte"
        );
    }

    #[test]
    fn geode_places_amethyst() {
        let pal = Pal::new();
        // La géode est en 1/24 : on essaie plusieurs seeds jusqu'à ce qu'elle tombe.
        let mut found = false;
        for s in 0..200i64 {
            let mut grid = vec![pal.stone; GRID_LEN].into_boxed_slice();
            let mut rng = Random::new(s);
            decorate_geode(&mut grid, &pal, &mut rng);
            if grid.contains(&pal.amethyst_block) {
                found = true;
                break;
            }
        }
        assert!(found, "aucune géode placée sur 200 essais");
    }

    #[test]
    fn dungeon_places_spawner_and_chest() {
        let pal = Pal::new();
        let mut found = false;
        for s in 0..40i64 {
            let mut g = vec![pal.stone; GRID_LEN].into_boxed_slice();
            let mut rng = Random::new(s);
            decorate_dungeon(&mut g, &pal, &mut rng);
            if g.contains(&pal.mob_spawner) {
                assert!(g.contains(&pal.cobblestone), "donjon sans murs");
                found = true;
                break;
            }
        }
        assert!(found, "aucun donjon placé sur 40 essais");
    }

    #[test]
    fn lush_and_deep_dark_cave_features() {
        let pal = Pal::new();
        let make = || {
            let mut g = vec![pal.stone; GRID_LEN].into_boxed_slice();
            for lx in 0..16usize {
                for lz in 0..16usize {
                    for y in -40..-36 {
                        g[grid_index(lx, y, lz)] = pal.air;
                    }
                }
            }
            g
        };
        let b3d = vec![[[0u16; 4]; 4]; 24];

        let mut g = make();
        let mut rng = Random::new(2);
        decorate_lush(
            &mut g,
            &pal,
            &b3d,
            &["minecraft:lush_caves".to_string()],
            &mut rng,
        );
        assert!(g.contains(&pal.moss_block), "pas de mousse en lush caves");

        let mut g2 = make();
        let mut rng2 = Random::new(2);
        decorate_deep_dark(
            &mut g2,
            &pal,
            &b3d,
            &["minecraft:deep_dark".to_string()],
            &mut rng2,
        );
        assert!(g2.contains(&pal.sculk), "pas de sculk en deep dark");
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
        decorate(
            &mut grid,
            7,
            0,
            0,
            &idx,
            &names,
            &surf,
            &[[[0u16; 4]; 4]; 1],
        );
        let coral = grid
            .iter()
            .filter(|&&b| pal.corals.contains(&b) || b == pal.kelp || b == pal.seagrass)
            .count();
        assert!(coral > 0, "océan chaud sans corail/kelp/seagrass");
    }
}
