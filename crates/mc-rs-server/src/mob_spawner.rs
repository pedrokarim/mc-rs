//! Spawner naturel de mobs — port simplifié de PMMP
//! `World::tickRandomBlocks` + `EntityFactory::spawn`.
//!
//! Cycle par game tick (20 TPS) :
//!   - Pour chaque joueur connecté
//!   - Pick un mob_kind candidat parmi tout le roster (`MobKind::ALL`) pondéré
//!     par `spawn_rules_vanilla::spawn_weight` (les espèces sans règle vanilla
//!     sont automatiquement ignorées)
//!   - Pick une position dans un rayon SPAWN_RADIUS_MIN..MAX autour du joueur
//!   - Vérifie : ground solide, headroom, light level (pour monsters dans le
//!     range vanilla 0-7 via `spawn_rules_vanilla::brightness_range`)
//!   - Vérifie le mob cap par catégorie
//!   - Spawn via `MobEntityManager::spawn` (~5 % d'animaux en bébé)

use rand::Rng;

use crate::mob_entities::{MobEntityManager, MobKind};
use crate::player_registry::PlayerRegistry;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;

const SPAWN_RADIUS_MIN: i32 = 24;
const SPAWN_RADIUS_MAX: i32 = 48;
const SPAWN_TRIES_PER_TICK: u32 = 3;

/// Cap maximum de mobs hostiles + passifs simultanés sur le serveur entier.
/// PMMP `MobCap` valeurs vanilla (par player) : 70 hostile, 10 passive.
/// Ici on globalise pour simplifier — proportionnel au nombre de joueurs.
const HOSTILE_CAP_PER_PLAYER: usize = 12;
const PASSIVE_CAP_PER_PLAYER: usize = 6;

fn entity_id_for(kind: MobKind) -> &'static str {
    kind.actor_type()
}

fn category_of(kind: MobKind) -> &'static str {
    match kind.category() {
        crate::mob_entities::MobCategory::Hostile => "monster",
        // Passifs + neutres comptent dans le cap « animal » pour le spawn.
        _ => "animal",
    }
}

/// Toutes les espèces candidates au spawn naturel = roster complet. Celles sans
/// règle de spawn vanilla (`spawn_weight` = None) sont automatiquement ignorées.
fn all_mobs() -> &'static [MobKind] {
    MobKind::ALL
}

/// Tick global — appelé chaque game tick (20 TPS) depuis main.rs.
/// Spawn 0..N mobs selon les règles vanilla + caps.
pub fn tick<R: Rng>(
    rng: &mut R,
    registry: &PlayerRegistry,
    cache: &mut ChunkCache,
    mobs: &mut MobEntityManager,
    is_night: bool,
) {
    if registry.players.is_empty() {
        return;
    }

    // Caps.
    let player_count = registry.players.len();
    let hostile_cap = HOSTILE_CAP_PER_PLAYER * player_count;
    let passive_cap = PASSIVE_CAP_PER_PLAYER * player_count;
    let mut hostile_count = 0;
    let mut passive_count = 0;
    for m in mobs.all() {
        match category_of(m.kind) {
            "monster" => hostile_count += 1,
            "animal" => passive_count += 1,
            _ => {}
        }
    }

    let player_positions: Vec<[f32; 3]> = registry.players.values().map(|p| p.position).collect();

    for _ in 0..SPAWN_TRIES_PER_TICK {
        // Pick joueur aléatoire.
        let pi = rng.gen_range(0..player_positions.len());
        let center = player_positions[pi];

        // Pick mob aléatoire pondéré par `spawn_weight` vanilla.
        let weights: Vec<(MobKind, u32)> = all_mobs()
            .iter()
            .filter_map(|&k| {
                crate::spawn_rules_vanilla::spawn_weight(entity_id_for(k)).map(|w| (k, w.max(1)))
            })
            .collect();
        if weights.is_empty() {
            return;
        }
        let total: u32 = weights.iter().map(|(_, w)| *w).sum();
        let mut roll = rng.gen_range(0..total);
        let mut picked = weights[0].0;
        for (k, w) in &weights {
            if roll < *w {
                picked = *k;
                break;
            }
            roll -= *w;
        }

        // Cap check par catégorie.
        let cat = category_of(picked);
        match cat {
            "monster" if hostile_count >= hostile_cap => continue,
            "animal" if passive_count >= passive_cap => continue,
            _ => {}
        }

        // Monster ne spawn que dans la nuit (simplification — vanilla check
        // light level réel chaque pos ; on veut éviter les mobs hostiles en
        // plein jour à la surface).
        if cat == "monster" && !is_night {
            continue;
        }

        // Pick position aléatoire dans anneau autour du joueur.
        let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
        let radius = rng.gen_range(SPAWN_RADIUS_MIN..SPAWN_RADIUS_MAX) as f32;
        let dx = (angle.cos() * radius) as i32;
        let dz = (angle.sin() * radius) as i32;
        let sx = center[0] as i32 + dx;
        let sz = center[2] as i32 + dz;

        // Trouve un sol valide (top non-air entre Y=320 et Y=-64).
        let mut sy = None;
        for y in (-64..=320).rev() {
            let id = cache.get_block(sx, y, sz);
            if id == BLOCKS.air || id == BLOCKS.water {
                continue;
            }
            let name = BLOCKS.name_for(id).unwrap_or("");
            if !crate::block_attachment::is_solid_support(name) {
                continue;
            }
            // Headroom : besoin d'au moins 2 blocs d'air au-dessus.
            let above1 = cache.get_block(sx, y + 1, sz);
            let above2 = cache.get_block(sx, y + 2, sz);
            if above1 == BLOCKS.air && above2 == BLOCKS.air {
                sy = Some(y + 1);
                break;
            }
        }
        let Some(sy) = sy else { continue };

        // Spawn. ~5 % des animaux apparaissent en bébé (convention Bedrock).
        let spawn_pos = [sx as f32 + 0.5, sy as f32, sz as f32 + 0.5];
        let _entity = if cat == "animal" && rng.gen_range(0..100) < 5 {
            mobs.spawn_baby(picked, spawn_pos)
        } else {
            mobs.spawn(picked, spawn_pos)
        };
        match cat {
            "monster" => hostile_count += 1,
            "animal" => passive_count += 1,
            _ => {}
        }

        tracing::debug!(
            "[spawner] Spawned {:?} at ({},{},{}) (cat={}, hostile={}/{}, passive={}/{})",
            picked,
            sx,
            sy,
            sz,
            cat,
            hostile_count,
            hostile_cap,
            passive_count,
            passive_cap
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_can_be_collected_for_supported_mobs() {
        let weights: Vec<_> = all_mobs()
            .iter()
            .filter_map(|&k| {
                crate::spawn_rules_vanilla::spawn_weight(entity_id_for(k)).map(|w| (k, w))
            })
            .collect();
        assert!(
            !weights.is_empty(),
            "should have weight for at least one mob"
        );
    }
}
