//! Formes d'arbres **fidèles à Bedrock**, portées des `*TreeFeature` du serveur
//! Allay (`.reference/Allay/.../world/feature/tree/`). Allay est un serveur
//! Bedrock : ses arbres sont la référence canonique de l'aspect Bedrock.
//!
//! Adaptations nécessaires vs Allay :
//! - `ThreadLocalRandom` (non déterministe) → notre `Random` seedé (obligatoire
//!   pour la passe de population CROSS-CHUNK déterministe : la forme d'un arbre
//!   ne doit dépendre QUE du RNG + position, jamais de l'état de la grille du
//!   chunk en cours, sinon les canopées débordantes divergeraient).
//! - Le « monde » est traité comme de l'AIR pour toutes les décisions de forme
//!   (hauteur libre, collisions) → l'arbre génère toujours sa forme canonique.
//!   Les écritures, elles, sont **clippées** au chunk cible et ne remplacent que
//!   l'air/l'eau — donc un arbre posé depuis deux chunks voisins écrit des blocs
//!   identiques aux mêmes positions monde.
//! - Coordonnées **locales au chunk cible** (peuvent sortir de 0..16 ; le clip
//!   est fait par `idx`). Pas besoin des coords monde.

use std::sync::LazyLock;

use super::super::block_registry::BLOCKS;
use super::super::random::Random;
use super::decoration::Species;
use super::noise_chunk::{grid_index, MAX_Y, MIN_Y};

/// IDs runtime des blocs d'arbres (résolus une fois).
struct TreeBlocks {
    // (log, leaves) par espèce de base.
    oak_log: u32,
    oak_leaves: u32,
    birch_log: u32,
    birch_leaves: u32,
    spruce_log: u32,
    spruce_leaves: u32,
    jungle_log: u32,
    jungle_leaves: u32,
    acacia_log: u32,
    acacia_leaves: u32,
    dark_oak_log: u32,
    dark_oak_leaves: u32,
    cherry_log: u32,
    cherry_leaves: u32,
    mangrove_log: u32,
    mangrove_leaves: u32,
    mangrove_roots: u32,
    azalea_leaves: u32,
    azalea_leaves_flowered: u32,
    // Sol / déco.
    air: u32,
    water: u32,
    dirt: u32,
    grass_block: u32,
    podzol: u32,
    dirt_with_roots: u32,
    moss_carpet: u32,
    vine: u32,
}

static TB: LazyLock<TreeBlocks> = LazyLock::new(|| {
    let g = |n: &str| BLOCKS.get(n);
    TreeBlocks {
        oak_log: g("minecraft:oak_log"),
        oak_leaves: g("minecraft:oak_leaves"),
        birch_log: g("minecraft:birch_log"),
        birch_leaves: g("minecraft:birch_leaves"),
        spruce_log: g("minecraft:spruce_log"),
        spruce_leaves: g("minecraft:spruce_leaves"),
        jungle_log: g("minecraft:jungle_log"),
        jungle_leaves: g("minecraft:jungle_leaves"),
        acacia_log: g("minecraft:acacia_log"),
        acacia_leaves: g("minecraft:acacia_leaves"),
        dark_oak_log: g("minecraft:dark_oak_log"),
        dark_oak_leaves: g("minecraft:dark_oak_leaves"),
        cherry_log: g("minecraft:cherry_log"),
        cherry_leaves: g("minecraft:cherry_leaves"),
        mangrove_log: g("minecraft:mangrove_log"),
        mangrove_leaves: g("minecraft:mangrove_leaves"),
        mangrove_roots: g("minecraft:mangrove_roots"),
        azalea_leaves: g("minecraft:azalea_leaves"),
        azalea_leaves_flowered: g("minecraft:azalea_leaves_flowered"),
        air: BLOCKS.air,
        water: BLOCKS.water,
        dirt: BLOCKS.dirt,
        grass_block: BLOCKS.grass_block,
        podzol: g("minecraft:podzol"),
        dirt_with_roots: g("minecraft:dirt_with_roots"),
        moss_carpet: g("minecraft:moss_carpet"),
        vine: g("minecraft:vine"),
    }
});

/// Contexte d'écriture : la grille cible (coords locales clippées) + table de
/// blocs. Le RNG est passé séparément pour éviter les conflits d'emprunt avec
/// les closures `skipper`.
struct Ctx<'a> {
    grid: &'a mut [u32],
    tb: &'static TreeBlocks,
    /// Toutes les positions de log de l'arbre courant (y compris clippées hors
    /// chunk — « virtuelles » — pour que la décay reste cohérente cross-chunk).
    logs: Vec<(i32, i32, i32)>,
    /// Positions de feuilles réellement écrites (candidates à la décay).
    leaves: Vec<(i32, i32, i32)>,
}

