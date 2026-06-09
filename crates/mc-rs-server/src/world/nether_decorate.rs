//! Décoration du Nether par biome : surfaces (nylium / soul sand / basalt), petite
//! végétation (roots, sprouts, vines), fungi crimson/warped, amas de glowstone aux
//! plafonds, et minerais (quartz, or, ancient debris).
//!
//! Appliqué après le terrain (`dim_noise`) via le callback `decorate`. Le biome de
//! chaque colonne vient du climat multi-noise (`nether_generator::nether_biome_index`),
//! la dispersion est déterministe (hash position+seed) donc rejouable.
//!
//! Réf : `new-docs/27-DIMENSIONS-GENERATION-SPEC.md` (données vanilla mcmeta). Les
//! **minerais** sont des vraies veines (`count`/chunk + `size` + plage Y, moteur
//! `feature`). Surface/végétation/fungi : en cours de portage fidèle.

use std::sync::LazyLock;

use super::block_registry::BLOCKS;
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
pub fn decorate(grid: &mut DimGrid, base_x: i32, base_z: i32, sampler: &ClimateSampler, seed: u64) {
    let nb = &*NB;
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            let wx = base_x + lx;
            let wz = base_z + lz;
            let bi = nether_biome_index(sampler, wx, 64, wz);
            let mut rng = Rng::new(wx, wz, seed);

            // Scan de haut en bas : on traite chaque frontière solide/air.
            for y in (2..126).rev() {
                let here = grid.get_world(wx, y, wz);
                if here != nb.netherrack {
                    continue;
                }
                let above = grid.get_world(wx, y + 1, wz);
                let below = grid.get_world(wx, y - 1, wz);
                if above == nb.air {
                    apply_floor(grid, nb, wx, y, wz, bi, &mut rng);
                }
                if below == nb.air {
                    apply_ceiling(grid, nb, wx, y, wz, bi, &mut rng);
                }
            }
        }
    }
    // Minerais : veines `count`/chunk (biome dominant échantillonné au centre).
    let chunk_biome = nether_biome_index(sampler, base_x + 8, 64, base_z + 8);
    place_ores(grid, nb, base_x, base_z, seed, chunk_biome);
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

/// Surface « sol » (bloc avec de l'air au-dessus) : remplace par le bloc de surface
/// du biome + pose éventuellement de la végétation / un fungus au-dessus.
fn apply_floor(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    y: i32,
    wz: i32,
    bi: u16,
    rng: &mut Rng,
) {
    let top = y + 1; // case d'air juste au-dessus du sol
    match bi {
        SOUL_SAND_VALLEY => {
            let s = if rng.chance(0.7) {
                nb.soul_sand
            } else {
                nb.soul_soil
            };
            grid.set(wx, y, wz, s);
            if rng.chance(0.02) {
                grid.set(wx, top, wz, nb.soul_fire);
            } else if rng.chance(0.05) {
                grid.set(wx, top, wz, nb.nether_sprouts);
            }
        }
        CRIMSON => {
            grid.set(wx, y, wz, nb.crimson_nylium);
            if rng.chance(0.012) {
                grow_fungus(grid, nb, wx, top, wz, true, rng);
            } else if rng.chance(0.18) {
                grid.set(wx, top, wz, nb.crimson_roots);
            } else if rng.chance(0.08) {
                grid.set(wx, top, wz, nb.nether_sprouts);
            } else if rng.chance(0.02) {
                grid.set(wx, top, wz, BLOCKS.get("minecraft:crimson_fungus"));
            }
        }
        WARPED => {
            grid.set(wx, y, wz, nb.warped_nylium);
            if rng.chance(0.01) {
                grow_fungus(grid, nb, wx, top, wz, false, rng);
            } else if rng.chance(0.18) {
                grid.set(wx, top, wz, nb.warped_roots);
            } else if rng.chance(0.08) {
                grid.set(wx, top, wz, nb.nether_sprouts);
            } else if rng.chance(0.03) {
                // pousse verticale de twisting_vines (1..4) depuis le sol
                let h = 1 + rng.range(4);
                for i in 0..h {
                    if grid.get_world(wx, top + i, wz) != nb.air {
                        break;
                    }
                    grid.set(wx, top + i, wz, nb.twisting_vines);
                }
            } else if rng.chance(0.02) {
                grid.set(wx, top, wz, BLOCKS.get("minecraft:warped_fungus"));
            }
        }
        BASALT => {
            let s = if rng.chance(0.7) {
                nb.basalt
            } else {
                nb.blackstone
            };
            grid.set(wx, y, wz, s);
            if rng.chance(0.04) {
                grid.set(wx, y, wz, nb.magma);
            }
            // petite colonne de basalte (delta) qui dépasse du sol
            if rng.chance(0.01) {
                let h = 1 + rng.range(4);
                for i in 1..=h {
                    if grid.get_world(wx, y + i, wz) != nb.air {
                        break;
                    }
                    grid.set(wx, y + i, wz, nb.basalt);
                }
            }
        }
        _ => {
            // nether_wastes : netherrack inchangé, feu occasionnel
            if rng.chance(0.006) {
                grid.set(wx, top, wz, nb.fire);
            }
        }
    }
}

/// Surface « plafond » (bloc avec de l'air en-dessous) : amas de glowstone qui pend
/// (tous biomes) ou weeping vines en crimson forest.
fn apply_ceiling(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    y: i32,
    wz: i32,
    bi: u16,
    rng: &mut Rng,
) {
    if rng.chance(0.006) {
        // Amas de glowstone : le bloc plafond + une petite grappe vers le bas.
        grid.set(wx, y, wz, nb.glowstone);
        let drop = 1 + rng.range(3);
        for i in 1..=drop {
            if grid.get_world(wx, y - i, wz) != nb.air {
                break;
            }
            if rng.chance(0.6) {
                grid.set(wx, y - i, wz, nb.glowstone);
            }
        }
    } else if bi == CRIMSON && rng.chance(0.04) {
        // weeping vines qui pendent du plafond (1..5)
        let h = 1 + rng.range(5);
        for i in 1..=h {
            if grid.get_world(wx, y - i, wz) != nb.air {
                break;
            }
            grid.set(wx, y - i, wz, nb.weeping_vines);
        }
    }
}

/// Fungus géant compact : tige (4..9) + chapeau de wart block + shroomlight.
fn grow_fungus(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    wx: i32,
    base_y: i32,
    wz: i32,
    crimson: bool,
    rng: &mut Rng,
) {
    let (stem, wart) = if crimson {
        (nb.crimson_stem, nb.nether_wart_block)
    } else {
        (nb.warped_stem, nb.warped_wart_block)
    };
    let h = 4 + rng.range(6); // 4..9
                              // tige
    for i in 0..h {
        if grid.get_world(wx, base_y + i, wz) != nb.air {
            return; // obstrué : on annule (pas de tige flottante tronquée bizarre)
        }
        grid.set(wx, base_y + i, wz, stem);
    }
    // chapeau : blob de wart block 3×3 (+ centre une couche plus haut)
    let cap = base_y + h;
    for dx in -1..=1i32 {
        for dz in -1..=1i32 {
            grid.set(wx + dx, cap, wz + dz, wart);
            if dx == 0 && dz == 0 {
                grid.set(wx, cap + 1, wz, wart);
            }
        }
    }
    // quelques shroomlight sous le chapeau
    if rng.chance(0.6) {
        grid.set(wx + 1, cap - 1, wz, nb.shroomlight);
    }
    if rng.chance(0.6) {
        grid.set(wx - 1, cap - 1, wz, nb.shroomlight);
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
