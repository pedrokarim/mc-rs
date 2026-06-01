//! Phase A4 — Échantillonnage NoiseChunk + remplissage terrain.
//!
//! Porte la sémantique vanilla `NoiseChunk` : la density function `final_density`
//! est échantillonnée aux COINS des cellules (4×4 horizontal, 8 vertical) puis
//! interpolée trilinéairement à l'intérieur de chaque cellule. C'est précisément
//! ce que signifie le wrapper `interpolated` côté données vanilla — l'A3 traite
//! ces marqueurs comme transparents (cf. `density.rs`) et l'interpolation par
//! cellules est faite ici.
//!
//! Simplifications assumées à ce stade (levées dans les phases suivantes) :
//! - Biomes : Plains partout (Phase B = placement multi-noise 6D).
//! - Blocs : `stone` si densité > 0, sinon `water` au niveau de la mer ou en
//!   dessous, sinon air. Les surface rules (grass/dirt/sand/gravel…) = Phase C ;
//!   les aquifères + grottes par bruit = Phase D.
//!
//! Le router de bruit (équivalent du `RandomState` vanilla) est instancié une
//! seule fois par seed et réutilisé pour tous les chunks, garantissant la
//! continuité du bruit aux frontières.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::super::block_registry::BLOCKS;
use super::super::chunk_serializer;
use super::density::{self, NoiseRouter};

/// Plancher du monde (overworld 1.18+).
pub(super) const MIN_Y: i32 = -64;
/// Hauteur totale colonne (overworld). `noise.height` de `noise_settings`.
pub(super) const HEIGHT: i32 = 384;
/// Plafond (exclusif) = MIN_Y + HEIGHT.
pub(super) const MAX_Y: i32 = MIN_Y + HEIGHT;
/// Niveau de la mer (`sea_level` de `noise_settings/overworld.json`).
pub(super) const SEA_LEVEL: i32 = 63;
/// Espacement minimal (Chebyshev) entre troncs d'arbres d'un même chunk
/// d'origine — approxime la collision vanilla (les biomes denses comme la jungle
/// ne posent pas réellement tous leurs ~50 arbres). Réglable : ↑ = plus clairsemé.
/// 4 = jungle dense mais praticable (Bedrock-like), vs les ~22 arbres de Java.
const TREE_MIN_SPACING: i32 = 4;
/// Colonnes par chunk (16×16).
pub(super) const COLS: usize = 256;
/// Taille de la grille de blocs d'un chunk (toute la hauteur).
pub(super) const GRID_LEN: usize = HEIGHT as usize * COLS;

/// Index dans la grille de blocs pleine hauteur, ordre `[y][x][z]`.
#[inline]
pub(super) fn grid_index(lx: usize, wy: i32, lz: usize) -> usize {
    ((wy - MIN_Y) as usize) * COLS + lx * 16 + lz
}

/// Largeur d'une cellule horizontale = `size_horizontal(1) * 4`.
const CELL_W: i32 = 4;
/// Hauteur d'une cellule = `size_vertical(2) * 4`.
const CELL_H: i32 = 8;

/// Nombre de cellules horizontales par chunk (16 / 4).
const CELLS_XZ: usize = 16 / CELL_W as usize;
/// Nombre de cellules verticales (384 / 8).
const CELLS_Y: usize = HEIGHT as usize / CELL_H as usize;
/// Nombre de sub-chunks émis (384 / 16).
const SUB_CHUNK_COUNT: usize = HEIGHT as usize / 16;

/// Nombre de coins de cellule par axe horizontal (CELLS_XZ + 1).
const NX: usize = CELLS_XZ + 1;
/// Nombre de coins de cellule sur l'axe vertical (CELLS_Y + 1).
const NY: usize = CELLS_Y + 1;

/// Router caché par seed — instancié une fois (équiv. `RandomState` vanilla).
static ROUTER: LazyLock<Mutex<(u64, Option<NoiseRouter>)>> =
    LazyLock::new(|| Mutex::new((0, None)));

fn with_router<R>(seed: u64, f: impl FnOnce(&NoiseRouter) -> R) -> R {
    let mut guard = ROUTER.lock().unwrap();
    if guard.1.is_none() || guard.0 != seed {
        *guard = (seed, Some(density::build_overworld(seed)));
    }
    f(guard.1.as_ref().unwrap())
}

