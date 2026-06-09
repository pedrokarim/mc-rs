//! Décoration du Nether par biome, appliquée après le terrain (`dim_noise`) via le
//! callback `decorate`. Le biome vient du climat multi-noise ; la dispersion est
//! déterministe (hash position+seed), donc rejouable.
//!
//! Réf : `new-docs/27-DIMENSIONS-GENERATION-SPEC.md` (données vanilla mcmeta).
//! - **Minerais** : vraies veines (moteur `feature`) — quartz/or/magma/gravel/
//!   blackstone/soul_sand/ancient_debris, `count`/chunk + `size` + plage Y exacts.
//! - **Surface** : surface rule par biome (nylium, soul sand/gravel, basalt/gravel).
//! - **Végétation** : `count_on_every_layer` pondéré + huge fungus (tige + dôme).
//! - **basalt_deltas** : blobs basalt/blackstone, deltas de lave, colonnes.
//! - Springs de lave, amas de glowstone, feu/soul fire, lianes (twisting/weeping).

use std::sync::LazyLock;

use super::block_registry::BLOCKS;
use super::feature::FeatureRng;
use super::nether_generator::nether_biome_index;
use super::worldgen::climate::ClimateSampler;
use super::worldgen::dim_noise::DimGrid;

// Index biome dans biome_parameters/nether.json (ordre du fichier).
const WASTES: u16 = 0;
const SOUL_SAND_VALLEY: u16 = 1;
const CRIMSON: u16 = 2;
const WARPED: u16 = 3;
const BASALT: u16 = 4;

struct NetherBlocks {
    air: u32,
    netherrack: u32,
    crimson_nylium: u32,
    warped_nylium: u32,
    soul_sand: u32,
    soul_soil: u32,
    basalt: u32,
    blackstone: u32,
    magma: u32,
    glowstone: u32,
    fire: u32,
    soul_fire: u32,
    crimson_roots: u32,
    warped_roots: u32,
    nether_sprouts: u32,
    weeping_vines: u32,
    twisting_vines: u32,
    crimson_stem: u32,
    warped_stem: u32,
    nether_wart_block: u32,
    warped_wart_block: u32,
    shroomlight: u32,
    quartz_ore: u32,
    gold_ore: u32,
    ancient_debris: u32,
    gravel: u32,
    lava: u32,
}

static NB: LazyLock<NetherBlocks> = LazyLock::new(|| NetherBlocks {
    air: BLOCKS.air,
    netherrack: BLOCKS.get("minecraft:netherrack"),
    crimson_nylium: BLOCKS.get("minecraft:crimson_nylium"),
    warped_nylium: BLOCKS.get("minecraft:warped_nylium"),
    soul_sand: BLOCKS.get("minecraft:soul_sand"),
    soul_soil: BLOCKS.get("minecraft:soul_soil"),
    basalt: BLOCKS.get("minecraft:basalt"),
    blackstone: BLOCKS.get("minecraft:blackstone"),
    magma: BLOCKS.get("minecraft:magma"),
    glowstone: BLOCKS.get("minecraft:glowstone"),
    fire: BLOCKS.get("minecraft:fire"),
    soul_fire: BLOCKS.get("minecraft:soul_fire"),
    crimson_roots: BLOCKS.get("minecraft:crimson_roots"),
    warped_roots: BLOCKS.get("minecraft:warped_roots"),
    nether_sprouts: BLOCKS.get("minecraft:nether_sprouts"),
    weeping_vines: BLOCKS.get("minecraft:weeping_vines"),
    twisting_vines: BLOCKS.get("minecraft:twisting_vines"),
    crimson_stem: BLOCKS.get("minecraft:crimson_stem"),
    warped_stem: BLOCKS.get("minecraft:warped_stem"),
    nether_wart_block: BLOCKS.get("minecraft:nether_wart_block"),
    warped_wart_block: BLOCKS.get("minecraft:warped_wart_block"),
    shroomlight: BLOCKS.get("minecraft:shroomlight"),
    quartz_ore: BLOCKS.get("minecraft:quartz_ore"),
    gold_ore: BLOCKS.get("minecraft:nether_gold_ore"),
    ancient_debris: BLOCKS.get("minecraft:ancient_debris"),
    gravel: BLOCKS.get("minecraft:gravel"),
    lava: BLOCKS.lava,
});