impl Ctx<'_> {
    #[inline]
    fn idx(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if (0..16).contains(&x) && (0..16).contains(&z) && (MIN_Y..MAX_Y).contains(&y) {
            Some(grid_index(x as usize, y, z as usize))
        } else {
            None
        }
    }

    #[inline]
    fn get(&self, x: i32, y: i32, z: i32) -> u32 {
        self.idx(x, y, z)
            .map(|i| self.grid[i])
            .unwrap_or(self.tb.air)
    }

    /// Un bloc remplaçable par du bois/feuille = air ou eau. (Au moment des
    /// arbres, seuls le terrain + air + eau existent : pas encore de plantes.)
    #[inline]
    fn replaceable(&self, id: u32) -> bool {
        id == self.tb.air || id == self.tb.water
    }

    /// Vrai si `id` est un bloc de feuilles (toutes espèces). Un TRONC peut
    /// écraser une feuille (il la traverse) : sans ça, un petit arbre sous la
    /// canopée d'un méga arbre aurait son tronc bloqué par les feuilles voisines
    /// → seul son sommet serait écrit (feuilles flottantes). Déterministe car
    /// l'ordre de pose des arbres l'est.
    #[inline]
    fn is_leaf(&self, id: u32) -> bool {
        let t = self.tb;
        id == t.oak_leaves
            || id == t.birch_leaves
            || id == t.spruce_leaves
            || id == t.jungle_leaves
            || id == t.acacia_leaves
            || id == t.dark_oak_leaves
            || id == t.cherry_leaves
            || id == t.mangrove_leaves
            || id == t.azalea_leaves
            || id == t.azalea_leaves_flowered
    }

    /// Vrai si `id` est un bloc de bûche/racine d'arbre.
    #[inline]
    fn is_log(&self, id: u32) -> bool {
        let t = self.tb;
        id == t.oak_log
            || id == t.birch_log
            || id == t.spruce_log
            || id == t.jungle_log
            || id == t.acacia_log
            || id == t.dark_oak_log
            || id == t.cherry_log
            || id == t.mangrove_log
            || id == t.mangrove_roots
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32, z: i32, id: u32) {
        if let Some(i) = self.idx(x, y, z) {
            self.grid[i] = id;
        }
    }

    /// Pose un log si la cible est air/eau OU une feuille (le tronc traverse le
    /// feuillage). Le log est enregistré pour les décos (vines/podzol).
    fn place_log(&mut self, x: i32, y: i32, z: i32, log: u32, placed: &mut Vec<(i32, i32, i32)>) {
        if let Some(i) = self.idx(x, y, z) {
            let cur = self.grid[i];
            if self.replaceable(cur) || self.is_leaf(cur) {
                self.grid[i] = log;
            }
        }
        placed.push((x, y, z));
        self.logs.push((x, y, z));
    }

    fn try_leaf(&mut self, x: i32, y: i32, z: i32, leaf: u32, placed: &mut Vec<(i32, i32, i32)>) {
        if let Some(i) = self.idx(x, y, z) {
            if self.replaceable(self.grid[i]) {
                self.grid[i] = leaf;
                placed.push((x, y, z));
                self.leaves.push((x, y, z));
            }
        }
    }
}

/// Coordonnée d'une cellule de rangée de feuilles.
#[derive(Clone, Copy)]
struct RowCoord {
    signed_x: i32,
    signed_z: i32,
    local_x: i32,
    local_z: i32,
}

/// Direction horizontale (offset xz) + rotations.
#[derive(Clone, Copy, PartialEq)]
struct Dir(i32, i32);
const HFACES: [Dir; 4] = [Dir(0, -1), Dir(1, 0), Dir(0, 1), Dir(-1, 0)]; // N,E,S,W
impl Dir {
    fn opposite(self) -> Dir {
        Dir(-self.0, -self.1)
    }
    fn rotate_y(self) -> Dir {
        Dir(-self.1, self.0)
    }
}

/// Point d'attache de feuillage (mirroir de `FoliageAttachment`).
#[derive(Clone, Copy)]
struct Foliage {
    x: i32,
    y: i32,
    z: i32,
    radius_offset: i32,
    double_trunk: bool,
}

/// Rangée de feuilles générique (port de `placeLeavesRow`). `skip(rng, coord,
/// local_y, range, large)` décide d'omettre une cellule.
#[allow(clippy::too_many_arguments)]
fn leaves_row(
    ctx: &mut Ctx,
    rng: &mut Random,
    x: i32,
    y: i32,
    z: i32,
    range: i32,
    local_y: i32,
    large: bool,
    leaf: u32,
    skip: impl Fn(&mut Random, RowCoord, i32, i32, bool) -> bool,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let extra = i32::from(large);
    for sx in -range..=range + extra {
        for sz in -range..=range + extra {
            let local_x = if large {
                sx.abs().min((sx - 1).abs())
            } else {
                sx.abs()
            };
            let local_z = if large {
                sz.abs().min((sz - 1).abs())
            } else {
                sz.abs()
            };
            let coord = RowCoord {
                signed_x: sx,
                signed_z: sz,
                local_x,
                local_z,
            };
            if !skip(rng, coord, local_y, range, large) {
                ctx.try_leaf(x + sx, y + local_y, z + sz, leaf, placed);
            }
        }
    }
}

// ── Foliage placers ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn blob_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_height: i32,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    for local_y in (offset - foliage_height..=offset).rev() {
        let range = (foliage_radius + a.radius_offset - 1 - local_y / 2).max(0);
        leaves_row(
            ctx,
            rng,
            a.x,
            a.y,
            a.z,
            range,
            local_y,
            a.double_trunk,
            leaf,
            |rng, c, row_y, rr, _large| {
                c.local_x == rr && c.local_z == rr && (rng.next_bounded_int(2) == 0 || row_y == 0)
            },
            placed,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn fancy_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_height: i32,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    for local_y in (offset - foliage_height..=offset).rev() {
        let range =
            foliage_radius + i32::from(local_y != offset && local_y != offset - foliage_height);
        leaves_row(
            ctx,
            rng,
            a.x,
            a.y,
            a.z,
            range,
            local_y,
            a.double_trunk,
            leaf,
            |_rng, c, _ly, rr, _large| {
                let dx = c.local_x as f32 + 0.5;
                let dz = c.local_z as f32 + 0.5;
                dx * dx + dz * dz > (rr * rr) as f32
            },
            placed,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spruce_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_height: i32,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let mut range = rng.next_bounded_int(2);
    let mut max_range = 1;
    let mut reset_range = 0;
    for local_y in (-foliage_height..=offset).rev() {
        leaves_row(
            ctx,
            rng,
            a.x,
            a.y,
            a.z,
            range,
            local_y,
            a.double_trunk,
            leaf,
            |_rng, c, _ly, rr, _large| c.local_x == rr && c.local_z == rr && rr > 0,
            placed,
        );
        if range >= max_range {
            range = reset_range;
            reset_range = 1;
            max_range = (max_range + 1).min(foliage_radius + a.radius_offset);
        } else {
            range += 1;
        }
    }
}

fn dark_oak_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let (bx, by, bz) = (a.x, a.y + offset, a.z);
    let skip = |_rng: &mut Random, c: RowCoord, row_y: i32, rr: i32, large: bool| -> bool {
        if row_y == 0
            && large
            && (c.signed_x == -rr || c.signed_x >= rr)
            && (c.signed_z == -rr || c.signed_z >= rr)
        {
            return true;
        }
        if row_y == -1 && !large {
            return c.local_x == rr && c.local_z == rr;
        }
        row_y == 1 && c.local_x + c.local_z > rr * 2 - 2
    };
    if a.double_trunk {
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            foliage_radius + 2,
            -1,
            true,
            leaf,
            skip,
            placed,
        );
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            foliage_radius + 3,
            0,
            true,
            leaf,
            skip,
            placed,
        );
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            foliage_radius + 2,
            1,
            true,
            leaf,
            skip,
            placed,
        );
        if rng.next_bounded_int(2) == 0 {
            leaves_row(
                ctx,
                rng,
                bx,
                by,
                bz,
                foliage_radius,
                2,
                true,
                leaf,
                skip,
                placed,
            );
        }
    } else {
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            foliage_radius + 2,
            -1,
            false,
            leaf,
            skip,
            placed,
        );
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            foliage_radius + 1,
            0,
            false,
            leaf,
            skip,
            placed,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn mega_pine_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_height: i32,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let mut previous_range = 0;
    for current_y in (a.y - foliage_height + offset)..=(a.y + offset) {
        let height_from_top = a.y - current_y;
        let range = foliage_radius
            + a.radius_offset
            + (height_from_top as f32 / foliage_height as f32 * 3.5).floor() as i32;
        let actual_range = if height_from_top > 0 && range == previous_range && (current_y & 1) == 0
        {
            range + 1
        } else {
            range
        };
        leaves_row(
            ctx,
            rng,
            a.x,
            current_y,
            a.z,
            actual_range,
            0,
            a.double_trunk,
            leaf,
            |_rng, c, _ly, rr, _large| {
                c.local_x + c.local_z >= 7
                    || c.local_x * c.local_x + c.local_z * c.local_z > rr * rr
            },
            placed,
        );
        previous_range = range;
    }
}