/// Cache global des hauteurs de surface par chunk (clé `(cx, cz)`), indispensable
/// à la population CROSS-CHUNK des arbres : pour placer les arbres d'un chunk
/// voisin (dont la canopée déborde dans le chunk courant), il faut sa surface —
/// calculée de façon DÉTERMINISTE (mêmes coins interpolés que son propre
/// terrain) et mise en cache pour ne la calculer qu'une fois par chunk.
type SurfaceGrid = Box<[[i32; 16]; 16]>;
/// `(seed courant, surfaces par chunk)`.
type SurfaceCache = (u64, HashMap<(i32, i32), SurfaceGrid>);
static SURF_CACHE: LazyLock<Mutex<SurfaceCache>> =
    LazyLock::new(|| Mutex::new((0, HashMap::new())));

/// Insère/écrase les surfaces d'un chunk déjà calculées (depuis le flux
/// principal) pour éviter de les recalculer dans la passe d'arbres.
fn put_surfaces(seed: u64, cx: i32, cz: i32, s: &[[i32; 16]; 16]) {
    let mut g = SURF_CACHE.lock().unwrap();
    if g.0 != seed {
        g.0 = seed;
        g.1.clear();
    }
    // Borne mémoire : vide si trop d'entrées (régénération bursty, sans impact
    // visuel — c'est un cache de perf, pas une source de vérité).
    if g.1.len() > 8192 {
        g.1.clear();
    }
    g.1.insert((cx, cz), Box::new(*s));
}

/// Surfaces d'un chunk (cache ou calcul). Le calcul échantillonne les coins du
/// chunk puis prend le plus haut bloc de densité > 0 par colonne — identique à
/// ce que fait le flux principal pour le chunk courant.
fn chunk_surfaces(seed: u64, router: &NoiseRouter, cx: i32, cz: i32) -> SurfaceGrid {
    {
        let g = SURF_CACHE.lock().unwrap();
        if g.0 == seed {
            if let Some(s) = g.1.get(&(cx, cz)) {
                return s.clone();
            }
        }
    }
    let corners = sample_corners(cx * 16, cz * 16, router);
    let mut s: SurfaceGrid = Box::new([[MIN_Y; 16]; 16]);
    for (lx, col) in s.iter_mut().enumerate() {
        for (lz, sy) in col.iter_mut().enumerate() {
            for wy in (MIN_Y..MAX_Y).rev() {
                if density_at(&corners, lx, wy, lz) > 0.0 {
                    *sy = wy;
                    break;
                }
            }
        }
    }
    put_surfaces(seed, cx, cz, &s);
    s
}

/// Biomes overworld (param list multi-noise + noms + IDs Bedrock), chargé une
/// fois. Partagé par la génération et la commande `/locate biome`.
static BIOMES: LazyLock<super::climate::OverworldBiomes> =
    LazyLock::new(super::climate::load_overworld);

/// Localise l'occurrence la plus proche d'un `biome` depuis `(origin_x,
/// origin_z)` à l'altitude `y`, par anneaux carrés croissants (pas `STEP`)
/// échantillonnant le climat — comme `/locate biome` vanilla (qui interroge la
/// `BiomeSource`, pas le terrain). Retourne `(x, z)` ou `None` si rien dans le
/// rayon max. Le biome accepte avec ou sans préfixe `minecraft:`.
pub fn locate_biome(
    seed: u64,
    origin_x: i32,
    origin_z: i32,
    y: i32,
    biome: &str,
) -> Option<(i32, i32)> {
    let want = biome.strip_prefix("minecraft:").unwrap_or(biome);
    let target_idx = BIOMES
        .names
        .iter()
        .position(|n| n.strip_prefix("minecraft:").unwrap_or(n) == want)?
        as u16;

    let router = with_router(seed, |r| r.clone());
    let climate = super::climate::ClimateSampler::from_router(&router);
    let qy = y >> 2;
    let matches =
        |x: i32, z: i32| BIOMES.params.find(&climate.sample(x >> 2, qy, z >> 2)) == target_idx;

    const STEP: i32 = 16;
    const MAX_RADIUS: i32 = 6400;
    if matches(origin_x, origin_z) {
        return Some((origin_x, origin_z));
    }
    let mut r = STEP;
    while r <= MAX_RADIUS {
        let mut i = -r;
        while i <= r {
            for (x, z) in [
                (origin_x + i, origin_z - r),
                (origin_x + i, origin_z + r),
                (origin_x - r, origin_z + i),
                (origin_x + r, origin_z + i),
            ] {
                if matches(x, z) {
                    return Some((x, z));
                }
            }
            i += STEP;
        }
        r += STEP;
    }
    None
}

