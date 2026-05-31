//! Entité flèche vivante (projectile tiré par les squelettes) + son manager.
//!
//! Mirroir léger de [`crate::mob_entities`] : gravité + déplacement, collision
//! avec les blocs solides (la flèche s'arrête → despawn), et détection de
//! collision **segment-vs-joueur** (anti-tunneling : on teste la distance du
//! joueur au segment de déplacement du tick, pas seulement la position finale).
//!
//! NB : pas de référence PMMP/Allay directe portée ici — l'aim et la physique
//! sont une approximation balistique simple, à affiner en jeu.

use std::collections::HashMap;

use crate::entity::{living_metadata, EntityBase};
use crate::mob_entities::{is_supporting_block, MovementUpdate};
use crate::world::chunk_cache::ChunkCache;

/// Gravité d'une flèche par tick (≈ vanilla 0.05 à 20 TPS).
const ARROW_GRAVITY: f32 = 0.05;
/// Traînée par tick.
const ARROW_DRAG: f32 = 0.99;
/// Durée de vie max (ticks 20 TPS → ~30 s) avant despawn.
const ARROW_MAX_LIFE: u32 = 600;
/// Rayon de collision flèche↔joueur (blocs).
const ARROW_HIT_RADIUS: f32 = 0.8;

#[derive(Clone)]
pub struct ArrowEntity {
    pub base: EntityBase,
    pub damage: f32,
    /// Runtime id du tireur (pour le knockback / l'attribution des dégâts).
    pub shooter_runtime_id: u64,
    life: u32,
    /// Le projectile est planté (a touché un bloc) → plus de mouvement.
    grounded: bool,
    /// Gravité par tick (0 = trajectoire rectiligne, ex boule de feu).
    gravity: f32,
    /// Se plante dans les blocs (flèche) ou disparaît à l'impact (boule de feu).
    sticky: bool,
    /// Joueur poursuivi (tête chercheuse, ex bullet du shulker).
    homing_target: Option<u64>,
}

impl ArrowEntity {
    /// Flèche : trajectoire balistique, se plante dans les blocs.
    pub fn spawn(position: [f32; 3], velocity: [f32; 3], damage: f32, shooter_runtime_id: u64) -> Self {
        Self::projectile(
            "minecraft:arrow",
            position,
            velocity,
            damage,
            shooter_runtime_id,
            ARROW_GRAVITY,
            true,
        )
    }

    /// Projectile générique (flèche, boule de feu, …).
    pub fn projectile(
        actor_type: &'static str,
        position: [f32; 3],
        velocity: [f32; 3],
        damage: f32,
        shooter_runtime_id: u64,
        gravity: f32,
        sticky: bool,
    ) -> Self {
        let mut base = EntityBase::new(
            actor_type,
            "",
            "",
            position,
            vec![],
            living_metadata(0.25, 0.25, None),
        );
        base.velocity = velocity;
        Self {
            base,
            damage,
            shooter_runtime_id,
            life: 0,
            grounded: false,
            gravity,
            sticky,
            homing_target: None,
        }
    }

    pub fn add_actor_packet(&self) -> Vec<u8> {
        self.base.add_actor_packet()
    }

    pub fn remove_packet(&self) -> Vec<u8> {
        self.base.remove_packet()
    }
}

/// Une flèche a touché un joueur.
pub struct ArrowHit {
    pub target_runtime_id: u64,
    pub damage: f32,
    /// Position de la flèche au moment de l'impact (origine du knockback).
    pub from_position: [f32; 3],
    pub shooter_runtime_id: u64,
}

#[derive(Default)]
pub struct ArrowTickResult {
    pub movement_updates: Vec<MovementUpdate>,
    pub despawned: Vec<ArrowEntity>,
    pub hits: Vec<ArrowHit>,
}

pub struct ArrowEntityManager {
    arrows: HashMap<u64, ArrowEntity>,
}

impl Default for ArrowEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrowEntityManager {
    pub fn new() -> Self {
        Self {
            arrows: HashMap::new(),
        }
    }

    pub fn spawn(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
        damage: f32,
        shooter_runtime_id: u64,
    ) -> ArrowEntity {
        let arrow = ArrowEntity::spawn(position, velocity, damage, shooter_runtime_id);
        self.arrows
            .insert(arrow.base.entity_runtime_id, arrow.clone());
        arrow
    }