#[allow(clippy::too_many_arguments)]
fn mega_jungle_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_height: i32,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let layers = if a.double_trunk {
        foliage_height
    } else {
        1 + rng.next_bounded_int(2)
    };
    for local_y in (offset - layers..=offset).rev() {
        let range = foliage_radius + a.radius_offset + 1 - local_y;
        leaves_row(
            ctx,
            rng,
            a.x,
            a.y,
            a.z,
            range,
            local_y,
            a.double_trunk,
            leaf,
            |_rng, c, _ly, rr, _large| {
                c.local_x + c.local_z >= 7
                    || c.local_x * c.local_x + c.local_z * c.local_z > rr * rr
            },
            placed,
        );
    }
}

fn acacia_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    foliage_radius: i32,
    offset: i32,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let (bx, by, bz) = (a.x, a.y + offset, a.z);
    let large = a.double_trunk;
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        foliage_radius + a.radius_offset,
        -1,
        large,
        leaf,
        |_rng, c, _ly, rr, _l| c.local_x == rr && c.local_z == rr && rr > 0,
        placed,
    );
    let ring = |_rng: &mut Random, c: RowCoord, _ly: i32, _rr: i32, _l: bool| {
        (c.local_x > 1 || c.local_z > 1) && c.local_x != 0 && c.local_z != 0
    };
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        foliage_radius - 1,
        0,
        large,
        leaf,
        ring,
        placed,
    );
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        foliage_radius + a.radius_offset - 1,
        0,
        large,
        leaf,
        ring,
        placed,
    );
}

// ── Trunk helpers ────────────────────────────────────────────────────────

/// Hauteur (mirroir de `calculateHeight`).
fn calc_height(rng: &mut Random, base: i32, rand_a: i32, rand_b: i32) -> i32 {
    base + rng.next_bounded_int(rand_a + 1) + rng.next_bounded_int(rand_b + 1)
}

fn place_dirt_under(ctx: &mut Ctx, x: i32, y: i32, z: i32) {
    let below = ctx.get(x, y, z);
    if below == ctx.tb.grass_block {
        ctx.set(x, y, z, ctx.tb.dirt);
    }
}

/// Trait d'union (mirroir de `makeLimb`, mode `place`). Monde virtuel = air, donc
/// toujours posable ; on écrit clippé.
fn make_limb(
    ctx: &mut Ctx,
    from: (i32, i32, i32),
    to: (i32, i32, i32),
    log: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let (dx, dy, dz) = (to.0 - from.0, to.1 - from.1, to.2 - from.2);
    let steps = dx.abs().max(dy.abs()).max(dz.abs());
    if steps == 0 {
        ctx.place_log(from.0, from.1, from.2, log, placed);
        return;
    }
    let (sx, sy, sz) = (
        dx as f32 / steps as f32,
        dy as f32 / steps as f32,
        dz as f32 / steps as f32,
    );
    for i in 0..=steps {
        let cx = from.0 + (0.5 + i as f32 * sx).floor() as i32;
        let cy = from.1 + (0.5 + i as f32 * sy).floor() as i32;
        let cz = from.2 + (0.5 + i as f32 * sz).floor() as i32;
        ctx.place_log(cx, cy, cz, log, placed);
    }
}

/// Podzol sous un méga conifère (mirroir simplifié de `placePodzol`).
fn place_podzol(ctx: &mut Ctx, rng: &mut Random, logs: &[(i32, i32, i32)]) {
    let min_y = logs.iter().map(|l| l.1).min().unwrap_or(i32::MIN);
    let base: Vec<_> = logs.iter().filter(|l| l.1 == min_y).copied().collect();
    for (bx, by, bz) in base {
        for (cx, cz) in [
            (bx - 1, bz - 1),
            (bx + 2, bz - 1),
            (bx - 1, bz + 2),
            (bx + 2, bz + 2),
        ] {
            podzol_circle(ctx, cx, by, cz);
        }
        for _ in 0..5 {
            let v = rng.next_bounded_int(64);
            let (xo, zo) = (v % 8, v / 8);
            if xo == 0 || xo == 7 || zo == 0 || zo == 7 {
                podzol_circle(ctx, bx - 3 + xo, by, bz - 3 + zo);
            }
        }
    }
}

fn podzol_circle(ctx: &mut Ctx, x: i32, y: i32, z: i32) {
    for dx in -2..=2i32 {
        for dz in -2..=2i32 {
            if dx.abs() != 2 || dz.abs() != 2 {
                podzol_at(ctx, x + dx, y, z + dz);
            }
        }
    }
}

fn podzol_at(ctx: &mut Ctx, x: i32, z_y: i32, z: i32) {
    for dy in (-3..=2).rev() {
        let cy = z_y + dy;
        let b = ctx.get(x, cy, z);
        if b == ctx.tb.grass_block || b == ctx.tb.dirt {
            ctx.set(x, cy, z, ctx.tb.podzol);
            return;
        }
        if b != ctx.tb.air && dy < 0 {
            return;
        }
    }
}