/// Seed déterministe d'un chunk pour la passe d'arbres — ne dépend QUE du chunk
/// d'origine (pas du chunk en cours de génération), pour que le même arbre soit
/// calculé à l'identique qu'on le voie depuis son chunk ou depuis un voisin.
fn tree_chunk_seed(cx: i32, cz: i32, seed: u64) -> i64 {
    (cx as i64).wrapping_mul(0x9e37_79b9_7f4a_7c15u64 as i64)
        ^ (cz as i64).wrapping_mul(0x6a09_e667_f3bc_c909u64 as i64)
        ^ seed as i64
}

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
fn palette_index(palette: &mut Vec<u32>, id: u32) -> u32 {
    if let Some(p) = palette.iter().position(|&b| b == id) {
        p as u32
    } else {
        palette.push(id);
        (palette.len() - 1) as u32
    }
}

/// Échantillonne la densité aux coins des cellules du chunk.
/// Indexé `[cell_x][cell_y][cell_z]` (coins, pas blocs).
fn sample_corners(base_x: i32, base_z: i32, router: &NoiseRouter) -> Box<[[[f64; NX]; NY]; NX]> {
    let mut corners = Box::new([[[0.0f64; NX]; NY]; NX]);
    let fd = &router.final_density;
    for (cx, plane) in corners.iter_mut().enumerate() {
        let wx = base_x + cx as i32 * CELL_W;
        for (cy, row) in plane.iter_mut().enumerate() {
            let wy = MIN_Y + cy as i32 * CELL_H;
            for (cz, slot) in row.iter_mut().enumerate() {
                let wz = base_z + cz as i32 * CELL_W;
                *slot = fd.compute(wx, wy, wz);
            }
        }
    }
    corners
}

/// Densité interpolée trilinéairement en un bloc, à partir des coins de cellule.
#[inline]
fn density_at(corners: &[[[f64; NX]; NY]; NX], lx: usize, wy: i32, lz: usize) -> f64 {
    let cx = lx / CELL_W as usize;
    let cz = lz / CELL_W as usize;
    let cy = ((wy - MIN_Y) / CELL_H) as usize;
    let dx = (lx % CELL_W as usize) as f64 / CELL_W as f64;
    let dz = (lz % CELL_W as usize) as f64 / CELL_W as f64;
    let dy = ((wy - MIN_Y) % CELL_H) as f64 / CELL_H as f64;

    let v000 = corners[cx][cy][cz];
    let v100 = corners[cx + 1][cy][cz];
    let v010 = corners[cx][cy + 1][cz];
    let v110 = corners[cx + 1][cy + 1][cz];
    let v001 = corners[cx][cy][cz + 1];
    let v101 = corners[cx + 1][cy][cz + 1];
    let v011 = corners[cx][cy + 1][cz + 1];
    let v111 = corners[cx + 1][cy + 1][cz + 1];

    // Interpolation x, puis y, puis z.
    let v00 = lerp(dx, v000, v100);
    let v10 = lerp(dx, v010, v110);
    let v01 = lerp(dx, v001, v101);
    let v11 = lerp(dx, v011, v111);
    let v0 = lerp(dy, v00, v10);
    let v1 = lerp(dy, v01, v11);
    lerp(dz, v0, v1)
}

/// Génère un chunk via le générateur de bruit moderne.
/// Retourne `(sub_chunk_count, payload réseau)` — même contrat que
/// `terrain_generator::generate_terrain_chunk`.
pub fn generate_noise_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let (grid, biome3d) = generate_chunk_grid(chunk_x, chunk_z, seed);
    serialize_chunk(&grid, &biome3d)
}

