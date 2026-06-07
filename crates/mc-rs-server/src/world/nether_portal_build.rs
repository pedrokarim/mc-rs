//! Construction / détection des portails du Nether dans le monde.
//!
//! Réf : Allay `NetherPortalHelper.java` + vanilla. Un portail vit dans un plan
//! vertical (axe X-Y ou Z-Y), entouré d'un cadre d'obsidienne (les 4 coins sont
//! optionnels), intérieur 2..21 large × 3..21 haut rempli de blocs `portal`.
//!
//! - [`try_ignite`] : à partir du bloc d'air où le feu est posé (flint&steel sur
//!   obsidienne), détecte un cadre valide et remplit l'intérieur de portail.
//! - [`create_portal`] : pose un portail 4×5 (intérieur 2×3) à la destination.
//! - [`is_portal_block`] / [`find_ground`] : utilitaires runtime.

use super::block_registry::BLOCKS;
use super::chunk_cache::ChunkCache;

const MIN_WIDTH: i32 = 2;
const MAX_WIDTH: i32 = 21;
const MIN_HEIGHT: i32 = 3;
const MAX_HEIGHT: i32 = 21;

#[derive(Clone, Copy)]
enum Axis {
    X,
    Z,
}

fn obsidian() -> u32 {
    BLOCKS.get("minecraft:obsidian")
}
fn portal() -> u32 {
    BLOCKS.get("minecraft:portal")
}

pub fn is_portal_block(cache: &mut ChunkCache, x: i32, y: i32, z: i32) -> bool {
    cache.get_block(x, y, z) == portal()
}

/// Tente d'allumer un portail depuis le bloc d'air `seed` (où le feu apparaît).
/// Remplit l'intérieur de blocs portail et renvoie ces cellules, ou `None` si
/// aucun cadre d'obsidienne valide n'entoure le seed.
pub fn try_ignite(cache: &mut ChunkCache, sx: i32, sy: i32, sz: i32) -> Option<Vec<[i32; 3]>> {
    for axis in [Axis::X, Axis::Z] {
        if let Some(cells) = detect_axis(cache, sx, sy, sz, axis) {
            let pid = portal();
            for c in &cells {
                cache.set_block(c[0], c[1], c[2], pid);
            }
            return Some(cells);
        }
    }
    None
}

fn step(axis: Axis) -> (i32, i32) {
    match axis {
        Axis::X => (1, 0),
        Axis::Z => (0, 1),
    }
}

/// Détecte un cadre dans le plan donné par `axis` autour du bloc d'air seed.
fn detect_axis(
    cache: &mut ChunkCache,
    sx: i32,
    sy: i32,
    sz: i32,
    axis: Axis,
) -> Option<Vec<[i32; 3]>> {
    let obs = obsidian();
    let air = BLOCKS.air;
    let (ax, az) = step(axis);

    // 1. Descendre jusqu'à la rangée du bas (air avec obsidienne juste dessous).
    let mut by = sy;
    while cache.get_block(sx, by - 1, sz) == air {
        by -= 1;
        if sy - by > MAX_HEIGHT {
            return None;
        }
    }
    if cache.get_block(sx, by - 1, sz) != obs {
        return None;
    }

    // 2. Étendue gauche/droite sur la rangée du bas (jusqu'à l'obsidienne).
    let mut left = 0;
    while cache.get_block(sx - ax * (left + 1), by, sz - az * (left + 1)) == air {
        left += 1;
        if left > MAX_WIDTH {
            return None;
        }
    }
    if cache.get_block(sx - ax * (left + 1), by, sz - az * (left + 1)) != obs {
        return None;
    }
    let mut right = 0;
    while cache.get_block(sx + ax * (right + 1), by, sz + az * (right + 1)) == air {
        right += 1;
        if right > MAX_WIDTH {
            return None;
        }
    }
    if cache.get_block(sx + ax * (right + 1), by, sz + az * (right + 1)) != obs {
        return None;
    }
    let width = left + right + 1;
    if !(MIN_WIDTH..=MAX_WIDTH).contains(&width) {
        return None;
    }
    let ox = sx - ax * left; // colonne intérieure la plus à gauche
    let oz = sz - az * left;

    // 3. Hauteur : monter tant que toutes les colonnes intérieures sont air.
    let mut height = 0;
    'h: loop {
        if height > MAX_HEIGHT {
            return None;
        }
        let y = by + height;
        for w in 0..width {
            if cache.get_block(ox + ax * w, y, oz + az * w) != air {
                break 'h;
            }
        }
        height += 1;
    }
    if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
        return None;
    }

    // 4. Cadre complet (coins exclus) : sol + plafond sous/au-dessus de chaque
    //    colonne intérieure, murs gauche/droite à chaque rangée intérieure.
    for w in 0..width {
        let fx = ox + ax * w;
        let fz = oz + az * w;
        if cache.get_block(fx, by - 1, fz) != obs {
            return None;
        }
        if cache.get_block(fx, by + height, fz) != obs {
            return None;
        }
    }
    for h in 0..height {
        let y = by + h;
        if cache.get_block(ox - ax, y, oz - az) != obs {
            return None;
        }
        if cache.get_block(ox + ax * width, y, oz + az * width) != obs {
            return None;
        }
    }

    // 5. Cellules intérieures à remplir.
    let mut cells = Vec::with_capacity((width * height) as usize);
    for w in 0..width {
        for h in 0..height {
            cells.push([ox + ax * w, by + h, oz + az * w]);
        }
    }
    Some(cells)
}

