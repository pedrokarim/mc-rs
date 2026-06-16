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

use crate::mob_entities::{Habitat, MobEntityManager, MobKind};
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

/// Dimension numérique (0/1/2) d'un habitat de mob.
fn habitat_dimension(h: Habitat) -> i32 {
    match h {
        Habitat::Overworld => 0,
        Habitat::Nether => 1,
        Habitat::End => 2,
    }
}

/// Espèces (+ poids vanilla) éligibles au spawn naturel dans `dim`. Filtre par
/// habitat (un blaze ne spawne qu'au Nether, un enderman/zombie en overworld…) et
/// exclut les aquatiques du spawn terrestre.
fn candidate_mobs_for_dimension(dim: i32) -> Vec<(MobKind, u32)> {
    all_mobs()
        .iter()
        .filter_map(|&k| {
            if habitat_dimension(k.habitat()) != dim {
                return None;
            }
            if k.is_aquatic() {
                return None;
            }
            crate::spawn_rules_vanilla::spawn_weight(entity_id_for(k)).map(|w| (k, w.max(1)))
        })
        .collect()
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
    // On ne spawn QUE autour des joueurs de la dimension de ce cache, et on
    // compte les caps par dimension (chaque dimension a son propre budget de mobs).
    let dim = cache.dimension();
    let player_positions: Vec<[f32; 3]> = registry
        .players
        .values()
        .filter(|p| p.dimension == dim)
        .map(|p| p.position)
        .collect();
    if player_positions.is_empty() {
        return;
    }

    // Caps (par dimension).
    let player_count = player_positions.len();
    let hostile_cap = HOSTILE_CAP_PER_PLAYER * player_count;
    let passive_cap = PASSIVE_CAP_PER_PLAYER * player_count;
    let mut hostile_count = 0;
    let mut passive_count = 0;
    for m in mobs.all() {
        if m.base.dimension != dim {
            continue;
        }
        match category_of(m.kind) {
            "monster" => hostile_count += 1,
            "animal" => passive_count += 1,
            _ => {}
        }
    }

    for _ in 0..SPAWN_TRIES_PER_TICK {
        // Pick joueur aléatoire.
        let pi = rng.gen_range(0..player_positions.len());
        let center = player_positions[pi];

        // Pick mob aléatoire pondéré par `spawn_weight` vanilla, restreint aux
        // espèces de la dimension actuellement tickée (overworld/nether/end).
        let weights = candidate_mobs_for_dimension(cache.dimension());
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

        // Monster ne spawn que dans la nuit — uniquement en overworld. Le Nether
        // et l'End n'ont pas de cycle jour/nuit : les hostiles y spawnent toujours.
        if cat == "monster" && cache.dimension() == 0 && !is_night {
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

        // Filtrage par biome : le mob doit être autorisé par le `biome_filter`
        // vanilla du biome RÉELLEMENT généré au point de spawn (ex. husk →
        // "desert", stray → "frozen", parrot → "jungle", cow → "animal"). On
        // lit le biome via le générateur ACTIF (`biome_identifier_at`) pour
        // qu'il corresponde à ce que le joueur voit — comme Bedrock, qui teste
        // le biome du bloc de spawn.
        let biome_id = cache.biome_identifier_at(sx, sy, sz);
        let biome_tags: &[String] = crate::biomes_vanilla::for_biome(biome_id)
            .map(|d| d.tags.as_slice())
            .unwrap_or(&[]);
        if !crate::spawn_rules_vanilla::biome_allows(entity_id_for(picked), biome_tags) {
            continue;
        }

        // Spawn. Slimes : taille aléatoire (1/2/4). Animaux : ~5 % en bébé.
        let spawn_pos = [sx as f32 + 0.5, sy as f32, sz as f32 + 0.5];
        let _entity = if picked.is_slime() {
            let size = [1u8, 2, 4][rng.gen_range(0..3)];
            mobs.spawn_slime(picked, spawn_pos, size)
        } else if cat == "animal" && rng.gen_range(0..100) < 5 {
            mobs.spawn_baby(picked, spawn_pos)
        } else {
            mobs.spawn(picked, spawn_pos)
        };
        // L'entité appartient à la dimension actuellement tickée.
        mobs.set_dimension(_entity.base.entity_runtime_id, cache.dimension());
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

/// Délai moyen (game ticks) entre deux salves d'un spawner actif. Vanilla : aléatoire
/// 200..800 ; on approxime par une proba par tick (pas de registre de délai par
/// spawner). À 20 TPS → ~1 salve toutes les ~20 s par spawner avec un joueur proche.
const SPAWNER_AVG_DELAY: i32 = 400;
/// Portée joueur requise (RequiredPlayerRange vanilla) + cap d'entités proches.
const SPAWNER_PLAYER_RANGE: f32 = 16.0;
const SPAWNER_MAX_NEARBY: usize = 6;
const SPAWNER_SPAWN_RANGE: i32 = 4;

/// Tick des **spawners (block entities)** d'une dimension : pour chaque spawner d'un
/// chunk chargé, si un joueur est à portée et que le cap d'entités proches n'est pas
/// atteint, fait apparaître par salves les mobs de l'EntityIdentifier (forteresse =
/// blaze). Réf comportement : `MonsterSpawner` (Bedrock).
pub fn tick_spawners<R: Rng>(
    rng: &mut R,
    registry: &PlayerRegistry,
    cache: &mut ChunkCache,
    mobs: &mut MobEntityManager,
) {
    let dim = cache.dimension();
    let players: Vec<[f32; 3]> = registry
        .players
        .values()
        .filter(|p| p.dimension == dim)
        .map(|p| p.position)
        .collect();
    if players.is_empty() {
        return;
    }
    let spawners = cache.mob_spawner_positions();
    for (pos, entity) in spawners {
        let center = [pos[0] as f32 + 0.5, pos[1] as f32, pos[2] as f32 + 0.5];
        // Joueur à portée ?
        let near_player = players.iter().any(|p| {
            let (dx, dy, dz) = (p[0] - center[0], p[1] - center[1], p[2] - center[2]);
            dx * dx + dy * dy + dz * dz <= SPAWNER_PLAYER_RANGE * SPAWNER_PLAYER_RANGE
        });
        if !near_player {
            continue;
        }
        // Cap d'entités proches du spawner (MaxNearbyEntities).
        let nearby = mobs
            .all()
            .filter(|m| m.base.dimension == dim)
            .filter(|m| {
                let (dx, dz) = (
                    m.base.position[0] - center[0],
                    m.base.position[2] - center[2],
                );
                dx * dx + dz * dz <= (SPAWNER_PLAYER_RANGE * SPAWNER_PLAYER_RANGE)
            })
            .count();
        if nearby >= SPAWNER_MAX_NEARBY {
            continue;
        }
        // Délai approximé par proba/tick.
        if rng.gen_range(0..SPAWNER_AVG_DELAY) != 0 {
            continue;
        }
        // Forteresse = blaze ; sinon on ignore (autres entités = suivi).
        let kind = if entity == "minecraft:blaze" {
            MobKind::Blaze
        } else {
            continue;
        };
        // Salve : SpawnCount (1..4) mobs dans SpawnRange autour du spawner.
        let count = 1 + rng.gen_range(0..4);
        for _ in 0..count {
            let sx = center[0] + rng.gen_range(-SPAWNER_SPAWN_RANGE..=SPAWNER_SPAWN_RANGE) as f32;
            let sz = center[2] + rng.gen_range(-SPAWNER_SPAWN_RANGE..=SPAWNER_SPAWN_RANGE) as f32;
            let spawned = mobs.spawn(kind, [sx, center[1], sz]);
            mobs.set_dimension(spawned.base.entity_runtime_id, dim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_can_be_collected_for_supported_mobs() {
        let weights = candidate_mobs_for_dimension(0);
        assert!(
            !weights.is_empty(),
            "should have weight for at least one overworld mob"
        );
    }

    #[test]
    fn overworld_candidates_are_all_overworld() {
        for (k, _) in candidate_mobs_for_dimension(0) {
            assert_eq!(
                k.habitat(),
                Habitat::Overworld,
                "{k:?} ne devrait pas spawner naturellement en overworld"
            );
        }
    }

    #[test]
    fn nether_candidates_are_all_nether() {
        let nether = candidate_mobs_for_dimension(1);
        for (k, _) in &nether {
            assert_eq!(habitat_dimension(k.habitat()), 1, "{k:?} pas Nether");
        }
        // Les mobs Nether connus avec une règle de spawn doivent être candidats.
        let kinds: Vec<MobKind> = nether.iter().map(|(k, _)| *k).collect();
        assert_eq!(MobKind::Blaze.habitat(), Habitat::Nether);
        assert!(
            kinds.contains(&MobKind::Blaze) || kinds.contains(&MobKind::Ghast),
            "au moins un mob Nether emblématique devrait être candidat"
        );
    }

    #[test]
    fn no_overworld_mob_leaks_into_nether_or_end() {
        for dim in [1, 2] {
            for (k, _) in candidate_mobs_for_dimension(dim) {
                assert_ne!(
                    k.habitat(),
                    Habitat::Overworld,
                    "{k:?} (overworld) ne doit pas spawner en dim {dim}"
                );
            }
        }
        // Et symétriquement, aucun mob Nether/End en overworld.
        for (k, _) in candidate_mobs_for_dimension(0) {
            assert_eq!(k.habitat(), Habitat::Overworld);
        }
    }
}