/// Construit la grille de blocs pleine hauteur d'un chunk (terrain + aquifères +
/// biomes + surface rules + minerais + arbres cross-chunk + décoration) et la
/// carte de biomes 3D. Séparé de la sérialisation pour pouvoir l'inspecter (tests
/// / diagnostics).
pub(crate) fn generate_chunk_grid(
    chunk_x: i32,
    chunk_z: i32,
    seed: u64,
) -> (Box<[u32]>, Vec<[[u16; 4]; 4]>) {
    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;

    // Router cloné hors du Mutex (partage d'`Arc<Df>`, peu coûteux).
    let router = with_router(seed, |r| r.clone());
    let corners = sample_corners(base_x, base_z, &router);
    let climate = super::climate::ClimateSampler::from_router(&router);

    // Surfaces (plus haut bloc solide = densité > 0), calculées AVANT le
    // remplissage : nécessaires à l'aquifère et au placement des biomes.
    let mut surfaces = [[MIN_Y; 16]; 16];
    for (lx, col) in surfaces.iter_mut().enumerate() {
        for (lz, sy) in col.iter_mut().enumerate() {
            for wy in (MIN_Y..MAX_Y).rev() {
                if density_at(&corners, lx, wy, lz) > 0.0 {
                    *sy = wy;
                    break;
                }
            }
        }
    }

    // 1) Forme du terrain + aquifères : stone / eau / lave / air. Sous le niveau
    // de la mer, l'aquifère décide eau/lave/air (grottes sèches ou inondées) ;
    // au-dessus, l'air reste l'air (les aquifères perchés sont négligés).
    let lava = BLOCKS.get("minecraft:lava");
    let mut aquifer = crate::world::worldgen::aquifer::Aquifer::new(&router, seed);
    let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
    for lx in 0..16usize {
        for lz in 0..16usize {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;
            for wy in MIN_Y..MAX_Y {
                let d = density_at(&corners, lx, wy, lz);
                if d > 0.0 {
                    grid[grid_index(lx, wy, lz)] = BLOCKS.stone;
                } else if wy <= SEA_LEVEL {
                    match aquifer.compute(wx, wy, wz, d) {
                        crate::world::worldgen::aquifer::Fluid::Water => {
                            grid[grid_index(lx, wy, lz)] = BLOCKS.water
                        }
                        crate::world::worldgen::aquifer::Fluid::Lava => {
                            grid[grid_index(lx, wy, lz)] = lava
                        }
                        crate::world::worldgen::aquifer::Fluid::Air => {}
                    }
                }
            }
        }
    }

    // 2a) Biome de SURFACE (2D) : échantillonné à la surface de chaque colonne.
    // Utilisé par les surface rules et la déco de surface. (`BIOMES` = static
    // module-level, partagé avec `locate_biome`.)
    let mut biome_idx = [[0u16; 16]; 16];
    for lx in 0..16usize {
        for lz in 0..16usize {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;
            let sy = surfaces[lx][lz];
            let target = climate.sample(wx >> 2, sy >> 2, wz >> 2);
            biome_idx[lx][lz] = BIOMES.params.find(&target);
        }
    }

    // 2b) Biomes 3D : échantillonnés au centre de chaque sub-chunk (cellule
    // 4×4×4). En profondeur, `depth` est élevé → biomes de grottes (lush_caves,
    // dripstone_caves, deep_dark). `biome3d[sub_idx][cx][cz]` = index Java.
    let mut biome3d: Vec<[[u16; 4]; 4]> = Vec::with_capacity(SUB_CHUNK_COUNT);
    for sub_idx in 0..SUB_CHUNK_COUNT {
        let cy = MIN_Y + sub_idx as i32 * 16 + 8;
        let mut sec = [[0u16; 4]; 4];
        #[allow(clippy::needless_range_loop)]
        for cx in 0..4usize {
            for cz in 0..4usize {
                let wx = base_x + cx as i32 * 4 + 2;
                let wz = base_z + cz as i32 * 4 + 2;
                let target = climate.sample(wx >> 2, cy >> 2, wz >> 2);
                sec[cx][cz] = BIOMES.params.find(&target);
            }
        }
        biome3d.push(sec);
    }

    // 3) Surface rules vanilla (par biome) : grass/dirt/sable/grès/gravier/
    // terracotta + bedrock/deepslate.
    super::surface::build(
        &mut grid,
        seed,
        base_x,
        base_z,
        &biome_idx,
        &BIOMES.names,
        &surfaces,
    );

    // 4) Minerais : insérés dans la roche (stone/deepslate) souterraine.
    let mut ore_rng = super::super::random::Random::new(
        0x006f_7265_i64 ^ ((chunk_x as i64) << 16) ^ chunk_z as i64 ^ seed as i64,
    );
    let ores = super::super::ore::generate_ores(chunk_x, chunk_z, &mut ore_rng);
    for (&(lx, wy, lz), &ore) in &ores {
        if (MIN_Y..MAX_Y).contains(&wy) {
            let i = grid_index(lx as usize, wy, lz as usize);
            if grid[i] == BLOCKS.stone || grid[i] == BLOCKS.deepslate {
                grid[i] = ore;
            }
        }
    }

    // 4b) Arbres — population CROSS-CHUNK. On parcourt le voisinage 3×3 et, pour
    // CHAQUE chunk d'origine (le courant + ses 8 voisins), on rejoue son placement
    // d'arbres déterministe et on écrit dans CE chunk uniquement les blocs qui y
    // tombent (le reste est clippé par `idx_ok`). Un arbre dont le tronc est dans
    // un voisin mais dont la canopée déborde ici est ainsi posé de façon
    // identique des deux côtés → plus de coupures aux frontières de chunk.
    put_surfaces(seed, chunk_x, chunk_z, &surfaces); // évite de recalculer le centre
    {
        // Surface du monde à des coords LOCALES au chunk courant (base_x/base_z),
        // pour la décay des feuilles (un log sous le terrain est mangé). Cache
        // local par chunk pour éviter de recloner les surfaces.
        let surf_cache: std::cell::RefCell<HashMap<(i32, i32), SurfaceGrid>> =
            std::cell::RefCell::new(HashMap::new());
        let surface_at = |gx: i32, gz: i32| -> i32 {
            let (wx, wz) = (base_x + gx, base_z + gz);
            let (ccx, ccz) = (wx >> 4, wz >> 4);
            let mut cache = surf_cache.borrow_mut();
            let s = cache
                .entry((ccx, ccz))
                .or_insert_with(|| chunk_surfaces(seed, &router, ccx, ccz));
            s[(wx & 15) as usize][(wz & 15) as usize]
        };
        for dcz in -1..=1i32 {
            for dcx in -1..=1i32 {
                let ocx = chunk_x + dcx;
                let ocz = chunk_z + dcz;
                let osurf = chunk_surfaces(seed, &router, ocx, ocz);
                let ob_x = ocx * 16;
                let ob_z = ocz * 16;
                let mut trng = super::super::random::Random::new(tree_chunk_seed(ocx, ocz, seed));

                // Densité moyenne d'arbres du chunk d'origine (échantillon 4×4,
                // déterministe : ne consomme pas le RNG d'arbres).
                let mut sum = 0.0f64;
                for sx in 0..4i32 {
                    for sz in 0..4i32 {
                        let lx = sx * 4 + 2;
                        let lz = sz * 4 + 2;
                        let sy = osurf[lx as usize][lz as usize];
                        let target = climate.sample((ob_x + lx) >> 2, sy >> 2, (ob_z + lz) >> 2);
                        let name = &BIOMES.names[BIOMES.params.find(&target) as usize];
                        sum += super::features::tree_plan(name).density;
                    }
                }
                let mean = sum / 16.0;
                let attempts = mean.floor() as i32 + i32::from(trng.next_float() < mean.fract());

                // Centres de troncs déjà posés CE chunk d'origine → espacement
                // déterministe. Approxime la collision vanilla : le `count` d'un
                // biome est un nombre de TENTATIVES, et les arbres denses se
                // gênent (la jungle ne pose pas réellement ses ~50 arbres). Comme
                // c'est calculé sur le seul RNG d'origine, ça reste cohérent
                // cross-chunk.
                let mut centers: Vec<(i32, i32)> = Vec::new();
                for _ in 0..attempts {
                    let lx = trng.next_bounded_int(16);
                    let lz = trng.next_bounded_int(16);
                    let wx = ob_x + lx;
                    let wz = ob_z + lz;
                    let ground = osurf[lx as usize][lz as usize];
                    if ground <= SEA_LEVEL {
                        continue;
                    }
                    let target = climate.sample(wx >> 2, ground >> 2, wz >> 2);
                    let name = &BIOMES.names[BIOMES.params.find(&target) as usize];
                    // Composition data-driven (vraies données vanilla, cf. `features`).
                    let plan = super::features::tree_plan(name);
                    if plan.density <= 0.0 {
                        continue;
                    }
                    // Rejette si trop proche d'un tronc déjà posé (Chebyshev).
                    if centers.iter().any(|&(px, pz)| {
                        (px - wx).abs() < TREE_MIN_SPACING && (pz - wz).abs() < TREE_MIN_SPACING
                    }) {
                        continue;
                    }
                    if let Some(species) = plan.pick(&mut trng) {
                        centers.push((wx, wz));
                        // Coords locales au chunk CIBLE (hors 0..16 → clippé) ;
                        // formes Bedrock fidèles (port Allay) dans `trees`.
                        super::trees::place(
                            &mut grid,
                            &mut trng,
                            species,
                            wx - base_x,
                            ground,
                            wz - base_z,
                            &surface_at,
                        );
                    }
                }
            }
        }
    }

    // 5) Décoration riche par biome (lianes, herbe/fleurs, aquatique :
    // kelp/seagrass/coraux). Pilotée par les noms de biome Java, opère
    // directement sur la grille. (Les arbres sont posés en 4b ci-dessus.)
    super::decoration::decorate(
        &mut grid,
        seed,
        chunk_x,
        chunk_z,
        &biome_idx,
        &BIOMES.names,
        &surfaces,
        &biome3d,
    );

    (grid, biome3d)
}