/// RNG positionnel déterministe (splitmix64) pour la dispersion des features.
struct Rng(u64);
impl Rng {
    fn new(wx: i32, wz: i32, seed: u64) -> Self {
        let mut h = seed
            ^ (wx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (wz as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        h ^= h >> 30;
        Rng(h | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// f64 dans [0,1).
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
    /// Entier dans [0,n).
    fn range(&mut self, n: i32) -> i32 {
        (self.next() % n as u64) as i32
    }
    fn chance(&mut self, p: f64) -> bool {
        self.f() < p
    }
}

/// Décore un chunk Nether déjà rempli (terrain). `base_x/base_z` = coin du chunk.
/// Ordre proche de vanilla : surface (par colonne) → minerais → springs → glowstone
/// → végétation → lianes.
pub fn decorate(grid: &mut DimGrid, base_x: i32, base_z: i32, sampler: &ClimateSampler, seed: u64) {
    let nb = &*NB;
    // 1. Surface : bloc de surface du biome (nylium / soul sand / basalt) + patches
    //    gravel/soul_sand selon la surface rule (doc 27 §3), par colonne.
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            let wx = base_x + lx;
            let wz = base_z + lz;
            let bi = nether_biome_index(sampler, wx, 64, wz);
            let mut rng = Rng::new(wx, wz, seed);
            for y in (2..126).rev() {
                if grid.get_world(wx, y, wz) != nb.netherrack {
                    continue;
                }
                if grid.get_world(wx, y + 1, wz) == nb.air {
                    apply_surface(grid, nb, wx, y, wz, bi, &mut rng);
                }
            }
        }
    }
    // Biome dominant du chunk (les features à count/chunk en dépendent).
    let biome = nether_biome_index(sampler, base_x + 8, 64, base_z + 8);
    // basalt_deltas : blobs + deltas de lave + colonnes (avant les minerais, pour
    // que les veines remplissent la netherrack restante).
    if biome == BASALT {
        place_basalt_deltas(grid, nb, base_x, base_z, seed);
    }
    place_ores(grid, nb, base_x, base_z, seed, biome);
    place_springs(grid, nb, base_x, base_z, seed);
    place_glowstone(grid, nb, base_x, base_z, seed);
    place_fire(grid, nb, base_x, base_z, seed);
    place_vegetation(grid, nb, base_x, base_z, seed, biome);
    place_vines(grid, nb, base_x, base_z, seed, biome);
}

/// `patch_fire` + `patch_soul_fire` (doc 27 §5) : 0..5 foyers/chunk. Feu sur le sol ;
/// soul fire si le sol est en soul soil.
fn place_fire(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let air = BLOCKS.air;
    let mut rng = FeatureRng::new(base_x >> 4, base_z >> 4, seed, 0x_F12E);
    let n = rng.range(6); // uniform 0..5
    for _ in 0..n {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let sy = rng.between(4, 124);
        // Premier sol (solide + air au-dessus) sous sy.
        if let Some(fy) = (5..sy)
            .rev()
            .find(|&y| grid.get_world(wx, y, wz) != air && grid.get_world(wx, y + 1, wz) == air)
        {
            let on = if grid.get_world(wx, fy, wz) == nb.soul_soil {
                nb.soul_fire
            } else {
                nb.fire
            };
            grid.set(wx, fy + 1, wz, on);
        }
    }
}

/// Minerais du Nether — veines vanilla (spec §4 du doc 27) : `count`/chunk, `size`
/// de veine et plage Y exacts, via le moteur `feature`. Le biome dominant du chunk
/// pilote les variantes (deltas : plus de quartz/or, ni gravel ni blackstone ;
/// valley : soul_sand).
fn place_ores(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    biome: u16,
) {
    use super::feature::{place_ore, Anchor::*, FeatureRng, HeightRange::*, Ore, Region};
    let region = Region {
        base_x,
        base_z,
        min_y: 0,
        max_y: 128,
    };
    let (cx, cz) = (base_x >> 4, base_z >> 4);
    let netherrack = nb.netherrack;
    let basalt = nb.basalt;
    let blackstone = nb.blackstone;
    let is_rack = move |b: u32| b == netherrack;
    // ancient debris cible la pierre de base du Nether (netherrack/basalt/blackstone).
    let is_base_stone = move |b: u32| b == netherrack || b == basalt || b == blackstone;
    let deltas = biome == BASALT;

    // (block, size, discard, scattered, count, height, salt, target)
    let mut run =
        |block, size, discard, scattered, count: i32, h, salt: u64, tgt: &dyn Fn(u32) -> bool| {
            let mut rng = FeatureRng::new(cx, cz, seed, salt);
            let ore = Ore {
                block,
                size,
                discard_air: discard,
                scattered,
                is_target: tgt,
            };
            place_ore(grid, &mut rng, &region, count, h, &ore);
        };

    let body = Uniform(AboveBottom(10), BelowTop(10));
    run(
        nb.quartz_ore,
        14,
        0.0,
        false,
        if deltas { 32 } else { 16 },
        body,
        0x0_0001,
        &is_rack,
    );
    run(
        nb.gold_ore,
        10,
        0.0,
        false,
        if deltas { 20 } else { 10 },
        body,
        0x0_0002,
        &is_rack,
    );
    run(
        nb.magma,
        33,
        0.0,
        false,
        4,
        Uniform(Absolute(27), Absolute(36)),
        0x0_0003,
        &is_rack,
    );
    if !deltas {
        run(
            BLOCKS.get("minecraft:gravel"),
            33,
            0.0,
            false,
            2,
            Uniform(Absolute(5), Absolute(41)),
            0x0_0004,
            &is_rack,
        );
        run(
            blackstone,
            33,
            0.0,
            false,
            2,
            Uniform(Absolute(5), Absolute(31)),
            0x0_0005,
            &is_rack,
        );
    }
    if biome == SOUL_SAND_VALLEY {
        run(
            nb.soul_sand,
            12,
            0.0,
            false,
            12,
            Uniform(AboveBottom(0), Absolute(31)),
            0x0_0006,
            &is_rack,
        );
    }
    // ancient debris : dispersé, jamais exposé à l'air (discard 1.0).
    run(
        nb.ancient_debris,
        3,
        1.0,
        true,
        1,
        Trapezoid(Absolute(8), Absolute(24)),
        0x0_0007,
        &is_base_stone,
    );
    run(
        nb.ancient_debris,
        2,
        1.0,
        true,
        1,
        Uniform(AboveBottom(8), BelowTop(8)),
        0x0_0008,
        &is_base_stone,
    );
}

/// Surface « sol » (bloc avec de l'air au-dessus) : pose **seulement** le bloc de
/// surface du biome selon la surface rule vanilla (doc 27 §3). La végétation est
/// posée par les passes chunk-level (`place_vegetation`).
fn apply_surface(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    y: i32,
    wz: i32,
    bi: u16,
    rng: &mut Rng,
) {
    match bi {
        SOUL_SAND_VALLEY => {
            // soul_sand/soul_soil ; gravel sous Y30 (surface rule).
            let s = if y < 30 && rng.chance(0.5) {
                nb.gravel
            } else if rng.chance(0.85) {
                nb.soul_sand
            } else {
                nb.soul_soil
            };
            grid.set(wx, y, wz, s);
        }
        CRIMSON => grid.set(wx, y, wz, nb.crimson_nylium),
        WARPED => grid.set(wx, y, wz, nb.warped_nylium),
        BASALT => {
            // basalt ; gravel au-dessus de Y30 (surface rule).
            let s = if y > 30 { nb.gravel } else { nb.basalt };
            grid.set(wx, y, wz, s);
        }
        _ => {
            // nether_wastes : netherrack ; patches soul_sand/gravel vers Y30-32.
            if (30..=32).contains(&y) && rng.chance(0.12) {
                let s = if rng.chance(0.5) {
                    nb.soul_sand
                } else {
                    nb.gravel
                };
                grid.set(wx, y, wz, s);
            }
        }
    }
}

/// Itère les sols (bloc solide avec air au-dessus) sur `count` tentatives par tranche
/// de 16 blocs de hauteur (vanilla `count_on_every_layer`) et appelle `f` sur chaque.
fn on_every_layer(
    grid: &mut DimGrid,
    base_x: i32,
    base_z: i32,
    rng: &mut FeatureRng,
    count: i32,
    mut f: impl FnMut(&mut DimGrid, i32, i32, i32, &mut FeatureRng),
) {
    let air = BLOCKS.air;
    for layer in 0..8 {
        // 128/16
        let y0 = layer * 16;
        for _ in 0..count {
            let wx = base_x + rng.range(16);
            let wz = base_z + rng.range(16);
            // Cherche un sol (solide + air au-dessus) dans cette tranche.
            let mut floor = None;
            for y in (y0 + 1..y0 + 16).rev() {
                if grid.get_world(wx, y, wz) != air && grid.get_world(wx, y + 1, wz) == air {
                    floor = Some(y);
                    break;
                }
            }
            if let Some(fy) = floor {
                f(grid, wx, fy, wz, rng);
            }
        }
    }
}

/// Végétation des forêts du Nether (doc 27 §5) via `count_on_every_layer` pondéré +
/// fungi géants ; nether_sprouts en warped ; crimson roots en valley.
fn place_vegetation(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    biome: u16,
) {
    let (cx, cz) = (base_x >> 4, base_z >> 4);
    let crimson_fungus = BLOCKS.get("minecraft:crimson_fungus");
    let warped_fungus = BLOCKS.get("minecraft:warped_fungus");
    match biome {
        CRIMSON => {
            // crimson_forest_vegetation : roots 87 / crimson_fungus 11 / warped_fungus 1.
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_C012);
            on_every_layer(grid, base_x, base_z, &mut rng, 6, |g, wx, fy, wz, r| {
                if g.get_world(wx, fy, wz) != nb.crimson_nylium {
                    return;
                }
                let w = r.range(99);
                let b = if w < 87 {
                    nb.crimson_roots
                } else if w < 98 {
                    crimson_fungus
                } else {
                    warped_fungus
                };
                g.set(wx, fy + 1, wz, b);
            });
            // crimson_fungi : fungi géants (every_layer 8).
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_C0F9);
            on_every_layer(grid, base_x, base_z, &mut rng, 8, |g, wx, fy, wz, r| {
                if g.get_world(wx, fy, wz) == nb.crimson_nylium {
                    grow_fungus(g, nb, wx, fy + 1, wz, true, r);
                }
            });
        }
        WARPED => {
            // warped_forest_vegetation : warped_roots 85 / warped_fungus 13 / crimson 1 / crimson_fungus 1.
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_7012);
            on_every_layer(grid, base_x, base_z, &mut rng, 5, |g, wx, fy, wz, r| {
                if g.get_world(wx, fy, wz) != nb.warped_nylium {
                    return;
                }
                let w = r.range(100);
                let b = if w < 85 {
                    nb.warped_roots
                } else if w < 98 {
                    warped_fungus
                } else if w < 99 {
                    nb.crimson_roots
                } else {
                    crimson_fungus
                };
                g.set(wx, fy + 1, wz, b);
            });
            // nether_sprouts (every_layer 4).
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_7059);
            on_every_layer(grid, base_x, base_z, &mut rng, 4, |g, wx, fy, wz, _| {
                if g.get_world(wx, fy, wz) == nb.warped_nylium
                    && g.get_world(wx, fy + 1, wz) == nb.air
                {
                    g.set(wx, fy + 1, wz, nb.nether_sprouts);
                }
            });
            // warped_fungi géants (every_layer 8).
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_70F9);
            on_every_layer(grid, base_x, base_z, &mut rng, 8, |g, wx, fy, wz, r| {
                if g.get_world(wx, fy, wz) == nb.warped_nylium {
                    grow_fungus(g, nb, wx, fy + 1, wz, false, r);
                }
            });
        }
        SOUL_SAND_VALLEY => {
            // patch_crimson_roots : touffes sur le sol.
            let mut rng = FeatureRng::new(cx, cz, seed, 0x_5012);
            on_every_layer(grid, base_x, base_z, &mut rng, 3, |g, wx, fy, wz, r| {
                if g.get_world(wx, fy + 1, wz) == nb.air && r.range(2) == 0 {
                    g.set(wx, fy + 1, wz, nb.crimson_roots);
                }
            });
        }
        _ => {}
    }
}

