//! Mémoire IA d'un mob — port de `MemoryStorage` d'Allay
//! (`api/.../entity/ai/memory/`), mais **typée par champs** plutôt que par une
//! map hétérogène `MemoryType<T>` (plus simple et sûr en Rust).
//!
//! Contient les sorties des sensors (joueur le plus proche), les cibles de
//! déplacement/regard posées par les executors, et l'**état de route** géré
//! localement (le RouteFinder est sans état, comme `BehaviorGroupImpl`).

/// État IA persistant d'un mob entre deux ticks.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// Runtime id du joueur le plus proche (sortie de `NearestPlayerSensor`).
    /// Équivalent `MemoryTypes.NEAREST_PLAYER`.
    pub nearest_player: Option<u64>,
    /// Cible de riposte : joueur ayant frappé le mob (`minecraft:behavior.hurt_by_target`).
    /// Prioritaire sur `nearest_player` et **ne requiert pas de ligne de vue** —
    /// le mob traque son agresseur même frappé en douce / de dos. Décrémenté par
    /// `aggro_ticks` (colère qui retombe).
    pub aggro_target: Option<u64>,
    /// Ticks de colère restants ; à 0 → `aggro_target` s'efface.
    pub aggro_ticks: u32,
    /// Cible de déplacement absolue (centre de bloc monde) — alimente le
    /// RouteFinder. Équivalent `MemoryTypes.MOVE_TARGET`.
    pub move_target: Option<[f64; 3]>,
    /// Cible de regard (yaw/pitch) — alimente le `LookController`.
    pub look_target: Option<[f64; 3]>,
    /// Vitesse de déplacement courante (blocs/tick), posée par l'executor actif
    /// dans `on_start`. Équivalent `MemoryTypes.MOVEMENT_SPEED`.
    pub movement_speed: f32,

    // --- État de route (port de l'état local de `BehaviorGroupImpl`) ---
    /// Waypoints courants (centres de blocs monde) renvoyés par le RouteFinder.
    pub route: Vec<[f64; 3]>,
    /// Index du prochain waypoint à consommer dans `route`.
    pub node_index: usize,
    /// Compteur de ticks avant recalcul périodique forcé de la route.
    pub route_update_tick: u32,
    /// Demande explicite de recalcul de route au prochain tick.
    pub route_update_required: bool,
    /// Segment de direction courant `(start, end)` suivi par le `WalkController`.
    pub move_dir_start: Option<[f64; 3]>,
    pub move_dir_end: Option<[f64; 3]>,
    /// Indique qu'il faut avancer au waypoint suivant (entité assez proche).
    pub should_update_move_dir: bool,
}

impl Memory {
    /// Mémoire neuve avec une vitesse de déplacement par défaut.
    pub fn new(default_speed: f32) -> Self {
        Self {
            movement_speed: default_speed,
            ..Default::default()
        }
    }

    /// Cible de combat effective : l'agresseur (riposte) prime sur le joueur
    /// visible le plus proche.
    pub fn target(&self) -> Option<u64> {
        self.aggro_target.or(self.nearest_player)
    }

    /// Pose une cible de riposte pour `ticks` ticks (colère).
    pub fn set_aggro(&mut self, attacker: u64, ticks: u32) {
        self.aggro_target = Some(attacker);
        self.aggro_ticks = ticks;
    }

    /// Décrémente la colère ; efface la cible de riposte à expiration.
    pub fn tick_aggro(&mut self) {
        if self.aggro_ticks > 0 {
            self.aggro_ticks -= 1;
            if self.aggro_ticks == 0 {
                self.aggro_target = None;
            }
        }
    }

    /// Le mob a-t-il un segment de direction actif ?
    pub fn has_move_direction(&self) -> bool {
        self.move_dir_start.is_some() && self.move_dir_end.is_some()
    }

    /// Réinitialise le segment de direction (mob immobile).
    pub fn clear_move_direction(&mut self) {
        self.move_dir_start = None;
        self.move_dir_end = None;
    }

    /// Reste-t-il des waypoints à consommer dans la route courante ?
    pub fn has_next_node(&self) -> bool {
        self.node_index < self.route.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggro_prioritizes_attacker_and_decays() {
        let mut m = Memory::new(0.1);
        m.nearest_player = Some(1);
        // Sans colère : cible = joueur visible.
        assert_eq!(m.target(), Some(1));
        // Frappé par le joueur 2 : la riposte prime, même sur nearest_player.
        m.set_aggro(2, 3);
        assert_eq!(m.target(), Some(2));
        // La colère retombe après N ticks → retour au joueur visible.
        m.tick_aggro();
        m.tick_aggro();
        assert_eq!(m.target(), Some(2));
        m.tick_aggro(); // 3e tick → expire
        assert_eq!(m.aggro_target, None);
        assert_eq!(m.target(), Some(1));
    }
}