/// Sérialise une grille + biomes 3D au format réseau sub-chunk de Bedrock.
fn serialize_chunk(grid: &[u32], biome3d: &[[[u16; 4]; 4]]) -> (u32, Vec<u8>) {
    // 5) Sérialisation sub-chunk par sub-chunk.
    let mut payload = Vec::with_capacity(16384);
    for sub_idx in 0..SUB_CHUNK_COUNT {
        let sub_y_start = MIN_Y + sub_idx as i32 * 16;

        let mut blocks = [0u32; 4096];
        let mut palette = vec![BLOCKS.air];

        for lx in 0..16usize {
            for lz in 0..16usize {
                for ly in 0..16usize {
                    let wy = sub_y_start + ly as i32;
                    let block = grid[grid_index(lx, wy, lz)];
                    if block == BLOCKS.air {
                        continue;
                    }
                    let pidx = palette_index(&mut palette, block);
                    blocks[(lx << 8) | (lz << 4) | ly] = pidx;
                }
            }
        }

        if palette.len() == 1 {
            // Sub-chunk entièrement air → forme compacte (version + 0 couche).
            payload.push(8);
            payload.push(0);
        } else {
            payload.extend_from_slice(&chunk_serializer::serialize_sub_chunk(&blocks, &palette));
        }
    }

    // 6) Biomes sérialisés en 3D (une section 4×4×4 par sub-chunk).
    for sec in biome3d {
        let mut cells = [0u32; 64];
        #[allow(clippy::needless_range_loop)]
        for cx in 0..4usize {
            for cz in 0..4usize {
                let id = BIOMES.bedrock_ids[sec[cx][cz] as usize];
                for cy in 0..4usize {
                    cells[(cx << 4) | (cz << 2) | cy] = id;
                }
            }
        }
        payload.extend_from_slice(&chunk_serializer::serialize_biome_section(&cells));
    }

    // Border blocks count.
    payload.push(0);

    (SUB_CHUNK_COUNT as u32, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_nonempty_chunk() {
        let (count, payload) = generate_noise_chunk(0, 0, 42);
        assert_eq!(count, SUB_CHUNK_COUNT as u32);
        assert!(!payload.is_empty());
    }

    #[test]
    fn deterministic_same_seed() {
        let a = generate_noise_chunk(3, -7, 123);
        let b = generate_noise_chunk(3, -7, 123);
        assert_eq!(a, b);
    }

    #[test]
    #[ignore]
    fn diag_floating_leaves() {
        use std::collections::HashSet;
        let seed = 10619927421199041390u64;
        let species = [
            "oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry", "mangrove",
        ];
        let leaf_ids: HashSet<u32> = species
            .iter()
            .map(|s| BLOCKS.get(&format!("minecraft:{s}_leaves")))
            .collect();
        let log_ids: HashSet<u32> = species
            .iter()
            .map(|s| BLOCKS.get(&format!("minecraft:{s}_log")))
            .collect();
        // Centre la région sur une jungle (sinon : pas d'arbres).
        let (jx, jz) = locate_biome(seed, 0, 0, 70, "jungle").expect("pas de jungle");
        let (ccx, ccz) = (jx >> 4, jz >> 4);
        println!("jungle trouvée en ({jx},{jz}) → chunk ({ccx},{ccz})");
        let mut logs: HashSet<(i32, i32, i32)> = HashSet::new();
        let mut leaves: Vec<(i32, i32, i32)> = Vec::new();
        let (x0, z0) = (ccx - 2, ccz - 2);
        for cx in x0..x0 + 5 {
            for cz in z0..z0 + 5 {
                let (grid, _) = generate_chunk_grid(cx, cz, seed);
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        for wy in SEA_LEVEL..MAX_Y {
                            let b = grid[grid_index(lx, wy, lz)];
                            let wp = (cx * 16 + lx as i32, wy, cz * 16 + lz as i32);
                            if log_ids.contains(&b) {
                                logs.insert(wp);
                            } else if leaf_ids.contains(&b) {
                                leaves.push(wp);
                            }
                        }
                    }
                }
            }
        }
        let near_log = |p: (i32, i32, i32)| {
            for dx in -6..=6i32 {
                for dy in -6..=6i32 {
                    for dz in -6..=6i32 {
                        if dx.abs() + dy.abs() + dz.abs() <= 6
                            && logs.contains(&(p.0 + dx, p.1 + dy, p.2 + dz))
                        {
                            return true;
                        }
                    }
                }
            }
            false
        };
        // Indexe les blocs pour pouvoir interroger le terrain sous un floater.
        let mut blocks: std::collections::HashMap<(i32, i32, i32), u32> =
            std::collections::HashMap::new();
        for cx in x0..x0 + 5 {
            for cz in z0..z0 + 5 {
                let (grid, _) = generate_chunk_grid(cx, cz, seed);
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        for wy in SEA_LEVEL..MAX_Y {
                            let b = grid[grid_index(lx, wy, lz)];
                            if b != BLOCKS.air {
                                blocks.insert((cx * 16 + lx as i32, wy, cz * 16 + lz as i32), b);
                            }
                        }
                    }
                }
            }
        }
        let stone = BLOCKS.stone;
        let dirt = BLOCKS.get("minecraft:dirt");
        let grass_block = BLOCKS.grass_block;
        let is_terrain = |b: u32| b == stone || b == dirt || b == grass_block;

        let (xlo, xhi) = ((x0 + 1) * 16, (x0 + 4) * 16);
        let (zlo, zhi) = ((z0 + 1) * 16, (z0 + 4) * 16);
        let mut floaters = 0;
        let mut on_boundary = 0; // floater dont x%16 ou z%16 ∈ {0,15}
        let mut terrain_below = 0; // floater avec terrain à ≤3 en dessous
        let mut samples = Vec::new();
        let mut by_species: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
        for &p in &leaves {
            if !(xlo..xhi).contains(&p.0) || !(zlo..zhi).contains(&p.2) {
                continue;
            }
            if !near_log(p) {
                floaters += 1;
                let bx = p.0 & 15;
                let bz = p.2 & 15;
                if bx == 0 || bx == 15 || bz == 0 || bz == 15 {
                    on_boundary += 1;
                }
                if (1..=3).any(|d| {
                    blocks
                        .get(&(p.0, p.1 - d, p.2))
                        .is_some_and(|&b| is_terrain(b))
                }) {
                    terrain_below += 1;
                }
                if samples.len() < 6 {
                    samples.push(p);
                }
                let id = blocks.get(&p).copied().unwrap_or(0);
                *by_species.entry(id).or_insert(0) += 1;
            }
        }
        println!("floaters: total={floaters}  sur bord {{0,15}}={on_boundary}  terrain≤3 dessous={terrain_below}");
        for s in &species {
            let id = BLOCKS.get(&format!("minecraft:{s}_leaves"));
            let n = by_species.get(&id).copied().unwrap_or(0);
            if n > 0 {
                println!("  espèce {s}: {n} floaters");
            }
        }
        // Distance au tronc le plus proche (rayon large) pour les samples.
        let nearest = |p: (i32, i32, i32)| -> i32 {
            let mut best = 999;
            for dx in -24..=24i32 {
                for dy in -24..=24i32 {
                    for dz in -24..=24i32 {
                        if logs.contains(&(p.0 + dx, p.1 + dy, p.2 + dz)) {
                            best = best.min(dx.abs() + dy.abs() + dz.abs());
                        }
                    }
                }
            }
            best
        };
        println!(
            "=== leaves={} logs={} floaters_interieur={floaters}",
            leaves.len(),
            logs.len()
        );
        for &p in &samples {
            println!(
                "  floater {p:?}  x%16={} z%16={}  tronc_le_plus_proche={}",
                p.0 & 15,
                p.2 & 15,
                nearest(p)
            );
        }
        // Dump : colonnes verticales (blocs) autour du 1er floater, pour voir où
        // le tronc s'arrête. L = log, F = feuille, # = terrain, . = air.
        let jungle_log = BLOCKS.get("minecraft:jungle_log");
        let jungle_leaves = BLOCKS.get("minecraft:jungle_leaves");
        let sym = |b: u32| -> char {
            if b == jungle_log {
                'L'
            } else if b == jungle_leaves {
                'F'
            } else if is_terrain(b) {
                '#'
            } else if b == BLOCKS.air {
                '.'
            } else {
                '?'
            }
        };
        if let Some(&f) = samples.first() {
            println!(
                "--- colonnes autour du floater {f:?} (y {} bas -> {} haut) ---",
                f.1 - 14,
                f.1 + 4
            );
            for dx in -3..=3i32 {
                for dz in -3..=3i32 {
                    let (cx, cz) = (f.0 + dx, f.2 + dz);
                    let col: String = (f.1 - 14..=f.1 + 4)
                        .map(|wy| sym(*blocks.get(&(cx, wy, cz)).unwrap_or(&BLOCKS.air)))
                        .collect();
                    if col.contains('L') || col.contains('F') {
                        println!("  ({cx},*,{cz}) x%16={} z%16={}: {col}", cx & 15, cz & 15);
                    }
                }
            }
        }
    }

    #[test]
    fn locate_biome_finds_origin_biome_and_rejects_unknown() {
        let seed = 42u64;
        // Le biome présent à l'origine est trouvé immédiatement (distance 0).
        let here = BIOMES.params.find(&{
            let r = with_router(seed, |r| r.clone());
            crate::world::worldgen::climate::ClimateSampler::from_router(&r).sample(0, 63 >> 2, 0)
        });
        let name = &BIOMES.names[here as usize];
        assert_eq!(locate_biome(seed, 0, 0, 63, name), Some((0, 0)));
        // Nom de biome inconnu → None (pas de boucle infinie).
        assert_eq!(locate_biome(seed, 0, 0, 63, "minecraft:not_a_biome"), None);
    }

    #[test]
    fn neighbor_generation_does_not_change_chunk() {
        // La passe d'arbres cross-chunk lit/écrit un cache global de surfaces
        // partagé entre chunks. Générer les voisins (qui mutent ce cache) ne doit
        // PAS changer la sortie d'un chunk donné → population déterministe.
        let a = generate_noise_chunk(5, 5, 777);
        let _ = generate_noise_chunk(6, 5, 777);
        let _ = generate_noise_chunk(5, 6, 777);
        let _ = generate_noise_chunk(4, 5, 777);
        let a2 = generate_noise_chunk(5, 5, 777);
        assert_eq!(a, a2, "la génération d'un chunk dépend de ses voisins");
    }

    #[test]
    fn chunk_surfaces_consistent_with_main_flow() {
        // La surface qu'un voisin calcule pour un chunk doit être identique à
        // celle que ce chunk calcule pour lui-même (sinon canopées décalées).
        let router = with_router(42, |r| r.clone());
        let s = chunk_surfaces(42, &router, 2, -3);
        // Recalcul direct (sans cache) via les coins, comme le flux principal.
        let corners = sample_corners(2 * 16, -3 * 16, &router);
        for lx in 0..16usize {
            for lz in 0..16usize {
                let mut expect = MIN_Y;
                for wy in (MIN_Y..MAX_Y).rev() {
                    if density_at(&corners, lx, wy, lz) > 0.0 {
                        expect = wy;
                        break;
                    }
                }
                assert_eq!(s[lx][lz], expect, "surface incohérente en ({lx},{lz})");
            }
        }
    }

    #[test]
    fn has_ground_and_air() {
        // Sur une colonne, il doit exister du solide en profondeur et de l'air
        // tout en haut (terrain plausible, pas une colonne pleine ou vide).
        let corners = with_router(42, |r| sample_corners(0, 0, r));
        let solid_deep = density_at(&corners, 0, -60, 0) > 0.0;
        let air_high = density_at(&corners, 0, 300, 0) <= 0.0;
        assert!(solid_deep, "le sous-sol profond devrait être solide");
        assert!(air_high, "le ciel devrait être de l'air");
    }

    #[test]
    fn surface_in_plausible_range() {
        let corners = with_router(42, |r| sample_corners(0, 0, r));
        let mut surface = None;
        for wy in (MIN_Y..(MIN_Y + HEIGHT)).rev() {
            if density_at(&corners, 8, wy, 8) > 0.0 {
                surface = Some(wy);
                break;
            }
        }
        let s = surface.expect("une surface solide doit exister");
        assert!(
            (-64..=200).contains(&s),
            "surface hors plage plausible: y={s}"
        );
    }
}
