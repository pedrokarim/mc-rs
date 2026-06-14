//! Décoration du Nether par biome — **portage fidèle de Nukkit** (serveur Bedrock),
//! appliquée après le terrain (`dim_noise`) via le callback `decorate`.
//!
//! Réf : `.reference/Nukkit/` (`Nether.java`, `biome/impl/nether/*`,
//! `populator/impl/*`). Les densités sont celles de **Bedrock** (Nukkit), pas les
//! données Java de mcmeta (qui divergent). Ordre proche de Nukkit : sol + surface par
//! biome (soul sand valley = colonne soul_sand, basalt deltas = basalt, forêts =
//! nylium en couverture) ; minerais globaux (`OreType`) + minerais de biome
//! (soul_soil / blackstone) ; feu (netherrack 1/8 chunk), lave de surface (~95%),
//! glowstone (1/9) ; décoration de biome (fungi, tapis de forêt, lianes, deltas).
//!
//! La dispersion est déterministe (hash position+seed), donc rejouable (positions ≠
//! Java mais statistiques identiques).

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
    crimson_fungus: u32,
    warped_fungus: u32,
    red_mushroom: u32,
    brown_mushroom: u32,
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
    crimson_fungus: BLOCKS.get("minecraft:crimson_fungus"),
    warped_fungus: BLOCKS.get("minecraft:warped_fungus"),
    red_mushroom: BLOCKS.get("minecraft:red_mushroom"),
    brown_mushroom: BLOCKS.get("minecraft:brown_mushroom"),
});

/// Décore un chunk Nether déjà rempli (terrain). `base_x/base_z` = coin du chunk.
pub fn decorate(grid: &mut DimGrid, base_x: i32, base_z: i32, sampler: &ClimateSampler, seed: u64) {
    let nb = &*NB;
    // 1. Sol + couverture par biome, par colonne (Nukkit getGroundId/getSurfaceId).
    apply_ground_and_surface(grid, nb, base_x, base_z, sampler);

    // Biome du centre du chunk (Nukkit décore selon le biome en (7,7)).
    let biome = nether_biome_index(sampler, base_x + 8, 64, base_z + 8);

    // 2. Minerais globaux (remplacent la netherrack restante) + minerais de biome.
    place_ores(grid, nb, base_x, base_z, seed);
    place_biome_ores(grid, nb, base_x, base_z, seed, biome);

    // 3. Feu (netherrack, 1/8 chunk), lave de surface (~95% chunk), glowstone (1/9).
    place_nether_fire(
        grid,
        base_x,
        base_z,
        seed,
        nb.fire,
        nb.netherrack,
        true,
        0x_F12E,
    );
    place_lava(grid, nb, base_x, base_z, seed);
    place_glowstone(grid, nb, base_x, base_z, seed, biome);

    // 4. Décoration spécifique au biome du chunk.
    match biome {
        CRIMSON => {
            place_fungi(grid, nb, base_x, base_z, seed, true);
            place_forest_ground(grid, nb, base_x, base_z, seed, true);
            place_weeping_vines(grid, nb, base_x, base_z, seed);
        }
        WARPED => {
            place_fungi(grid, nb, base_x, base_z, seed, false);
            place_forest_ground(grid, nb, base_x, base_z, seed, false);
            place_twisting_vines(grid, nb, base_x, base_z, seed);
        }
        SOUL_SAND_VALLEY => {
            place_nether_fire(
                grid,
                base_x,
                base_z,
                seed,
                nb.soul_fire,
                nb.soul_soil,
                false,
                0x_50F1,
            );
            place_ssv_ground(grid, nb, base_x, base_z, seed);
        }
        BASALT => place_basalt_deltas(grid, nb, base_x, base_z, seed),
        _ => {}
    }
}