/// Construit un portail 4×5 (intérieur 2 large × 3 haut, plan X-Y) avec le sol à
/// `floor_y`, et renvoie la position d'arrivée (intérieur bas, position des pieds).
pub fn create_portal(cache: &mut ChunkCache, x: i32, floor_y: i32, z: i32) -> [f32; 3] {
    let obs = obsidian();
    let pid = portal();
    let air = BLOCKS.air;

    // Cadre : murs (x-1 et x+2), sol (floor_y-1) et plafond (floor_y+3).
    for w in -1..=2 {
        cache.set_block(x + w, floor_y - 1, z, obs);
        cache.set_block(x + w, floor_y + 3, z, obs);
    }
    for h in 0..3 {
        cache.set_block(x - 1, floor_y + h, z, obs);
        cache.set_block(x + 2, floor_y + h, z, obs);
    }
    // Intérieur : portail (2×3), et on dégage l'air devant/derrière pour ne pas
    // être suffoqué par le terrain de la dimension cible.
    for w in 0..2 {
        for h in 0..3 {
            cache.set_block(x + w, floor_y + h, z, pid);
            cache.set_block(x + w, floor_y + h, z - 1, air);
            cache.set_block(x + w, floor_y + h, z + 1, air);
        }
    }

    [x as f32 + 0.5, floor_y as f32, z as f32 + 0.5]
}

/// Cherche un Y de sol solide à (x,z) dans `cache` (de haut en bas), pour y poser
/// un portail destination. Renvoie le Y du premier bloc d'air au-dessus d'un solide.
pub fn find_ground(cache: &mut ChunkCache, dim: i32, x: i32, z: i32) -> i32 {
    let air = BLOCKS.air;
    let (min_y, max_y) = crate::dimensions::DimensionId::from_id(dim)
        .map(|d| (d.min_y(), d.max_y()))
        .unwrap_or((-64, 320));
    let mut y = max_y - 4;
    while y > min_y + 1 {
        if cache.get_block(x, y, z) == air && cache.get_block(x, y - 1, z) != air {
            return y;
        }
        y -= 1;
    }
    // Rien trouvé : pose une dalle d'obsidienne et bâtit dessus.
    let base = ((min_y + max_y) / 2).clamp(min_y + 2, max_y - 6);
    for dx in -1..=2 {
        for dz in -1..=1 {
            cache.set_block(x + dx, base - 1, z + dz, obsidian());
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::chunk_cache::ChunkCache;

    /// Construit un cadre d'obsidienne 4×5 (intérieur 2×3) dans un cache génératif
    /// flat, puis vérifie que `try_ignite` détecte le cadre et remplit 6 cellules.
    #[test]
    fn ignite_detects_valid_frame() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 0);
        let obs = obsidian();
        // Cadre plan X-Y au niveau y=70, intérieur x∈{0,1}, y∈{70,71,72}.
        let (fx, fz) = (0, 0);
        let floor = 70;
        for w in -1..=2 {
            cache.set_block(fx + w, floor - 1, fz, obs); // sol
            cache.set_block(fx + w, floor + 3, fz, obs); // plafond
        }
        for h in 0..3 {
            cache.set_block(fx - 1, floor + h, fz, obs); // mur gauche
            cache.set_block(fx + 2, floor + h, fz, obs); // mur droit
        }
        // Seed : un bloc d'air intérieur.
        let cells = try_ignite(&mut cache, fx, floor, fz).expect("frame should ignite");
        assert_eq!(cells.len(), 6, "2×3 interior = 6 portal cells");
        assert!(is_portal_block(&mut cache, fx, floor, fz));
    }

    #[test]
    fn ignite_rejects_incomplete_frame() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 0);
        let obs = obsidian();
        // Cadre sans murs → invalide.
        for w in -1..=2 {
            cache.set_block(w, 69, 0, obs);
        }
        assert!(try_ignite(&mut cache, 0, 70, 0).is_none());
    }

    #[test]
    fn create_portal_places_portal_blocks() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 1);
        let arrival = create_portal(&mut cache, 10, 64, 10);
        assert!(is_portal_block(&mut cache, 10, 64, 10));
        assert_eq!(arrival, [10.5, 64.0, 10.5]);
    }
}
