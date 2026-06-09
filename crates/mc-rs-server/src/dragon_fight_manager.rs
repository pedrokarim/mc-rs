//! Contrôleur runtime du combat de l'ender dragon (DIM-J).
//!
//! Consolide les modules dormants [`crate::dragon_fight`] (state machine du combat),
//! [`crate::ender_dragon`] (HP/phases/heal du dragon) et [`crate::end_crystal`]
//! (crystals qui soignent + invulnérabilité) en un manager pilotable par la game loop.
//!
//! Le **vol** et les **phases** du dragon sont gérés par l'IA générique
//! (`AiProfile::Dragon` → `DragonExecutor` + `FlyController`) : ici on ne gère que la
//! couche « combat de boss » (spawn, crystals, invulnérabilité, soin, mort, œuf).
//!
//! API à effets : `tick()` et les hooks renvoient des [`DragonFightEffect`] que la
//! game loop applique (spawn d'entités, barre de boss, drops), gardant le couplage
//! minimal avec `main.rs`.

use crate::dragon_fight::{DragonFight, DragonFightPhase};
use crate::ender_dragon::{EnderDragon, HP_MAX};
use crate::obsidian_pillar;

/// Centre de l'île de l'End / portail de sortie (X,Z) et altitude de spawn du dragon.
pub const FOUNTAIN_XZ: (i32, i32) = (0, 0);
pub const DRAGON_SPAWN: [f32; 3] = [0.0, 80.0, 0.0];

/// Effet à appliquer par la game loop suite à un tick / hook du combat.
#[derive(Debug, Clone, PartialEq)]
pub enum DragonFightEffect {
    /// Faire apparaître le dragon (MobEntity EnderDragon) à `position`.
    SpawnDragon { position: [f32; 3] },
    /// Faire apparaître un end crystal à `position` (entité ender_crystal).
    SpawnCrystal { position: [f32; 3] },
    /// Afficher la barre de boss aux joueurs de l'End.
    ShowBossBar,
    /// Mettre à jour la barre de boss (0.0..1.0).
    UpdateBossBar { health_percent: f32 },
    /// Cacher la barre de boss.
    HideBossBar,
    /// Le dragon est mort : générer le portail de sortie ; déposer l'œuf si premier kill.
    DragonDied {
        fountain_xz: (i32, i32),
        drop_egg: bool,
    },
}

pub struct DragonFightManager {
    fight: DragonFight,
    dragon: EnderDragon,
    /// runtime_id de l'entité dragon dans le MobEntityManager (si vivante).
    dragon_runtime_id: Option<u64>,
    /// runtime_ids des end crystals encore en vie.
    crystal_ids: Vec<u64>,
    /// True une fois le spawn initial (dragon + crystals) émis.
    spawned: bool,
}

impl Default for DragonFightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DragonFightManager {
    pub fn new() -> Self {
        Self {
            fight: DragonFight::new(),
            dragon: EnderDragon::new(),
            dragon_runtime_id: None,
            crystal_ids: Vec::new(),
            spawned: false,
        }
    }

    pub fn dragon_runtime_id(&self) -> Option<u64> {
        self.dragon_runtime_id
    }

    /// True si `id` est un end crystal vivant de ce combat.
    pub fn is_crystal(&self, id: u64) -> bool {
        self.crystal_ids.contains(&id)
    }

    /// True tant qu'au moins un crystal soigne le dragon (→ dragon invulnérable).
    pub fn dragon_invulnerable(&self) -> bool {
        !self.crystal_ids.is_empty()
    }

    pub fn dragon_alive(&self) -> bool {
        self.fight.phase == DragonFightPhase::DragonAlive && self.dragon_runtime_id.is_some()
    }

    /// Tick du combat. `end_has_players` : un joueur est présent à l'End. `seed` :
    /// graine du monde (pour placer les cristaux au sommet des bons piliers).
    /// Renvoie les effets à appliquer (spawn initial, mise à jour de barre, etc.).
    pub fn tick(&mut self, end_has_players: bool, seed: u64) -> Vec<DragonFightEffect> {
        let mut effects = Vec::new();
        self.fight.tick();

        // Spawn initial : à la première présence d'un joueur, si le dragon n'a jamais
        // été tué (vanilla : le dragon est présent à la première venue).
        if !self.spawned && end_has_players && !self.fight.previously_killed {
            self.spawned = true;
            self.fight.phase = DragonFightPhase::SpawningDragon;
            effects.push(DragonFightEffect::SpawnDragon {
                position: DRAGON_SPAWN,
            });
            // Un cristal au sommet (height+1) de chaque end spike — coords/hauteurs
            // vanilla (partagées avec le terrain via `obsidian_pillar::end_spikes`).
            for s in obsidian_pillar::end_spikes(seed) {
                effects.push(DragonFightEffect::SpawnCrystal {
                    position: [s.x as f32 + 0.5, s.crystal_y() as f32, s.z as f32 + 0.5],
                });
            }
            effects.push(DragonFightEffect::ShowBossBar);
        }

        // Soin par les crystals (le dragon ne se soigne que tant qu'il en reste).
        if self.dragon_alive() {
            self.dragon.connected_crystals = self.crystal_ids.clone();
            self.dragon.tick();
            effects.push(DragonFightEffect::UpdateBossBar {
                health_percent: (self.dragon.hp / HP_MAX).clamp(0.0, 1.0),
            });
        }

        effects
    }