/// Chunk-RNG positionnel (FeatureRng) — `salt` distingue chaque populator.
fn crng(base_x: i32, base_z: i32, seed: u64, salt: u64) -> FeatureRng {
    FeatureRng::new(base_x >> 4, base_z >> 4, seed, salt)
}

/// Ys d'air dont le bloc juste en dessous satisfait `is_ground` (sols exposés).
fn floors(grid: &DimGrid, wx: i32, wz: i32, is_ground: impl Fn(u32) -> bool) -> Vec<i32> {
    let air = BLOCKS.air;
    (2..127)
        .filter(|&y| grid.get_world(wx, y, wz) == air && is_ground(grid.get_world(wx, y - 1, wz)))
        .collect()
}

/// Sol et couverture par biome (réf Nukkit `CoveredBiome` ground/surface).
/// soul_sand_valley → colonne entière soul_sand ; basalt_deltas → basalt ;
/// crimson/warped → nylium sur chaque face de sol ; wastes → netherrack inchangé.
fn apply_ground_and_surface(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    sampler: &ClimateSampler,
) {
    let air = BLOCKS.air;
    for lx in 0..16i32 {
        for lz in 0..16i32 {
            let wx = base_x + lx;
            let wz = base_z + lz;
            let biome = nether_biome_index(sampler, wx, 64, wz);
            if biome == WASTES {
                continue;
            }
            for y in 1..127 {
                if grid.get_world(wx, y, wz) != nb.netherrack {
                    continue;
                }
                let above_air = grid.get_world(wx, y + 1, wz) == air;
                match biome {
                    SOUL_SAND_VALLEY => grid.set(wx, y, wz, nb.soul_sand),
                    BASALT => grid.set(wx, y, wz, nb.basalt),
                    CRIMSON if above_air => grid.set(wx, y, wz, nb.crimson_nylium),
                    WARPED if above_air => grid.set(wx, y, wz, nb.warped_nylium),
                    _ => {}
                }
            }
        }
    }
}

/// Minerais globaux du Nether — veines **Bedrock** (réf Nukkit `Nether.java` +
/// `OreType`) : quartz 16×24, or 10×16, magma 4×64, gravel 2×64, soul_sand 12×23,
/// lave 1×16, ancient_debris 2×3 + 3×2. Tout remplace la netherrack, sans
/// discard-on-air, mêmes valeurs pour tous les biomes.
fn place_ores(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let netherrack = nb.netherrack;
    let is_rack = move |b: u32| b == netherrack;
    let mut run = |block, size, count, ymin, ymax, salt| {
        run_ore(
            grid, base_x, base_z, seed, block, size, count, ymin, ymax, salt, &is_rack,
        );
    };
    run(nb.quartz_ore, 24, 16, 10, 117, 0x01);
    run(nb.gold_ore, 16, 10, 10, 117, 0x02);
    run(nb.magma, 64, 4, 26, 33, 0x03);
    run(nb.gravel, 64, 2, 5, 105, 0x04);
    run(nb.soul_sand, 23, 12, 0, 105, 0x05);
    run(nb.lava, 16, 1, 0, 31, 0x06); // poches de lave
    run(nb.ancient_debris, 3, 2, 8, 23, 0x07);
    run(nb.ancient_debris, 2, 3, 8, 119, 0x08);
}

/// Minerais propres au biome (réf Nukkit biome `PopulatorOre`) : soul_soil dans le
/// soul_sand (3×128), blackstone dans le basalt (4×128).
fn place_biome_ores(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    biome: u16,
) {
    match biome {
        SOUL_SAND_VALLEY => {
            let soul_sand = nb.soul_sand;
            let tgt = move |b: u32| b == soul_sand;
            run_ore(
                grid,
                base_x,
                base_z,
                seed,
                nb.soul_soil,
                128,
                3,
                0,
                128,
                0x11,
                &tgt,
            );
        }
        BASALT => {
            let basalt = nb.basalt;
            let tgt = move |b: u32| b == basalt;
            run_ore(
                grid,
                base_x,
                base_z,
                seed,
                nb.blackstone,
                128,
                4,
                0,
                128,
                0x12,
                &tgt,
            );
        }
        _ => {}
    }
}