/// Lianes pendantes sur des feuilles (mirroir de `placeLeafVines`).
fn place_leaf_vines(ctx: &mut Ctx, rng: &mut Random, leaves: &[(i32, i32, i32)], prob: f64) {
    let vine = ctx.tb.vine;
    for &(x, y, z) in leaves {
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if rng.next_float() < prob && ctx.get(x + dx, y, z + dz) == ctx.tb.air {
                ctx.set(x + dx, y, z + dz, vine);
                let mut i = 1;
                while i <= 4 && ctx.get(x + dx, y - i, z + dz) == ctx.tb.air {
                    ctx.set(x + dx, y - i, z + dz, vine);
                    i += 1;
                }
            }
        }
    }
}

// ── Espèces ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn straight_blob(
    ctx: &mut Ctx,
    rng: &mut Random,
    x: i32,
    y: i32,
    z: i32,
    log: u32,
    leaf: u32,
    base: i32,
    rand_a: i32,
    rand_b: i32,
) {
    let h = calc_height(rng, base, rand_a, rand_b);
    place_dirt_under(ctx, x, y - 1, z);
    let mut logs = Vec::new();
    for dy in 0..h {
        ctx.place_log(x, y + dy, z, log, &mut logs);
    }
    let mut leaves = Vec::new();
    blob_foliage(
        ctx,
        rng,
        Foliage {
            x,
            y: y + h,
            z,
            radius_offset: 0,
            double_trunk: false,
        },
        3,
        2,
        0,
        leaf,
        &mut leaves,
    );
}

fn fancy_oak(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let height = calc_height(rng, 3, 11, 0);
    let trunk_and_foliage = height + 2;
    let trunk_top_offset = (trunk_and_foliage as f64 * 0.618).floor() as i32;
    let branch_base_limit = y + trunk_top_offset;
    let foliage_start = trunk_and_foliage - 5;
    let mut logs = Vec::new();

    // Attaches de feuillage : (attachment xyz, branch_base).
    let mut coords: Vec<((i32, i32, i32), i32)> =
        vec![((x, y + foliage_start, z), branch_base_limit)];
    for current in (0..=foliage_start).rev() {
        let shape = fancy_shape(trunk_and_foliage, current);
        if shape < 0.0 {
            continue;
        }
        let branch_len = shape as f64 * (rng.next_float() + 0.328);
        let angle = rng.next_float() * std::f64::consts::PI * 2.0;
        let bx = x + (branch_len * angle.sin() + 0.5).floor() as i32;
        let bz = z + (branch_len * angle.cos() + 0.5).floor() as i32;
        let foliage_pos = (bx, y + current - 1, bz);
        let (xd, zd) = (x - bx, z - bz);
        let branch_base_y = foliage_pos.1 as f64 - ((xd * xd + zd * zd) as f64).sqrt() * 0.381;
        let attach_base = if branch_base_y > branch_base_limit as f64 {
            branch_base_limit
        } else {
            branch_base_y as i32
        };
        coords.push((foliage_pos, attach_base));
    }

    place_dirt_under(ctx, x, y - 1, z);
    make_limb(ctx, (x, y, z), (x, y + trunk_top_offset, z), log, &mut logs);

    // Branches (limbs) — gardées par `trim`, COMME le feuillage, et incluant la
    // couronne sommitale (depuis le haut du tronc) : indispensable, sinon le
    // feuillage du sommet flotte au-dessus du tronc (port fidèle d'Allay).
    for &(attach, branch_base) in &coords {
        if fancy_trim(trunk_and_foliage, branch_base - y) {
            let branch_start = (x, branch_base, z);
            if branch_start != attach {
                make_limb(ctx, branch_start, attach, log, &mut logs);
            }
        }
    }

    let mut leaves = Vec::new();
    for &(attach, branch_base) in &coords {
        if fancy_trim(trunk_and_foliage, branch_base - y) {
            fancy_foliage(
                ctx,
                rng,
                Foliage {
                    x: attach.0,
                    y: attach.1,
                    z: attach.2,
                    radius_offset: 0,
                    double_trunk: false,
                },
                4,
                2,
                4,
                leaf,
                &mut leaves,
            );
        }
    }
}

fn fancy_trim(max_height: i32, current: i32) -> bool {
    current as f32 >= max_height as f32 * 0.2
}

fn fancy_shape(height: i32, current_y: i32) -> f32 {
    if (current_y as f32) < height as f32 * 0.3 {
        return -1.0;
    }
    let midpoint = height as f32 / 2.0;
    let from_mid = midpoint - current_y as f32;
    if from_mid.abs() >= midpoint {
        return 0.0;
    }
    let radius = if from_mid == 0.0 {
        midpoint
    } else {
        (midpoint * midpoint - from_mid * from_mid).sqrt()
    };
    radius * 0.5
}

fn spruce(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let h = calc_height(rng, 5, 2, 1);
    place_dirt_under(ctx, x, y - 1, z);
    let mut logs = Vec::new();
    for dy in 0..h {
        ctx.place_log(x, y + dy, z, log, &mut logs);
    }
    let foliage_height = 4.max(h - (1 + rng.next_bounded_int(2)));
    let foliage_radius = 2 + rng.next_bounded_int(2);
    let offset = rng.next_bounded_int(3);
    let mut leaves = Vec::new();
    spruce_foliage(
        ctx,
        rng,
        Foliage {
            x,
            y: y + h,
            z,
            radius_offset: 0,
            double_trunk: false,
        },
        foliage_height,
        foliage_radius,
        offset,
        leaf,
        &mut leaves,
    );
}