/// Lianes : twisting_vines (montantes, warped) et weeping_vines (pendantes, crimson),
/// count 10/chunk chacune (doc 27 §5).
fn place_vines(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    biome: u16,
) {
    let (cx, cz) = (base_x >> 4, base_z >> 4);
    let air = BLOCKS.air;
    if biome == WARPED {
        let mut rng = FeatureRng::new(cx, cz, seed, 0x_71E5);
        on_every_layer(grid, base_x, base_z, &mut rng, 10, |g, wx, fy, wz, r| {
            let h = r.between(1, 8);
            for i in 1..=h {
                if g.get_world(wx, fy + i, wz) != air {
                    break;
                }
                g.set(wx, fy + i, wz, nb.twisting_vines);
            }
        });
    }
    if biome == CRIMSON {
        // weeping vines : pendent des plafonds (solide avec air en-dessous).
        let mut rng = FeatureRng::new(cx, cz, seed, 0x_C1E5);
        for _ in 0..10 {
            let wx = base_x + rng.range(16);
            let wz = base_z + rng.range(16);
            let sy = rng.between(20, 110);
            if let Some(cy) = find_ceiling(grid, nb, wx, sy, wz) {
                let h = rng.between(1, 8);
                for i in 1..=h {
                    if grid.get_world(wx, cy - i, wz) != air {
                        break;
                    }
                    grid.set(wx, cy - i, wz, nb.weeping_vines);
                }
            }
        }
    }
}