    /// Spawn d'une boule de feu (trajectoire rectiligne, disparaît à l'impact).
    /// `homing_target` = `Some(player)` pour un projectile à tête chercheuse.
    pub fn spawn_fireball(
        &mut self,
        actor_type: &'static str,
        position: [f32; 3],
        velocity: [f32; 3],
        damage: f32,
        shooter_runtime_id: u64,
        homing_target: Option<u64>,
    ) -> ArrowEntity {
        let mut fb = ArrowEntity::projectile(
            actor_type,
            position,
            velocity,
            damage,
            shooter_runtime_id,
            0.0,
            false,
        );
        fb.homing_target = homing_target;
        self.arrows.insert(fb.base.entity_runtime_id, fb.clone());
        fb
    }

    pub fn all(&self) -> impl Iterator<Item = &ArrowEntity> {
        self.arrows.values()
    }

    /// Avance toutes les flèches : physique, collision bloc, hit joueur.
    /// `players` : (runtime_id, position pieds) des joueurs en jeu.
    pub fn tick(&mut self, chunk_cache: &mut ChunkCache, players: &[(u64, [f32; 3])]) -> ArrowTickResult {
        let mut result = ArrowTickResult::default();
        let ids: Vec<u64> = self.arrows.keys().copied().collect();

        for id in ids {
            let Some(arrow) = self.arrows.get_mut(&id) else {
                continue;
            };

            arrow.life += 1;
            if arrow.life >= ARROW_MAX_LIFE {
                if let Some(a) = self.arrows.remove(&id) {
                    result.despawned.push(a);
                }
                continue;
            }
            if arrow.grounded {
                continue; // flèche plantée : immobile, attend le despawn
            }

            let start = arrow.base.position;

            // Physique : gravité par-projectile (0 pour une boule de feu) + traînée.
            arrow.base.velocity[1] -= arrow.gravity;
            for c in arrow.base.velocity.iter_mut() {
                *c *= ARROW_DRAG;
            }
            // Tête chercheuse : on infléchit la vélocité vers la cible (à vitesse
            // constante) — le projectile poursuit le joueur (bullet du shulker).
            if let Some(tid) = arrow.homing_target {
                if let Some((_, tpos)) = players.iter().find(|(pid, _)| *pid == tid) {
                    let target = [tpos[0], tpos[1] + 1.0, tpos[2]];
                    let to = [
                        target[0] - arrow.base.position[0],
                        target[1] - arrow.base.position[1],
                        target[2] - arrow.base.position[2],
                    ];
                    let to_len = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
                    let v = arrow.base.velocity;
                    let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                    if to_len > 0.1 && speed > 0.001 {
                        const STEER: f32 = 0.25; // agilité du virage
                        for i in 0..3 {
                            let desired = to[i] / to_len * speed;
                            arrow.base.velocity[i] = v[i] * (1.0 - STEER) + desired * STEER;
                        }
                    }
                }
            }
            let end = [
                start[0] + arrow.base.velocity[0],
                start[1] + arrow.base.velocity[1],
                start[2] + arrow.base.velocity[2],
            ];

            // Collision joueur : distance du joueur au segment start→end.
            let mut hit_player: Option<(u64, [f32; 3])> = None;
            for (pid, ppos) in players {
                if *pid == arrow.shooter_runtime_id && arrow.life < 5 {
                    continue; // ne pas toucher le tireur juste après le tir
                }
                // Centre de masse du joueur (~mi-hauteur).
                let target = [ppos[0], ppos[1] + 1.0, ppos[2]];
                if point_segment_distance(target, start, end) < ARROW_HIT_RADIUS {
                    hit_player = Some((*pid, end));
                    break;
                }
            }
            if let Some((pid, pos)) = hit_player {
                result.hits.push(ArrowHit {
                    target_runtime_id: pid,
                    damage: arrow.damage,
                    from_position: pos,
                    shooter_runtime_id: arrow.shooter_runtime_id,
                });
                if let Some(a) = self.arrows.remove(&id) {
                    result.despawned.push(a);
                }
                continue;
            }

            // Collision bloc : la flèche se plante, la boule de feu disparaît.
            let bx = end[0].floor() as i32;
            let by = end[1].floor() as i32;
            let bz = end[2].floor() as i32;
            if is_supporting_block(chunk_cache.get_block(bx, by, bz)) {
                if arrow.sticky {
                    arrow.base.velocity = [0.0, 0.0, 0.0];
                    arrow.grounded = true; // plantée jusqu'au despawn par vie
                    continue;
                }
                // Non collante (boule de feu) : disparaît à l'impact.
                if let Some(a) = self.arrows.remove(&id) {
                    result.despawned.push(a);
                }
                continue;
            }

            arrow.base.position = end;
            result.movement_updates.push(MovementUpdate {
                entity_unique_id: arrow.base.entity_unique_id,
                entity_position: arrow.base.position,
                add_packet: arrow.base.add_actor_packet(),
                move_packet: arrow.base.move_absolute_packet(false, false),
                motion_packet: arrow.base.motion_packet(),
            });
        }

        result
    }
}