/// Pose une veine `OreType` (réf Nukkit) : `count` clusters de taille `size` entre
/// `ymin..ymax`, remplaçant les blocs cible `tgt`.
#[allow(clippy::too_many_arguments)]
fn run_ore(
    grid: &mut DimGrid,
    base_x: i32,
    base_z: i32,
    seed: u64,
    block: u32,
    size: i32,
    count: i32,
    ymin: i32,
    ymax: i32,
    salt: u64,
    tgt: &dyn Fn(u32) -> bool,
) {
    use super::feature::{place_ore, Anchor::Absolute, HeightRange::Uniform, Ore, Region};
    let region = Region {
        base_x,
        base_z,
        min_y: 0,
        max_y: 128,
    };
    let mut rng = crng(base_x, base_z, seed, salt);
    let ore = Ore {
        block,
        size,
        discard_air: 0.0,
        scattered: false,
        is_target: tgt,
    };
    place_ore(
        grid,
        &mut rng,
        &region,
        count,
        Uniform(Absolute(ymin), Absolute(ymax)),
        &ore,
    );
}

/// Feu (réf Nukkit `PopulatorNetherFire`) : sur `replace`, 1/8 chunk si `gated`,
/// 4-7 foyers posés sur chaque face de sol. En soul sand valley : soul fire sur
/// soul_soil, sans gate.
#[allow(clippy::too_many_arguments)]
fn place_nether_fire(
    grid: &mut DimGrid,
    base_x: i32,
    base_z: i32,
    seed: u64,
    fire: u32,
    replace: u32,
    gated: bool,
    salt: u64,
) {
    let mut rng = crng(base_x, base_z, seed, salt);
    if gated && rng.range(8) < 7 {
        return;
    }
    let amount = rng.range(4) + 4; // 4..7
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for fy in floors(grid, wx, wz, |b| b == replace) {
            if fy > 1 {
                grid.set(wx, fy, wz, fire);
            }
        }
    }
}

/// Lave de surface (réf Nukkit `PopulatorLava`, randomAmount 2) : ≈95% des chunks,
/// 0-3 sources au plus haut bloc d'air. (Propagation laissée à la physique runtime.)
fn place_lava(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let air = BLOCKS.air;
    let mut rng = crng(base_x, base_z, seed, 0x_1A7A);
    if rng.range(101) < 5 {
        return;
    }
    let amount = rng.range(4); // 0..3
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let mut y = -1;
        for yy in (1..=120).rev() {
            if grid.get_world(wx, yy, wz) == air {
                y = yy;
                break;
            }
        }
        if y > 0 && grid.get_world(wx, y, wz) == air {
            grid.set(wx, y, wz, nb.lava);
        }
    }
}

/// Amas de glowstone — **portage fidèle de Nukkit** `PopulatorGlowStone` (Bedrock) :
/// seulement ~1 chunk sur 9 en porte un (jamais en soul sand valley), et alors **un
/// seul** blob accroché sous le plafond (count 20-42 blocs, taille 3-6).
fn place_glowstone(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    biome: u16,
) {
    let mut rng = crng(base_x, base_z, seed, 0x_610B);
    if rng.range(9) < 8 || biome == SOUL_SAND_VALLEY {
        return;
    }
    let x = base_x + rng.range(16);
    let z = base_z + rng.range(16);
    let mut y = -1;
    for yy in (0..=120).rev() {
        if grid.get_world(x, yy, z) == nb.air {
            y = yy;
            break;
        }
    }
    if y <= 0 {
        return;
    }
    y += 1;
    let vertical = rng.range(2) == 0;
    let mut size = rng.between(3, 6);
    let count = rng.between(20 + size * 2, 30 + size * 2);
    if vertical && size > 3 {
        size -= 1;
    }
    for _ in 0..count {
        let yy = y + (rng.range(size) - rng.between(3, if vertical { 5 } else { 3 }));
        if yy <= 0 {
            continue;
        }
        let xx = x + (rng.range(size) - 3);
        let zz = z + (rng.range(size) - 3);
        if grid.get_world(xx, yy, zz) == nb.air {
            grid.set(xx, yy, zz, nb.glowstone);
        }
    }
}