fn acacia(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let h = calc_height(rng, 5, 2, 2);
    place_dirt_under(ctx, x, y - 1, z);
    let mut logs = Vec::new();
    let mut attachments = Vec::new();
    let (mut cx, mut cz) = (x, z);
    let primary = HFACES[rng.next_bounded_int(4) as usize];
    let bend_start = h - rng.next_bounded_int(4) - 1;
    let mut bend_len = 3 - rng.next_bounded_int(3);
    let mut last_y = y;
    for dy in 0..h {
        let log_y = y + dy;
        if dy >= bend_start && bend_len > 0 {
            cx += primary.0;
            cz += primary.1;
            bend_len -= 1;
        }
        ctx.place_log(cx, log_y, cz, log, &mut logs);
        last_y = log_y + 1;
    }
    attachments.push(Foliage {
        x: cx,
        y: last_y,
        z: cz,
        radius_offset: 1,
        double_trunk: false,
    });

    let secondary = HFACES[rng.next_bounded_int(4) as usize];
    if secondary != primary {
        let secondary_start = bend_start - rng.next_bounded_int(2) - 1;
        let mut secondary_len = 1 + rng.next_bounded_int(3);
        let (mut sx, mut sz) = (x, z);
        let mut top_y = -1;
        let mut dy = secondary_start;
        while dy < h && secondary_len > 0 {
            if dy >= 1 {
                let log_y = y + dy;
                sx += secondary.0;
                sz += secondary.1;
                ctx.place_log(sx, log_y, sz, log, &mut logs);
                top_y = log_y + 1;
            }
            secondary_len -= 1;
            dy += 1;
        }
        if top_y >= 0 {
            attachments.push(Foliage {
                x: sx,
                y: top_y,
                z: sz,
                radius_offset: 0,
                double_trunk: false,
            });
        }
    }
    let mut leaves = Vec::new();
    for a in attachments {
        acacia_foliage(ctx, rng, a, 2, 0, leaf, &mut leaves);
    }
}

#[allow(clippy::too_many_arguments)]
fn mega_conical(
    ctx: &mut Ctx,
    rng: &mut Random,
    x: i32,
    y: i32,
    z: i32,
    log: u32,
    leaf: u32,
    foliage_base: i32,
    foliage_rand: i32,
) {
    let h = calc_height(rng, 13, 2, 14);
    for dx in 0..=1 {
        for dz in 0..=1 {
            place_dirt_under(ctx, x + dx, y - 1, z + dz);
        }
    }
    let mut logs = Vec::new();
    for dy in 0..h {
        ctx.place_log(x, y + dy, z, log, &mut logs);
        if dy < h - 1 {
            ctx.place_log(x + 1, y + dy, z, log, &mut logs);
            ctx.place_log(x + 1, y + dy, z + 1, log, &mut logs);
            ctx.place_log(x, y + dy, z + 1, log, &mut logs);
        }
    }
    let crown = foliage_base + rng.next_bounded_int(foliage_rand);
    let mut leaves = Vec::new();
    mega_pine_foliage(
        ctx,
        rng,
        Foliage {
            x,
            y: y + h,
            z,
            radius_offset: 0,
            double_trunk: true,
        },
        crown,
        0,
        0,
        leaf,
        &mut leaves,
    );
    place_podzol(ctx, rng, &logs);
}

fn double_trunk(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let h = calc_height(rng, 6, 2, 1);
    for dx in 0..=1 {
        for dz in 0..=1 {
            place_dirt_under(ctx, x + dx, y - 1, z + dz);
        }
    }
    let mut logs = Vec::new();
    let mut attachments = Vec::new();
    let bend = HFACES[rng.next_bounded_int(4) as usize];
    let bend_start = h - rng.next_bounded_int(4);
    let mut bend_len = 2 - rng.next_bounded_int(3);
    let (mut cx, mut cz) = (x, z);
    let top_y = y + h - 1;
    for dy in 0..h {
        if dy >= bend_start && bend_len > 0 {
            cx += bend.0;
            cz += bend.1;
            bend_len -= 1;
        }
        let log_y = y + dy;
        ctx.place_log(cx, log_y, cz, log, &mut logs);
        ctx.place_log(cx + 1, log_y, cz, log, &mut logs);
        ctx.place_log(cx, log_y, cz + 1, log, &mut logs);
        ctx.place_log(cx + 1, log_y, cz + 1, log, &mut logs);
    }
    attachments.push(Foliage {
        x: cx,
        y: top_y,
        z: cz,
        radius_offset: 0,
        double_trunk: true,
    });
    for dx in -1..=2 {
        for dz in -1..=2 {
            if (!(0..=1).contains(&dx) || !(0..=1).contains(&dz)) && rng.next_bounded_int(3) == 0 {
                let blen = rng.next_bounded_int(3) + 2;
                for i in 0..blen {
                    ctx.place_log(x + dx, top_y - i - 1, z + dz, log, &mut logs);
                }
                attachments.push(Foliage {
                    x: x + dx,
                    y: top_y,
                    z: z + dz,
                    radius_offset: 0,
                    double_trunk: false,
                });
            }
        }
    }
    let mut leaves = Vec::new();
    for a in attachments {
        dark_oak_foliage(ctx, rng, a, 0, 0, leaf, &mut leaves);
    }
}

fn mega_jungle(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let h = calc_height(rng, 10, 2, 19);
    for dx in 0..=1 {
        for dz in 0..=1 {
            place_dirt_under(ctx, x + dx, y - 1, z + dz);
        }
    }
    let mut logs = Vec::new();
    let mut attachments = Vec::new();
    for dy in 0..h {
        ctx.place_log(x, y + dy, z, log, &mut logs);
        if dy < h - 1 {
            ctx.place_log(x + 1, y + dy, z, log, &mut logs);
            ctx.place_log(x + 1, y + dy, z + 1, log, &mut logs);
            ctx.place_log(x, y + dy, z + 1, log, &mut logs);
        }
    }
    attachments.push(Foliage {
        x,
        y: y + h,
        z,
        radius_offset: 0,
        double_trunk: true,
    });
    let mut branch_start = h - 2 - rng.next_bounded_int(4);
    while branch_start > h / 2 {
        let angle = rng.next_float() * std::f64::consts::PI * 2.0;
        let (mut bx, mut bz) = (0, 0);
        for i in 0..5 {
            bx = (1.5 + angle.cos() * i as f64) as i32;
            bz = (1.5 + angle.sin() * i as f64) as i32;
            ctx.place_log(x + bx, y + branch_start - 3 + i / 2, z + bz, log, &mut logs);
        }
        attachments.push(Foliage {
            x: x + bx,
            y: y + branch_start,
            z: z + bz,
            radius_offset: -2,
            double_trunk: false,
        });
        branch_start -= 2 + rng.next_bounded_int(4);
    }
    let mut leaves = Vec::new();
    for a in attachments {
        mega_jungle_foliage(ctx, rng, a, 2, 2, 0, leaf, &mut leaves);
    }
    place_leaf_vines(ctx, rng, &leaves, 0.25);
}