/// Cherche un plafond (netherrack avec air en-dessous) à/sous `from_y`.
fn find_ceiling(grid: &DimGrid, nb: &NetherBlocks, wx: i32, from_y: i32, wz: i32) -> Option<i32> {
    (4..from_y).rev().find(|&y| {
        grid.get_world(wx, y, wz) == nb.netherrack && grid.get_world(wx, y - 1, wz) == nb.air
    })
}

/// Amas de glowstone (`glowstone_blob`) pendant aux plafonds : count 10/chunk + extra.
fn place_glowstone(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let (cx, cz) = (base_x >> 4, base_z >> 4);
    let mut rng = FeatureRng::new(cx, cz, seed, 0x_610B);
    let count = 10 + rng.range(10).min(rng.range(10)); // 10 + glowstone_extra biased bas
    for _ in 0..count {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let sy = rng.between(8, 122);
        if let Some(cy) = find_ceiling(grid, nb, wx, sy, wz) {
            grow_glowstone_blob(grid, nb, wx, cy - 1, wz, &mut rng);
        }
    }
}

/// Blob de glowstone : sphère partielle accrochée sous le plafond (rayon ~2).
fn grow_glowstone_blob(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    top_y: i32,
    wz: i32,
    rng: &mut FeatureRng,
) {
    let air = BLOCKS.air;
    for dy in 0..=3 {
        let r = 2 - dy.min(2);
        for dx in -r..=r {
            for dz in -r..=r {
                if dx * dx + dz * dz > r * r {
                    continue;
                }
                let (x, y, z) = (wx + dx, top_y - dy, wz + dz);
                if grid.get_world(x, y, z) == air && rng.range(3) != 0 {
                    grid.set(x, y, z, nb.glowstone);
                }
            }
        }
    }
}