/// Fungi géants (réf Nukkit `PopulatorCrimson/WarpedFungus`) : 3-8 tentatives/chunk,
/// sur chaque face de nylium (3/4 de chance) un arbre fungus.
fn place_fungi(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    crimson: bool,
) {
    let nylium = if crimson {
        nb.crimson_nylium
    } else {
        nb.warped_nylium
    };
    let skip = if crimson { 1 } else { 0 }; // crimson skip si range(4)==1, warped si ==0
    let mut rng = crng(base_x, base_z, seed, 0x_F009);
    let amount = rng.range(6) + 3; // 3..8
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for fy in floors(grid, wx, wz, |b| b == nylium) {
            if fy <= 1 || rng.range(4) == skip {
                continue;
            }
            grow_fungus(grid, nb, wx, fy, wz, crimson, &mut rng);
        }
    }
}

/// Tapis de forêt (réf Nukkit `PopulatorCrimson/WarpedForestGround`) : 32-95
/// tentatives/chunk, roots/sprouts/petits fungus pondérés sur les faces de nylium.
fn place_forest_ground(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
    crimson: bool,
) {
    let nylium = if crimson {
        nb.crimson_nylium
    } else {
        nb.warped_nylium
    };
    let mut rng = crng(base_x, base_z, seed, 0x_6204);
    let amount = rng.range(64) + 32; // 32..95
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for fy in floors(grid, wx, wz, |b| b == nylium) {
            if fy <= 1 {
                continue;
            }
            let block = if crimson {
                if rng.range(6) == 0 {
                    if rng.range(8) == 1 {
                        nb.warped_fungus
                    } else {
                        nb.crimson_fungus
                    }
                } else {
                    nb.crimson_roots
                }
            } else if rng.range(4) == 0 {
                nb.nether_sprouts
            } else if rng.range(6) == 0 {
                nb.warped_fungus
            } else {
                nb.warped_roots
            };
            grid.set(wx, fy, wz, block);
        }
    }
}

/// Sol de soul sand valley (réf Nukkit `PopulatorSoulSandValleyGround`) : 1/15 chunk,
/// 3-9 touffes d'un bloc unique (crimson roots / champignon) sur le soul sand/soil.
fn place_ssv_ground(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let mut rng = crng(base_x, base_z, seed, 0x_5507);
    if rng.range(15) != 1 {
        return;
    }
    let block = if rng.range(2) == 0 {
        nb.crimson_roots
    } else if rng.range(2) == 0 {
        nb.red_mushroom
    } else {
        nb.brown_mushroom
    };
    let (soul_sand, soul_soil) = (nb.soul_sand, nb.soul_soil);
    let amount = rng.range(7) + 3; // 3..9
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for fy in floors(grid, wx, wz, |b| b == soul_sand || b == soul_soil) {
            if fy > 1 {
                grid.set(wx, fy, wz, block);
            }
        }
    }
}