#[allow(clippy::too_many_arguments)]
fn cherry(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let h = calc_height(rng, 7, 1, 0);
    place_dirt_under(ctx, x, y - 1, z);
    let first_start = 0.max(h - 1 + uniform(rng, -4, -3));
    let mut second_start = 0.max(h - 1 - 4);
    if second_start >= first_start {
        second_start += 1;
    }
    let branch_count = rng.next_bounded_int(3) + 1;
    let place_top = branch_count == 3;
    let place_second = branch_count >= 2;
    let trunk_height = if place_top {
        h
    } else if place_second {
        first_start.max(second_start) + 1
    } else {
        first_start + 1
    };
    let mut logs = Vec::new();
    for dy in 0..trunk_height {
        ctx.place_log(x, y + dy, z, log, &mut logs);
    }
    let mut attachments = Vec::new();
    if place_top {
        attachments.push(Foliage {
            x,
            y: y + trunk_height,
            z,
            radius_offset: 0,
            double_trunk: false,
        });
    }
    let dir = HFACES[rng.next_bounded_int(4) as usize];
    attachments.push(cherry_branch(
        ctx,
        rng,
        x,
        y,
        z,
        h,
        dir,
        first_start,
        &mut logs,
    ));
    if place_second {
        attachments.push(cherry_branch(
            ctx,
            rng,
            x,
            y,
            z,
            h,
            dir.opposite(),
            second_start,
            &mut logs,
        ));
    }
    let mut leaves = Vec::new();
    for a in attachments {
        cherry_foliage(ctx, rng, a, leaf, &mut leaves);
    }
}

#[allow(clippy::too_many_arguments)]
fn cherry_branch(
    ctx: &mut Ctx,
    rng: &mut Random,
    x: i32,
    y: i32,
    z: i32,
    tree_height: i32,
    dir: Dir,
    branch_start: i32,
    logs: &mut Vec<(i32, i32, i32)>,
) -> Foliage {
    let (mut cx, mut cy, mut cz) = (x, y + branch_start, z);
    let branch_end_y = tree_height - 1 + uniform(rng, -1, 0);
    let extended = branch_end_y < branch_start;
    let horizontal_len = uniform(rng, 2, 4) + i32::from(extended);
    let target_x = x + dir.0 * horizontal_len;
    let target_y = y + branch_end_y;
    let target_z = z + dir.1 * horizontal_len;
    let first_steps = if extended { 2 } else { 1 };
    for _ in 0..first_steps {
        cx += dir.0;
        cz += dir.1;
        ctx.place_log(cx, cy, cz, ctx.tb.cherry_log, logs);
    }
    let vstep = if target_y > cy { 1 } else { -1 };
    loop {
        let dist = (target_x - cx).abs() + (target_y - cy).abs() + (target_z - cz).abs();
        if dist == 0 {
            return Foliage {
                x: target_x,
                y: target_y + 1,
                z: target_z,
                radius_offset: 0,
                double_trunk: false,
            };
        }
        let vchance = (target_y - cy).abs() as f64 / dist as f64;
        if rng.next_float() < vchance {
            cy += vstep;
        } else {
            cx += dir.0;
            cz += dir.1;
        }
        ctx.place_log(cx, cy, cz, ctx.tb.cherry_log, logs);
    }
}

fn cherry_foliage(
    ctx: &mut Ctx,
    rng: &mut Random,
    a: Foliage,
    leaf: u32,
    placed: &mut Vec<(i32, i32, i32)>,
) {
    let foliage_height = 5;
    let range = 4 + a.radius_offset - 1;
    let (bx, by, bz) = (a.x, a.y, a.z);
    let skip = |rng: &mut Random, c: RowCoord, local_y: i32, rr: i32, _l: bool| -> bool {
        if local_y == -1 && (c.local_x == rr || c.local_z == rr) && rng.next_float() < 0.25 {
            return true;
        }
        let corner = c.local_x == rr && c.local_z == rr;
        if rr > 2 {
            corner || (c.local_x + c.local_z > rr * 2 - 2 && rng.next_float() < 0.25)
        } else {
            corner && rng.next_float() < 0.25
        }
    };
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        range - 2,
        foliage_height - 3,
        a.double_trunk,
        leaf,
        skip,
        placed,
    );
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        range - 1,
        foliage_height - 4,
        a.double_trunk,
        leaf,
        skip,
        placed,
    );
    for local_y in (0..=foliage_height - 5).rev() {
        leaves_row(
            ctx,
            rng,
            bx,
            by,
            bz,
            range,
            local_y,
            a.double_trunk,
            leaf,
            skip,
            placed,
        );
    }
    // Rangées basses (les feuilles pendantes vanilla sont approximées par ces
    // deux rangées ; le détail des hanging leaves est porté plus tard).
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        range,
        -1,
        a.double_trunk,
        leaf,
        skip,
        placed,
    );
    leaves_row(
        ctx,
        rng,
        bx,
        by,
        bz,
        range - 1,
        -2,
        a.double_trunk,
        leaf,
        skip,
        placed,
    );
}

fn azalea(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32) {
    let h = calc_height(rng, 4, 2, 0);
    ctx.set(x, y - 1, z, ctx.tb.dirt_with_roots);
    let mut attachments = Vec::new();
    let mut logs = Vec::new();
    let bend = HFACES[rng.next_bounded_int(4) as usize];
    let top = h - 1;
    let (mut cx, mut cz) = (x, z);
    for i in 0..=top {
        if i + 1 >= top + rng.next_bounded_int(2) {
            cx += bend.0;
            cz += bend.1;
        }
        ctx.place_log(cx, y + i, cz, ctx.tb.oak_log, &mut logs);
        if i >= 3 {
            attachments.push((cx, y + i, cz));
        }
    }
    let bend_len = 1 + rng.next_bounded_int(2);
    let bend_y = y + h;
    for _ in 0..=bend_len {
        ctx.place_log(cx, bend_y, cz, ctx.tb.oak_log, &mut logs);
        attachments.push((cx, bend_y, cz));
        cx += bend.0;
        cz += bend.1;
    }
    let mut leaves = Vec::new();
    for (ax, ay, az) in attachments {
        for _ in 0..50 {
            let tx = ax + rng.next_bounded_int(3) - rng.next_bounded_int(3);
            let ty = ay + rng.next_bounded_int(2) - rng.next_bounded_int(2);
            let tz = az + rng.next_bounded_int(3) - rng.next_bounded_int(3);
            let l = if rng.next_bounded_int(4) == 0 {
                ctx.tb.azalea_leaves_flowered
            } else {
                ctx.tb.azalea_leaves
            };
            ctx.try_leaf(tx, ty, tz, l, &mut leaves);
        }
    }
}