/// Fungus géant (`HugeFungusFeature`) : tige (4..13) + chapeau dôme de wart block +
/// shroomlight semé. `base_y` = case d'air juste au-dessus du nylium.
fn grow_fungus(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    base_y: i32,
    wz: i32,
    crimson: bool,
    rng: &mut FeatureRng,
) {
    let (stem, wart) = if crimson {
        (nb.crimson_stem, nb.nether_wart_block)
    } else {
        (nb.warped_stem, nb.warped_wart_block)
    };
    let h = rng.between(4, 13);
    for i in 0..h {
        if grid.get_world(wx, base_y + i, wz) != nb.air {
            return; // obstrué : pas de tige tronquée
        }
        grid.set(wx, base_y + i, wz, stem);
    }
    // Chapeau : dôme de wart block sur 3 couches (rayon 2 → 1 → 1 au sommet).
    let cap = base_y + h;
    for dy in 0..=2i32 {
        let r = if dy == 0 { 2 } else { 1 };
        for dx in -r..=r {
            for dz in -r..=r {
                if dx * dx + dz * dz <= r * r + 1 {
                    grid.set(wx + dx, cap + dy, wz + dz, wart);
                }
            }
        }
    }
    // Shroomlight semés dans le chapeau (décor lumineux).
    for _ in 0..rng.between(1, 4) {
        let dx = rng.between(-2, 2);
        let dz = rng.between(-2, 2);
        grid.set(wx + dx, cap, wz + dz, nb.shroomlight);
    }
}