/// Weeping vines (réf Nukkit `PopulatorWeepingVines`) : crimson, 1/8 chunk, 1-5
/// tentatives, pendent des plafonds (jusqu'à 10 blocs).
fn place_weeping_vines(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let air = BLOCKS.air;
    let mut rng = crng(base_x, base_z, seed, 0x_C1E5);
    if rng.range(8) < 7 {
        return;
    }
    let amount = rng.range(5) + 1; // 1..5
    let is_ceiling = |b: u32| b == nb.netherrack || b == nb.crimson_nylium;
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        let ceils: Vec<i32> = (2..126)
            .filter(|&y| {
                grid.get_world(wx, y, wz) == air && is_ceiling(grid.get_world(wx, y + 1, wz))
            })
            .collect();
        for cy in ceils {
            if cy <= 1 {
                continue;
            }
            let len = rng.range(10) + 1; // 1..10
            for i in 0..len {
                if grid.get_world(wx, cy - i, wz) != air {
                    break;
                }
                grid.set(wx, cy - i, wz, nb.weeping_vines);
            }
        }
    }
}

/// Twisting vines (réf Nukkit `PopulatorTwistingVines`) : warped, 1/8 chunk, 2-6
/// tentatives, montent du sol (3/4 de chance, jusqu'à ~7 blocs).
fn place_twisting_vines(
    grid: &mut DimGrid,
    nb: &NetherBlocks,
    base_x: i32,
    base_z: i32,
    seed: u64,
) {
    let air = BLOCKS.air;
    let mut rng = crng(base_x, base_z, seed, 0x_71E5);
    if rng.range(8) < 7 {
        return;
    }
    let amount = rng.range(5) + 2; // 2..6
    let is_ground = |b: u32| b == nb.netherrack || b == nb.warped_nylium;
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for fy in floors(grid, wx, wz, is_ground) {
            if fy <= 1 || rng.range(4) == 0 {
                continue;
            }
            let len = rng.range(7) + 1; // 1..7
            for i in 0..len {
                if grid.get_world(wx, fy + i, wz) != air {
                    break;
                }
                grid.set(wx, fy + i, wz, nb.twisting_vines);
            }
        }
    }
}

/// Fungus géant (réf Nukkit `ObjectNetherTree`) : tige (4..12) + chapeau de wart block
/// (rayon 2) + shroomlight semé. `base_y` = case d'air juste au-dessus du nylium.
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
    let h = rng.range(9) + 4; // 4..12
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
    for _ in 0..rng.between(1, 4) {
        let dx = rng.between(-2, 2);
        let dz = rng.between(-2, 2);
        grid.set(wx + dx, cap, wz + dz, nb.shroomlight);
    }
}