/// Mangrove **simplifié** (increment A) : tronc droit + feuillage random-spread +
/// court pied de racines. Le port fidèle (racines simulées, propagules, boue)
/// viendra dans un increment ultérieur.
fn mangrove_simple(ctx: &mut Ctx, rng: &mut Random, x: i32, y: i32, z: i32, log: u32, leaf: u32) {
    let roots = 1 + rng.next_bounded_int(3);
    let mut placed = Vec::new();
    for dy in 0..roots {
        ctx.place_log(x, y + dy, z, ctx.tb.mangrove_roots, &mut placed);
    }
    let h = calc_height(rng, 4, 1, 6);
    let mut logs = Vec::new();
    for dy in 0..h {
        ctx.place_log(x, y + roots + dy, z, log, &mut logs);
    }
    let mut leaves = Vec::new();
    let a = Foliage {
        x,
        y: y + roots + h,
        z,
        radius_offset: 0,
        double_trunk: false,
    };
    // random_spread foliage (foliage_height 2, radius 3, 70 essais).
    for _ in 0..70 {
        let tx = a.x + rng.next_bounded_int(3) - rng.next_bounded_int(3);
        let ty = a.y + rng.next_bounded_int(2) - rng.next_bounded_int(2);
        let tz = a.z + rng.next_bounded_int(3) - rng.next_bounded_int(3);
        ctx.try_leaf(tx, ty, tz, leaf, &mut leaves);
    }
    place_leaf_vines(ctx, rng, &leaves, 0.125);
}

#[inline]
fn uniform(rng: &mut Random, min: i32, max: i32) -> i32 {
    rng.next_range(min, max)
}

