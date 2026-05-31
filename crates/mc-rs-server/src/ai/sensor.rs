//! Sensors — captent l'environnement et écrivent dans la [`Memory`].
//! Port de `server/.../ai/sensor/` d'Allay.

use super::memory::Memory;
use super::{PlayerSnapshot, Sensor};
use crate::entity::EntityBase;

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
    fn sense(&mut self, memory: &mut Memory, base: &EntityBase, players: &[PlayerSnapshot]) {
        let pos = base.position;
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
            if dist_sq < nearest_dist_sq {
                nearest_dist_sq = dist_sq;
                nearest = Some(player.runtime_id);
            }
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
        }
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
        sensor.sense(&mut memory, &base, &players);
        assert_eq!(memory.nearest_player, Some(2));
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
        sensor.sense(&mut memory, &base, &players);
        assert_eq!(memory.nearest_player, None);
    }
}