/// Basalt deltas (réf Nukkit `PopulatorBasaltDeltaLava/Magma/Pillar`) : sur le sol
/// basalt/blackstone — mares de lave (64-127, faces solides), magma au bord des
/// mares (20-23), colonnes de basalte (128-255).
fn place_basalt_deltas(grid: &mut DimGrid, nb: &NetherBlocks, base_x: i32, base_z: i32, seed: u64) {
    let air = BLOCKS.air;
    let is_delta = |g: &DimGrid, x: i32, y: i32, z: i32| {
        let b = g.get_world(x, y, z);
        (b == nb.basalt || b == nb.blackstone) && g.get_world(x, y + 1, z) == air
    };
    let solid = |g: &DimGrid, x: i32, y: i32, z: i32| g.get_world(x, y, z) != air;

    // Mares de lave : sol delta dont les 4 côtés sont solides → la surface devient lave.
    let mut rng = crng(base_x, base_z, seed, 0x_DA1A);
    let amount = rng.range(64) + 64; // 64..127
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for y in (2..127).rev() {
            if y > 1
                && is_delta(grid, wx, y, wz)
                && solid(grid, wx + 1, y, wz)
                && solid(grid, wx - 1, y, wz)
                && solid(grid, wx, y, wz + 1)
                && solid(grid, wx, y, wz - 1)
            {
                grid.set(wx, y, wz, nb.lava);
            }
        }
    }

    // Magma : sol delta avec une face de lave adjacente (bord des mares).
    let mut rng = crng(base_x, base_z, seed, 0x_DA2A);
    let amount = rng.range(4) + 20; // 20..23
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for y in (2..127).rev() {
            let near_lava = grid.get_world(wx + 1, y, wz) == nb.lava
                || grid.get_world(wx - 1, y, wz) == nb.lava
                || grid.get_world(wx, y, wz + 1) == nb.lava
                || grid.get_world(wx, y, wz - 1) == nb.lava;
            if y > 1 && is_delta(grid, wx, y, wz) && near_lava {
                grid.set(wx, y, wz, nb.magma);
            }
        }
    }

    // Colonnes de basalte : 128-255 tentatives, monte 1-5 blocs (4/5 de chance).
    let mut rng = crng(base_x, base_z, seed, 0x_DA3A);
    let amount = rng.range(128) + 128; // 128..255
    for _ in 0..amount {
        let wx = base_x + rng.range(16);
        let wz = base_z + rng.range(16);
        for y in (2..127).rev() {
            if y > 1 && is_delta(grid, wx, y, wz) {
                if rng.range(5) == 0 {
                    continue;
                }
                let height = rng.range(5) + 1; // 1..5
                for h in 0..height {
                    if grid.get_world(wx, y + 1 + h, wz) != air {
                        break;
                    }
                    grid.set(wx, y + 1 + h, wz, nb.basalt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorate_changes_surface_and_adds_features() {
        use crate::world::worldgen::density;
        let nb = &*NB;
        let seed = 42u64;
        let router = density::build_nether(seed);
        let sampler = ClimateSampler::from_router(&router);
        // Plusieurs biomes : au moins un bloc de surface de biome + des minerais.
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
        assert!(nylium_or_special > 0, "surfaces de biome attendues");
        assert!(ores > 0, "minerais Nether attendus");
    }

    #[test]
    fn forests_get_vegetation_and_glowstone() {
        use crate::world::worldgen::density;
        let nb = &*NB;
        let router = density::build_nether(42);
        let sampler = ClimateSampler::from_router(&router);
        let (mut veg, mut glow, mut lava) = (0usize, 0usize, 0usize);
        for cx in -8..8 {
            for cz in -8..8 {
                let (bx, bz) = (cx * 16, cz * 16);
                // Caverne : sol jusqu'à y60, vide, toit à partir de y100.
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
        assert!(lava > 0, "lave attendue");
    }

    #[test]
    fn basalt_deltas_get_lava_and_magma() {
        let nb = &*NB;
        // Sol de basalte (le ground du biome) → deltas posent lave + magma.
        let mut grid = DimGrid::test_cavern(0, 0, 128, nb.basalt, 60, 100);
        place_basalt_deltas(&mut grid, nb, 0, 0, 42);
        assert!(grid.count(nb.lava) > 0, "mares de lave attendues");
        assert!(grid.count(nb.magma) > 0, "magma de bord attendu");
    }
}

#[cfg(test)]
mod ore_probe {
    use super::*;
    use crate::world::worldgen::density;
    #[test]
    fn ore_counts_in_bedrock_ballpark() {
        let nb = &*NB;
        let router = density::build_nether(42);
        let sampler = ClimateSampler::from_router(&router);
        let (mut q, mut g, mut m) = (0usize, 0usize, 0usize);
        let chunks = 20;
        for cx in 0..chunks {
            let (bx, bz) = (cx * 16, 0);
            let mut grid = DimGrid::test_floor(bx, bz, 0, 128, nb.netherrack, 120);
            decorate(&mut grid, bx, bz, &sampler, 42);
            q += grid.count(nb.quartz_ore);
            g += grid.count(nb.gold_ore);
            m += grid.count(nb.magma);
        }
        let n = chunks as usize;
        // quartz 16×24 → plusieurs centaines/chunk ; magma 4×64 présent.
        assert!(q / n > 80, "quartz/chunk trop bas: {}", q / n);
        assert!(g / n > 10, "or/chunk trop bas: {}", g / n);
        assert!(m / n > 20, "magma attendu: {}", m / n);
    }
}