/// Génération de `basalt_deltas` (doc 27 §5) : blobs basalt/blackstone qui remplacent
/// la netherrack, deltas de lave (mares à bord de magma) et colonnes de basalte.
fn place_basalt_deltas(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let (cx, cz) = (base_x >> 4, base_z >> 4);
    let netherrack = nb.netherrack;

    // basalt_blobs (75) + blackstone_blobs (25) : sphères remplaçant la netherrack.
    let mut rng = FeatureRng::new(cx, cz, seed, 0x_BA10);
    for (count, state) in [(75, nb.basalt), (25, nb.blackstone)] {
        for _ in 0..count {
            let wx = base_x + rng.range(16);
            let wz = base_z + rng.range(16);
            let wy = rng.between(2, 125);
            let r = rng.between(3, 7);
            replace_blob(grid, wx, wy, wz, r, netherrack, state);
        }
    }

    // delta : mares de lave (rayon 3-7) à bord de magma, sur les sols. every_layer 40.
    let mut rng = FeatureRng::new(cx, cz, seed, 0x_DE17);
    on_every_layer(grid, base_x, base_z, &mut rng, 40, |g, wx, fy, wz, r| {
        let size = r.between(3, 7);
        let rim = r.between(0, 2);
        for dx in -size..=size {
            for dz in -size..=size {
                let d = dx * dx + dz * dz;
                if d > size * size {
                    continue;
                }
                // bord (rim) = magma, intérieur = lave (surface remplacée).
                let block = if d > (size - rim) * (size - rim) {
                    nb.magma
                } else {
                    nb.lava
                };
                if g.get_world(wx + dx, fy, wz + dz) != BLOCKS.air {
                    g.set(wx + dx, fy, wz + dz, block);
                }
            }
        }
    });

    // colonnes de basalte : small (h1-4, reach1) ev4 + large (h5-10, reach2-3) ev2.
    let mut rng = FeatureRng::new(cx, cz, seed, 0x_C011);
    on_every_layer(grid, base_x, base_z, &mut rng, 4, |g, wx, fy, wz, r| {
        basalt_column(g, nb, wx, fy, wz, r.between(1, 4), 1);
    });
    let mut rng = FeatureRng::new(cx, cz, seed, 0x_C012);
    on_every_layer(grid, base_x, base_z, &mut rng, 2, |g, wx, fy, wz, r| {
        basalt_column(g, nb, wx, fy, wz, r.between(5, 10), r.between(2, 3));
    });
}