/// Distance d'un point `p` au segment `[a, b]` (3D).
fn point_segment_distance(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let ab_len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if ab_len2 > 1e-9 {
        ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab_len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    let d = [p[0] - closest[0], p[1] - closest[1], p[2] - closest[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache(tag: &str) -> (ChunkCache, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mc-rs-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (ChunkCache::new(&dir, 1, "flat"), dir)
    }

    #[test]
    fn point_segment_distance_basics() {
        // Point au-dessus du milieu d'un segment horizontal.
        let d = point_segment_distance([5.0, 1.0, 0.0], [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        assert!((d - 1.0).abs() < 1e-5);
        // Point au-delà de l'extrémité → distance à l'extrémité.
        let d2 = point_segment_distance([12.0, 0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        assert!((d2 - 2.0).abs() < 1e-5);
    }

    #[test]
    fn arrow_hits_player_in_path() {
        let (mut cache, dir) = temp_cache("arrow-hit");
        let mut mgr = ArrowEntityManager::new();
        // Flèche en (0,65,0) filant en +X à 1.5 b/tick ; joueur en x≈3.
        mgr.spawn([0.0, 65.0, 0.0], [1.5, 0.0, 0.0], 4.0, 999);
        let players = vec![(7u64, [3.0, 64.0, 0.0])]; // centre de masse ~ y65
        let mut hit = false;
        for _ in 0..6 {
            let r = mgr.tick(&mut cache, &players);
            if !r.hits.is_empty() {
                assert_eq!(r.hits[0].target_runtime_id, 7);
                assert_eq!(r.hits[0].damage, 4.0);
                hit = true;
                break;
            }
        }
        assert!(hit, "la flèche doit toucher le joueur sur sa trajectoire");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn homing_bullet_curves_to_hit_offset_player() {
        let (mut cache, dir) = temp_cache("homing-bullet");
        let mut mgr = ArrowEntityManager::new();
        // Bullet filant en +X mais joueur décalé en +Z : sans tête chercheuse,
        // il manquerait. Le homing doit l'infléchir jusqu'à toucher.
        let fb = mgr.spawn_fireball("minecraft:shulker_bullet", [0.0, 65.0, 0.0], [1.2, 0.0, 0.0], 4.0, 999, Some(7));
        assert!(fb.homing_target == Some(7));
        let players = vec![(7u64, [5.0, 64.0, 5.0])];
        let mut hit = false;
        for _ in 0..40 {
            let r = mgr.tick(&mut cache, &players);
            if r.hits.iter().any(|h| h.target_runtime_id == 7) {
                hit = true;
                break;
            }
        }
        assert!(hit, "le projectile à tête chercheuse doit toucher le joueur décalé");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn arrow_does_not_hit_its_shooter_immediately() {
        let (mut cache, dir) = temp_cache("arrow-shooter");
        let mut mgr = ArrowEntityManager::new();
        // Tireur (id 7) à l'origine ; flèche part de l'origine.
        mgr.spawn([0.0, 65.0, 0.0], [0.1, 0.0, 0.0], 4.0, 7);
        let players = vec![(7u64, [0.0, 64.0, 0.0])];
        let r = mgr.tick(&mut cache, &players);
        assert!(r.hits.is_empty(), "ne touche pas le tireur juste après le tir");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