/// Pose un arbre de l'espèce donnée, origine au sol `(x, ground, z)` (coords
/// **locales au chunk cible**, hors-bornes clippées). Le 1er log est en
/// `ground + 1`.
pub(super) fn place(
    grid: &mut [u32],
    rng: &mut Random,
    species: Species,
    x: i32,
    ground: i32,
    z: i32,
    // Surface du monde (Y) à des coords LOCALES au chunk cible — pour la décay :
    // un log sous la surface serait mangé par le terrain (d'ici ou d'un chunk
    // voisin) et ne compte donc pas comme support de feuille.
    surface_at: &dyn Fn(i32, i32) -> i32,
) {
    let tb: &TreeBlocks = &TB;
    let mut ctx = Ctx {
        grid,
        tb,
        logs: Vec::new(),
        leaves: Vec::new(),
    };
    let y = ground + 1;
    match species {
        Species::Oak => straight_blob(&mut ctx, rng, x, y, z, tb.oak_log, tb.oak_leaves, 4, 2, 0),
        Species::Birch => straight_blob(
            &mut ctx,
            rng,
            x,
            y,
            z,
            tb.birch_log,
            tb.birch_leaves,
            5,
            2,
            0,
        ),
        // Super bouleau (old growth) = bouleau haut.
        Species::SuperBirch => straight_blob(
            &mut ctx,
            rng,
            x,
            y,
            z,
            tb.birch_log,
            tb.birch_leaves,
            5,
            2,
            6,
        ),
        Species::JungleTree => straight_blob(
            &mut ctx,
            rng,
            x,
            y,
            z,
            tb.jungle_log,
            tb.jungle_leaves,
            4,
            8,
            0,
        ),
        Species::JungleBush => {
            // Buisson : 1 log + petite grappe.
            let mut logs = Vec::new();
            ctx.place_log(x, y, z, tb.jungle_log, &mut logs);
            let mut leaves = Vec::new();
            blob_foliage(
                &mut ctx,
                rng,
                Foliage {
                    x,
                    y: y + 1,
                    z,
                    radius_offset: 0,
                    double_trunk: false,
                },
                1,
                2,
                0,
                tb.jungle_leaves,
                &mut leaves,
            );
        }
        Species::FancyOak => fancy_oak(&mut ctx, rng, x, y, z, tb.oak_log, tb.oak_leaves),
        // Pin : Allay (Bedrock) ne distingue pas le pin du sapin → sapin.
        Species::Spruce | Species::Pine => {
            spruce(&mut ctx, rng, x, y, z, tb.spruce_log, tb.spruce_leaves)
        }
        Species::MegaSpruce => mega_conical(
            &mut ctx,
            rng,
            x,
            y,
            z,
            tb.spruce_log,
            tb.spruce_leaves,
            13,
            5,
        ),
        Species::MegaJungle => mega_jungle(&mut ctx, rng, x, y, z, tb.jungle_log, tb.jungle_leaves),
        Species::DarkOak => {
            double_trunk(&mut ctx, rng, x, y, z, tb.dark_oak_log, tb.dark_oak_leaves)
        }
        Species::Acacia => acacia(&mut ctx, rng, x, y, z, tb.acacia_log, tb.acacia_leaves),
        Species::Cherry => cherry(&mut ctx, rng, x, y, z, tb.cherry_log, tb.cherry_leaves),
        Species::Mangrove => {
            mangrove_simple(&mut ctx, rng, x, y, z, tb.mangrove_log, tb.mangrove_leaves)
        }
    }
    // Azalée n'est pas dans `Species` (réservée aux grottes luxuriantes) ; gérée
    // ailleurs si besoin. `azalea` est exposée pour un usage futur.
    let _ = azalea;

    // ── Décay des feuilles orphelines (règle Minecraft : une feuille à plus de
    // 6 blocs de TOUT log décline). Garde-fou universel contre les feuilles
    // flottantes (déterministe ; les logs « virtuels » clippés sont inclus, donc
    // cohérent cross-chunk). Bon marché : la quasi-totalité des feuilles trouvent
    // un log immédiatement (early-exit). ──
    let logs = std::mem::take(&mut ctx.logs);
    let leaves = std::mem::take(&mut ctx.leaves);
    for (lx, ly, lz) in leaves {
        let near = logs.iter().any(|&(gx, gy, gz)| {
            gy > surface_at(gx, gz) && (gx - lx).abs() + (gy - ly).abs() + (gz - lz).abs() <= 6
        });
        if !near {
            if let Some(i) = ctx.idx(lx, ly, lz) {
                let cur = ctx.grid[i];
                if ctx.is_leaf(cur) {
                    ctx.grid[i] = ctx.tb.air;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise_chunk::{grid_index, GRID_LEN, MAX_Y, MIN_Y};
    use super::*;

    /// Place une espèce isolée sur sol plat et compte les feuilles sans aucun log
    /// à ≤ 6 (taxicab) — un arbre bien formé ne doit pas en avoir (hors clip de
    /// bord). Détecte les bugs de FORME (feuilles déconnectées du tronc).
    #[test]
    fn no_floating_leaves_in_isolation() {
        let tb: &TreeBlocks = &TB;
        let leaves_set: std::collections::HashSet<u32> = [
            tb.oak_leaves,
            tb.birch_leaves,
            tb.spruce_leaves,
            tb.jungle_leaves,
            tb.acacia_leaves,
            tb.dark_oak_leaves,
            tb.cherry_leaves,
            tb.mangrove_leaves,
        ]
        .into_iter()
        .collect();
        let log_set: std::collections::HashSet<u32> = [
            tb.oak_log,
            tb.birch_log,
            tb.spruce_log,
            tb.jungle_log,
            tb.acacia_log,
            tb.dark_oak_log,
            tb.cherry_log,
            tb.mangrove_log,
            tb.mangrove_roots,
        ]
        .into_iter()
        .collect();
        let species = [
            Species::MegaJungle,
            Species::MegaSpruce,
            Species::FancyOak,
            Species::DarkOak,
            Species::Acacia,
            Species::Cherry,
            Species::JungleTree,
            Species::Oak,
            Species::Spruce,
        ];
        let mut worst: Option<(Species, i64, usize)> = None;
        for &sp in &species {
            for seed in 0..150i64 {
                let mut grid = vec![tb.air; GRID_LEN].into_boxed_slice();
                let mut rng = Random::new(seed);
                place(&mut grid, &mut rng, sp, 8, 70, 8, &|_, _| i32::MIN);
                // Indexe logs + leaves (coords locales).
                let mut logs = std::collections::HashSet::new();
                let mut leaves = Vec::new();
                for lx in 0..16i32 {
                    for lz in 0..16i32 {
                        for wy in MIN_Y..MAX_Y {
                            let b = grid[grid_index(lx as usize, wy, lz as usize)];
                            if log_set.contains(&b) {
                                logs.insert((lx, wy, lz));
                            } else if leaves_set.contains(&b) {
                                leaves.push((lx, wy, lz));
                            }
                        }
                    }
                }
                let mut floaters = 0;
                for &(lx, wy, lz) in &leaves {
                    // hors bord (évite les faux positifs de clip 0..16).
                    if !(3..13).contains(&lx) || !(3..13).contains(&lz) {
                        continue;
                    }
                    let near = (-6..=6i32).any(|dx| {
                        (-6..=6i32).any(|dy| {
                            (-6..=6i32).any(|dz| {
                                dx.abs() + dy.abs() + dz.abs() <= 6
                                    && logs.contains(&(lx + dx, wy + dy, lz + dz))
                            })
                        })
                    });
                    if !near {
                        floaters += 1;
                    }
                }
                if floaters > 0 && worst.is_none_or(|(_, _, w)| floaters > w) {
                    worst = Some((sp, seed, floaters));
                }
            }
        }
        assert!(
            worst.is_none(),
            "feuilles flottantes en isolation : {worst:?}"
        );
    }

    fn count(species: Species, seed: i64) -> (usize, usize) {
        let tb: &TreeBlocks = &TB;
        let logs_set = [
            tb.oak_log,
            tb.birch_log,
            tb.spruce_log,
            tb.jungle_log,
            tb.acacia_log,
            tb.dark_oak_log,
            tb.cherry_log,
            tb.mangrove_log,
            tb.mangrove_roots,
        ];
        let leaves_set = [
            tb.oak_leaves,
            tb.birch_leaves,
            tb.spruce_leaves,
            tb.jungle_leaves,
            tb.acacia_leaves,
            tb.dark_oak_leaves,
            tb.cherry_leaves,
            tb.mangrove_leaves,
        ];
        let mut grid = vec![tb.air; GRID_LEN].into_boxed_slice();
        let mut rng = Random::new(seed);
        place(&mut grid, &mut rng, species, 8, 70, 8, &|_, _| i32::MIN);
        let logs = grid.iter().filter(|b| logs_set.contains(b)).count();
        let leaves = grid.iter().filter(|b| leaves_set.contains(b)).count();
        (logs, leaves)
    }

    #[test]
    fn every_species_places_logs_and_leaves() {
        for sp in [
            Species::Oak,
            Species::Birch,
            Species::SuperBirch,
            Species::JungleTree,
            Species::FancyOak,
            Species::Spruce,
            Species::Pine,
            Species::MegaSpruce,
            Species::MegaJungle,
            Species::DarkOak,
            Species::Acacia,
            Species::Cherry,
            Species::Mangrove,
        ] {
            let (logs, leaves) = count(sp, 12345);
            assert!(logs > 0, "{sp:?} sans tronc");
            assert!(leaves > 0, "{sp:?} sans feuilles");
        }
    }

    #[test]
    fn fancy_oak_has_much_more_foliage_than_oak() {
        // La plainte utilisateur : le fancy oak (grand chêne) doit être nettement
        // plus touffu/imposant que le chêne normal.
        let oak: usize = (0..20).map(|s| count(Species::Oak, s).1).sum();
        let fancy: usize = (0..20).map(|s| count(Species::FancyOak, s).1).sum();
        assert!(
            fancy > oak * 2,
            "fancy oak pas assez touffu (oak={oak}, fancy={fancy})"
        );
    }

    #[test]
    fn placement_is_deterministic() {
        let a = count(Species::FancyOak, 99);
        let b = count(Species::FancyOak, 99);
        assert_eq!(a, b);
    }
}