    /// Lie le runtime_id de l'entité dragon une fois spawnée par la game loop.
    pub fn on_dragon_spawned(&mut self, runtime_id: u64) {
        self.dragon_runtime_id = Some(runtime_id);
        self.fight.spawn_dragon(runtime_id);
    }

    /// Lie le runtime_id d'un end crystal spawné.
    pub fn on_crystal_spawned(&mut self, runtime_id: u64) {
        self.crystal_ids.push(runtime_id);
    }

    /// Un end crystal est détruit (joueur ou explosion). Le déconnecte du dragon.
    /// Renvoie true si c'était bien un crystal de ce combat.
    pub fn on_crystal_destroyed(&mut self, id: u64) -> bool {
        if let Some(pos) = self.crystal_ids.iter().position(|&c| c == id) {
            self.crystal_ids.remove(pos);
            self.dragon.disconnect_crystal(id);
            true
        } else {
            false
        }
    }

    /// À appeler quand l'entité dragon a disparu du monde (tuée via le combat mob).
    /// Renvoie l'effet de mort : portail de sortie + œuf au tout premier kill.
    pub fn notify_dragon_killed(&mut self) -> Vec<DragonFightEffect> {
        if self.dragon_runtime_id.is_none() {
            return Vec::new();
        }
        let drop_egg = !self.fight.previously_killed;
        self.fight.on_dragon_death();
        self.dragon_runtime_id = None;
        vec![DragonFightEffect::DragonDied {
            fountain_xz: FOUNTAIN_XZ,
            drop_egg,
        }]
    }

    /// Tentative d'infliger `amount` de dégâts au dragon (à la tête ou au corps).
    /// Rejette les dégâts tant qu'un crystal le soigne (invulnérabilité vanilla).
    /// Renvoie les effets (mort éventuelle, mise à jour de barre).
    pub fn damage_dragon(&mut self, amount: f32, to_head: bool) -> Vec<DragonFightEffect> {
        let mut effects = Vec::new();
        if !self.dragon_alive() || self.dragon_invulnerable() {
            return effects;
        }
        self.dragon.take_damage(amount, to_head);
        effects.push(DragonFightEffect::UpdateBossBar {
            health_percent: (self.dragon.hp / HP_MAX).clamp(0.0, 1.0),
        });
        if self.dragon.is_dying() {
            let drop_egg = !self.fight.previously_killed;
            self.fight.on_dragon_death();
            self.dragon_runtime_id = None;
            effects.push(DragonFightEffect::HideBossBar);
            effects.push(DragonFightEffect::DragonDied {
                fountain_xz: FOUNTAIN_XZ,
                drop_egg,
            });
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawned_manager() -> DragonFightManager {
        let mut m = DragonFightManager::new();
        let effects = m.tick(true, 42);
        // Spawn dragon + 10 crystals + show boss bar.
        assert!(effects.contains(&DragonFightEffect::SpawnDragon {
            position: DRAGON_SPAWN
        }));
        assert_eq!(
            effects
                .iter()
                .filter(|e| matches!(e, DragonFightEffect::SpawnCrystal { .. }))
                .count(),
            10
        );
        // La game loop lie les ids.
        m.on_dragon_spawned(1000);
        for i in 0..10 {
            m.on_crystal_spawned(2000 + i);
        }
        m
    }

    #[test]
    fn spawns_dragon_and_crystals_on_first_end_visit() {
        let m = spawned_manager();
        assert!(m.dragon_alive());
        assert_eq!(m.dragon_runtime_id(), Some(1000));
        assert!(m.dragon_invulnerable());
    }

    #[test]
    fn dragon_invulnerable_while_crystals_alive() {
        let mut m = spawned_manager();
        let effects = m.damage_dragon(50.0, true);
        // Aucun dégât appliqué (invulnérable), donc pas d'effet.
        assert!(effects.is_empty());
        assert_eq!(m.dragon.hp, HP_MAX);
    }

    #[test]
    fn dragon_takes_damage_once_crystals_gone() {
        let mut m = spawned_manager();
        let ids: Vec<u64> = (0..10).map(|i| 2000 + i).collect();
        for id in ids {
            assert!(m.on_crystal_destroyed(id));
        }
        assert!(!m.dragon_invulnerable());
        let effects = m.damage_dragon(50.0, true);
        assert!(effects
            .iter()
            .any(|e| matches!(e, DragonFightEffect::UpdateBossBar { .. })));
        assert!(m.dragon.hp < HP_MAX);
    }

    #[test]
    fn first_kill_drops_egg_and_generates_portal() {
        let mut m = spawned_manager();
        for i in 0..10 {
            m.on_crystal_destroyed(2000 + i);
        }
        let effects = m.damage_dragon(1000.0, true);
        assert!(effects.contains(&DragonFightEffect::HideBossBar));
        let died = effects
            .iter()
            .find_map(|e| match e {
                DragonFightEffect::DragonDied { drop_egg, .. } => Some(*drop_egg),
                _ => None,
            })
            .expect("dragon should die");
        assert!(died, "first kill must drop the egg");
        assert!(!m.dragon_alive());
    }

    #[test]
    fn healing_recovers_hp_while_crystals_alive() {
        let mut m = spawned_manager();
        // Force quelques HP manquants en retirant tous les crystals, frappant, puis…
        // ici on vérifie juste que le tick soigne tant que des crystals existent.
        m.dragon.hp = 100.0;
        m.tick(true, 42);
        assert!(m.dragon.hp > 100.0, "crystals should heal the dragon");
    }
}