/// Remplace la `target` par `state` dans une sphère de rayon `radius`.
fn replace_blob(
    grid: &mut DimGrid,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
    target: u32,
    state: u32,
) {
    let r2 = radius * radius;
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dy * dy + dz * dz > r2 {
                    continue;
                }
                let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                if grid.get_world(x, y, z) == target {
                    grid.set(x, y, z, state);
                }
            }
        }
    }
}

/// Colonne de basalte montant du sol `fy` (rayon de base `reach`, hauteur `height`).
fn basalt_column(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    fy: i32,
    wz: i32,
    height: i32,
    reach: i32,
) {
    let air = BLOCKS.air;
    let r2 = reach * reach;
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            if dx * dx + dz * dz > r2 {
                continue;
            }
            for h in 1..=height {
                let (x, y, z) = (wx + dx, fy + h, wz + dz);
                if grid.get_world(x, y, z) != air {
                    break;
                }
                grid.set(x, y, z, nb.basalt);
            }
        }
    }
}

/// Springs de lave (doc 27 §5) : `spring_nether_open` (count 8, source exposée à
/// l'air) + `spring_nether_closed` (count 16, poche enclavée dans la netherrack).
fn place_springs(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let air = BLOCKS.air;
    let is_air_exposed = |g: &DimGrid, x: i32, y: i32, z: i32| {
        g.get_world(x + 1, y, z) == air
            || g.get_world(x - 1, y, z) == air
            || g.get_world(x, y, z + 1) == air
            || g.get_world(x, y, z - 1) == air
            || g.get_world(x, y - 1, z) == air
    };
    // open : 8 sources de lave dans une paroi/sol exposé.
    let mut rng = FeatureRng::new(base_x >> 4, base_z >> 4, seed, 0x_5A0E);
    for _ in 0..8 {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let y = rng.between(6, 120);
        if grid.get_world(wx, y, wz) == nb.netherrack
            && grid.get_world(wx, y + 1, wz) == nb.netherrack
            && is_air_exposed(grid, wx, y, wz)
        {
            grid.set(wx, y, wz, nb.lava);
        }
    }
    // closed : 16 poches de lave enclavées (aucune face d'air).
    let mut rng = FeatureRng::new(base_x >> 4, base_z >> 4, seed, 0x_5A0C);
    for _ in 0..16 {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let y = rng.between(6, 120);
        if grid.get_world(wx, y, wz) == nb.netherrack
            && !is_air_exposed(grid, wx, y, wz)
            && grid.get_world(wx, y + 1, wz) == nb.netherrack
        {
            grid.set(wx, y, wz, nb.lava);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_per_position() {
        let mut a = Rng::new(10, -7, 42);
        let mut b = Rng::new(10, -7, 42);
        assert_eq!(a.next(), b.next());
        let mut c = Rng::new(11, -7, 42);
        assert_ne!(Rng::new(10, -7, 42).next(), c.next());
    }

    #[test]
    fn decorate_changes_surface_and_adds_features() {
        use crate::world::worldgen::density;
        let nb = &*NB;
        let seed = 42u64;
        let router = density::build_nether(seed);
        let sampler = ClimateSampler::from_router(&router);
        // Cherche des chunks couvrant des biomes différents, décore, vérifie qu'au
        // moins un bloc de surface de biome (nylium/soul sand/basalt) apparaît, et
        // que des minerais sont dispersés.
        let mut nylium_or_special = 0usize;
        let mut ores = 0usize;
        for cx in -6..6 {
            for cz in -6..6 {
                let (bx, bz) = (cx * 16, cz * 16);
                let mut grid = DimGrid::test_floor(bx, bz, 0, 128, nb.netherrack, 70);
                decorate(&mut grid, bx, bz, &sampler, seed);
                nylium_or_special += grid.count(nb.crimson_nylium)
                    + grid.count(nb.warped_nylium)
                    + grid.count(nb.soul_sand)
                    + grid.count(nb.basalt);
                ores += grid.count(nb.quartz_ore) + grid.count(nb.gold_ore);
            }
        }
        assert!(
            nylium_or_special > 0,
            "surfaces de biome attendues (nylium/soul sand/basalt)"
        );
        assert!(ores > 0, "minerais Nether attendus");
    }

    #[test]
    fn forests_get_vegetation_and_glowstone() {
        use crate::world::worldgen::density;
        let nb = &*NB;
        let router = density::build_nether(42);
        let sampler = ClimateSampler::from_router(&router);
        // Sur une large zone (plusieurs biomes), on doit voir de la végétation de
        // forêt (roots/stems) et des amas de glowstone.
        let (mut veg, mut glow, mut lava) = (0usize, 0usize, 0usize);
        for cx in -8..8 {
            for cz in -8..8 {
                let (bx, bz) = (cx * 16, cz * 16);
                // Caverne : sol jusqu'à y60, vide, toit à partir de y100 (plafonds
                // pour glowstone/lianes).
                let mut grid = DimGrid::test_cavern(bx, bz, 128, nb.netherrack, 60, 100);
                decorate(&mut grid, bx, bz, &sampler, 42);
                veg += grid.count(nb.crimson_roots)
                    + grid.count(nb.warped_roots)
                    + grid.count(nb.crimson_stem)
                    + grid.count(nb.warped_stem)
                    + grid.count(nb.nether_sprouts);
                glow += grid.count(nb.glowstone);
                lava += grid.count(nb.lava);
            }
        }
        assert!(
            veg > 0,
            "végétation de forêt attendue (roots/stems/sprouts)"
        );
        assert!(glow > 0, "amas de glowstone attendus");
        assert!(lava > 0, "springs de lave attendus");
    }

    #[test]
    fn basalt_deltas_get_blobs_and_lava() {
        let nb = &*NB;
        let mut grid = DimGrid::test_cavern(0, 0, 128, nb.netherrack, 60, 100);
        place_basalt_deltas(&mut grid, nb, 0, 0, 42);
        assert!(
            grid.count(nb.basalt) > 0,
            "blobs/colonnes de basalte attendus"
        );
        assert!(
            grid.count(nb.blackstone) > 0,
            "blobs de blackstone attendus"
        );
        assert!(grid.count(nb.lava) > 0, "deltas de lave attendus");
    }
}

#[cfg(test)]
mod ore_probe {
    use super::*;
    use crate::world::worldgen::density;
    #[test]
    fn ore_counts_in_vanilla_ballpark() {
        let nb = &*NB;
        let router = density::build_nether(42);
        let sampler = ClimateSampler::from_router(&router);
        let (mut q, mut g, mut m, mut d) = (0usize, 0usize, 0usize, 0usize);
        let chunks = 20;
        for cx in 0..chunks {
            let (bx, bz) = (cx * 16, 0);
            let mut grid = DimGrid::test_floor(bx, bz, 0, 128, nb.netherrack, 120);
            decorate(&mut grid, bx, bz, &sampler, 42);
            q += grid.count(nb.quartz_ore);
            g += grid.count(nb.gold_ore);
            m += grid.count(nb.magma);
            d += grid.count(nb.ancient_debris);
        }
        let n = chunks as usize;
        println!(
            "[ore/chunk] quartz={} gold={} magma={} debris={}",
            q / n,
            g / n,
            m / n,
            d / n
        );
        // quartz ~16 veines*size14 -> ~150-400 blocs/chunk ; debris rare mais présent.
        assert!(
            q / n > 80 && q / n < 1200,
            "quartz/chunk hors plage: {}",
            q / n
        );
        assert!(m / n > 20, "magma (size33) attendu: {}", m / n);
    }
}
