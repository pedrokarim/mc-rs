//! Portail de l'End : formation du cadre (eyes of ender) + plateforme d'arrivée.
//!
//! Réf : Allay `BlockEndPortalBaseComponentImpl` / `BlockEndPortalFrameBaseComponentImpl`.
//! Le portail est un carré 3×3 de blocs `end_portal`, entouré de 12 `end_portal_frame`
//! (3 par côté). En vanilla il faut un eye of ender dans chacun ; faute d'exposer
//! l'état « œil » du bloc côté registre, la v1 forme le portail dès que l'anneau de
//! 12 cadres est complet au moment où le joueur insère un eye (simplification).
//!
//! Le voyage proprement dit (entrer dans un bloc `end_portal`) est géré côté
//! connexion ([`Connection::check_end_portal`]). La plateforme d'arrivée est créée par
//! [`create_end_platform`].

use super::block_registry::BLOCKS;
use super::chunk_cache::ChunkCache;

fn end_portal() -> u32 {
    BLOCKS.get("minecraft:end_portal")
}
fn end_portal_frame() -> u32 {
    BLOCKS.get("minecraft:end_portal_frame")
}

pub fn is_end_portal_block(cache: &mut ChunkCache, x: i32, y: i32, z: i32) -> bool {
    cache.get_block(x, y, z) == end_portal()
}

/// Les 12 positions de cadre autour d'un centre `(cx, cz)` au niveau `y`.
fn frame_positions(cx: i32, y: i32, cz: i32) -> [[i32; 3]; 12] {
    [
        // Côté nord (cz-2) et sud (cz+2), 3 cadres chacun.
        [cx - 1, y, cz - 2],
        [cx, y, cz - 2],
        [cx + 1, y, cz - 2],
        [cx - 1, y, cz + 2],
        [cx, y, cz + 2],
        [cx + 1, y, cz + 2],
        // Côté ouest (cx-2) et est (cx+2).
        [cx - 2, y, cz - 1],
        [cx - 2, y, cz],
        [cx - 2, y, cz + 1],
        [cx + 2, y, cz - 1],
        [cx + 2, y, cz],
        [cx + 2, y, cz + 1],
    ]
}

/// Tente de former un portail End : à partir d'un cadre cliqué `(fx, fy, fz)`,
/// cherche un centre dont l'anneau des 12 cadres est complet, puis remplit le 3×3
/// intérieur de blocs `end_portal`. Renvoie les cellules remplies, ou `None`.
pub fn try_form_end_portal(
    cache: &mut ChunkCache,
    fx: i32,
    fy: i32,
    fz: i32,
) -> Option<Vec<[i32; 3]>> {
    let frame = end_portal_frame();
    // Le centre est à au plus 2 blocs (en X et Z) du cadre cliqué.
    for cx in (fx - 2)..=(fx + 2) {
        for cz in (fz - 2)..=(fz + 2) {
            // L'intérieur 3×3 ne doit pas chevaucher un cadre : centre distinct.
            let positions = frame_positions(cx, fy, cz);
            if !positions.contains(&[fx, fy, fz]) {
                continue; // le cadre cliqué doit faire partie de cet anneau
            }
            let complete = positions
                .iter()
                .all(|p| cache.get_block(p[0], p[1], p[2]) == frame);
            if !complete {
                continue;
            }
            // Remplit le 3×3 intérieur.
            let portal = end_portal();
            let mut cells = Vec::with_capacity(9);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let c = [cx + dx, fy, cz + dz];
                    cache.set_block(c[0], c[1], c[2], portal);
                    cells.push(c);
                }
            }
            return Some(cells);
        }
    }
    None
}

/// Construit la plateforme d'obsidienne 5×5 d'arrivée dans l'End (centrée sur
/// `[100, 49, 0]`, réf Allay `createEndPlatform`) et dégage l'espace au-dessus.
/// Renvoie la position des pieds d'arrivée (`[100.5, 50, 0.5]`).
pub fn create_end_platform(cache: &mut ChunkCache) -> [f32; 3] {
    let obsidian = BLOCKS.get("minecraft:obsidian");
    let air = BLOCKS.air;
    let (px, py, pz) = (100, 49, 0);
    for dx in -2..=2 {
        for dz in -2..=2 {
            cache.set_block(px + dx, py, pz + dz, obsidian);
            // Dégage 3 blocs d'air au-dessus.
            for dy in 1..=3 {
                cache.set_block(px + dx, py + dy, pz + dz, air);
            }
        }
    }
    [px as f32 + 0.5, (py + 1) as f32, pz as f32 + 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::chunk_cache::ChunkCache;

    #[test]
    fn forms_portal_when_ring_complete() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 0);
        let frame = end_portal_frame();
        let (cx, cy, cz) = (0, 64, 0);
        for p in frame_positions(cx, cy, cz) {
            cache.set_block(p[0], p[1], p[2], frame);
        }
        // Clic sur l'un des cadres.
        let cells = try_form_end_portal(&mut cache, cx - 1, cy, cz - 2).expect("ring complete");
        assert_eq!(cells.len(), 9, "interior 3x3 = 9 end_portal cells");
        assert!(is_end_portal_block(&mut cache, cx, cy, cz));
    }

    #[test]
    fn no_portal_when_ring_incomplete() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 0);
        let frame = end_portal_frame();
        let (cx, cy, cz) = (0, 64, 0);
        let positions = frame_positions(cx, cy, cz);
        // Pose tous les cadres sauf un.
        for p in &positions[..11] {
            cache.set_block(p[0], p[1], p[2], frame);
        }
        assert!(try_form_end_portal(&mut cache, cx - 1, cy, cz - 2).is_none());
    }

    #[test]
    fn platform_is_obsidian_and_arrival_on_top() {
        let mut cache = ChunkCache::with_shared_storage(None, 1, "flat", 2);
        let arrival = create_end_platform(&mut cache);
        assert_eq!(arrival, [100.5, 50.0, 0.5]);
        assert_eq!(
            cache.get_block(100, 49, 0),
            BLOCKS.get("minecraft:obsidian")
        );
        assert_eq!(cache.get_block(100, 50, 0), BLOCKS.air);
    }
}
