//! Sensors — captent l'environnement et écrivent dans la [`Memory`].
//! Port de `server/.../ai/sensor/` d'Allay.

use super::memory::Memory;
use super::{PlayerSnapshot, Sensor};
use crate::entity::EntityBase;
use crate::mob_entities::is_supporting_block;
use crate::world::chunk_cache::ChunkCache;

/// Ligne de vue dégagée entre deux points (raycast voxel pas à pas) ?
/// Aucun bloc solide ne doit obstruer le segment.
fn line_of_sight(cache: &mut ChunkCache, from: [f32; 3], to: [f32; 3]) -> bool {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if dist < 0.001 {
        return true;
    }
    let steps = (dist / 0.5).ceil() as i32; // un échantillon tous les 0.5 bloc
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let x = (from[0] + d[0] * t).floor() as i32;
        let y = (from[1] + d[1] * t).floor() as i32;
        let z = (from[2] + d[2] * t).floor() as i32;
        if is_supporting_block(cache.get_block(x, y, z)) {
            return false;
        }
    }
    true
}

/// Détecte le joueur **attaquable** le plus proche dans `[min_range, range]` et
/// stocke son runtime id dans `memory.nearest_player`. Port de
/// `NearestPlayerSensor`.
///
/// On filtre dès le sensor sur `is_attackable` (vivant + survival/adventure)
/// pour que les mobs ignorent les joueurs creative/spectator — c.f.
/// `MeleeAttackExecutor.isTargetValid`.
pub struct NearestPlayerSensor {
    range_sq: f64,
    min_range_sq: f64,
    period: u32,
}

impl NearestPlayerSensor {
    pub fn new(range: f64, min_range: f64, period: u32) -> Self {
        Self {
            range_sq: range * range,
            min_range_sq: min_range * min_range,
            period: period.max(1),
        }
    }

    /// Raccourci : portée `range`, sans distance minimale, échantillonné chaque tick.
    pub fn with_range(range: f64) -> Self {
        Self::new(range, 0.0, 1)
    }
}

impl Sensor for NearestPlayerSensor {
    fn sense(
        &mut self,
        memory: &mut Memory,
        base: &EntityBase,
        players: &[PlayerSnapshot],
        chunk_cache: &mut ChunkCache,
    ) {
        let pos = base.position;
        // Yeux du mob (≈ mi-hauteur) pour le raycast de ligne de vue.
        let eye = [pos[0], pos[1] + 1.4, pos[2]];
        let mut nearest: Option<u64> = None;
        let mut nearest_dist_sq = f64::MAX;

        for player in players {
            if !player.is_attackable() {
                continue;
            }
            let dx = (player.position[0] - pos[0]) as f64;
            let dy = (player.position[1] - pos[1]) as f64;
            let dz = (player.position[2] - pos[2]) as f64;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq < self.min_range_sq || dist_sq > self.range_sq {
                continue;
            }
            if dist_sq >= nearest_dist_sq {
                continue;
            }
            // Ne cible que si la ligne de vue vers les yeux du joueur est dégagée.
            let target_eye = [player.position[0], player.position[1] + 1.62, player.position[2]];
            if !line_of_sight(chunk_cache, eye, target_eye) {
                continue;
            }
            nearest_dist_sq = dist_sq;
            nearest = Some(player.runtime_id);
        }

        memory.nearest_player = nearest;
    }

    fn period(&self) -> u32 {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::block_registry::BLOCKS;

    fn base_at(pos: [f32; 3]) -> EntityBase {
        EntityBase::new("minecraft:zombie", "zombie", "Zombie", pos, vec![], vec![])
    }

    fn player(runtime_id: u64, pos: [f32; 3], gamemode: i32) -> PlayerSnapshot {
        PlayerSnapshot {
            runtime_id,
            position: pos,
            gamemode,
            alive: true,
            held_item: 0,
            look_dir: [0.0, 0.0, 1.0],        }
    }

    /// Cache "flat" : air au-dessus de la surface basse → ligne de vue dégagée.
    fn temp_cache(tag: &str) -> (ChunkCache, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mc-rs-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (ChunkCache::new(&dir, 1, "flat"), dir)
    }

    #[test]
    fn picks_nearest_attackable_player_in_range() {
        let mut sensor = NearestPlayerSensor::with_range(16.0);
        let mut memory = Memory::default();
        let base = base_at([0.0, 64.0, 0.0]);
        let players = vec![
            player(1, [10.0, 64.0, 0.0], 0), // survival, distance 10
            player(2, [3.0, 64.0, 0.0], 0),  // survival, distance 3 → le plus proche
        ];
        let (mut cache, dir) = temp_cache("sensor-near");
        sensor.sense(&mut memory, &base, &players, &mut cache);
        assert_eq!(memory.nearest_player, Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignores_creative_and_out_of_range() {
        let mut sensor = NearestPlayerSensor::with_range(16.0);
        let mut memory = Memory::default();
        let base = base_at([0.0, 64.0, 0.0]);
        let players = vec![
            player(1, [2.0, 64.0, 0.0], 1),   // creative, proche → ignoré
            player(2, [100.0, 64.0, 0.0], 0), // hors portée → ignoré
        ];
        let (mut cache, dir) = temp_cache("sensor-ignore");
        sensor.sense(&mut memory, &base, &players, &mut cache);
        assert_eq!(memory.nearest_player, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn blocked_line_of_sight_prevents_targeting() {
        let mut sensor = NearestPlayerSensor::with_range(16.0);
        let mut memory = Memory::default();
        let base = base_at([0.0, 64.0, 0.0]);
        let players = vec![player(1, [6.0, 64.0, 0.0], 0)];
        let (mut cache, dir) = temp_cache("sensor-los");
        // Mur de pierre entre le mob (x=0) et le joueur (x=6).
        for y in 64..=66 {
            cache.set_block(3, y, 0, BLOCKS.stone);
        }
        sensor.sense(&mut memory, &base, &players, &mut cache);
        assert_eq!(memory.nearest_player, None, "mur → pas de ligne de vue → pas de cible");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
