//! Entités passives non-mobs — port de `.reference/PocketMine-MP/src/entity/object/*`.
//!
//! Contient :
//! - `PrimedTntEntity` (fuse 80 ticks, explosion au bout, physique gravité/drag)
//! - `FallingBlockEntity` (gravité jusqu'à collision sol, devient bloc solide)
//! - `ExperienceOrbEntity` (attirée vers joueur proche, donne de l'XP au ramassage)
//!
//! Chaque entité porte un `entity_runtime_id` unique. La boucle principale
//! (`main.rs`) itère ces entités chaque tick pour les broadcast aux joueurs
//! via `AddActor` / `MoveActorAbsolute` / `RemoveActor`.

use std::collections::HashMap;

// ── Physics / gravity constants (PMMP) ──────────────────────────────────────

pub const DEFAULT_GRAVITY: f32 = 0.04;
pub const DEFAULT_DRAG: f32 = 0.02;
pub const TNT_FUSE_DEFAULT: u32 = 80; // 80 ticks ≈ 4s
pub const TNT_RADIUS: f32 = 4.0;
pub const XP_ORB_DESPAWN_TICKS: u32 = 6000; // 5 min @ 20 TPS
pub const XP_ORB_PICKUP_RANGE: f32 = 1.425; // PMMP `ExperienceOrb::PICKUP_RANGE`
pub const XP_ORB_ATTRACT_RANGE: f32 = 8.0;
pub const FALLING_BLOCK_MAX_AGE: u32 = 600; // 30s

// ── PrimedTNT ───────────────────────────────────────────────────────────────

/// Port de `.reference/PocketMine-MP/src/entity/object/PrimedTNT.php`.
#[derive(Debug, Clone)]
pub struct PrimedTntEntity {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    pub fuse: u32,
    pub age_ticks: u32,
    pub works_underwater: bool,
}

impl PrimedTntEntity {
    pub fn new(entity_unique_id: i64, entity_runtime_id: u64, position: [f32; 3]) -> Self {
        Self {
            entity_unique_id,
            entity_runtime_id,
            position,
            // PMMP `Entity::tryChangeMovement()` : motion initial au spawn
            // pour TNT = (rand*0.02-0.01, 0.2, rand*0.02-0.01). Ici on init à
            // une motion nulle ; les callers peuvent set une motion custom.
            motion: [0.0, 0.2, 0.0],
            fuse: TNT_FUSE_DEFAULT,
            age_ticks: 0,
            works_underwater: false,
        }
    }

    /// Tick physique + décrément fuse. Retourne `Some(explosion_position)` si
    /// la TNT doit exploser ce tick.
    pub fn tick(&mut self) -> Option<[f32; 3]> {
        self.age_ticks = self.age_ticks.saturating_add(1);

        // Physics : drag + gravity (PMMP `entityBaseTick` → `tryChangeMovement`).
        self.motion[0] *= 1.0 - DEFAULT_DRAG;
        self.motion[2] *= 1.0 - DEFAULT_DRAG;
        self.motion[1] -= DEFAULT_GRAVITY;
        self.position[0] += self.motion[0];
        self.position[1] += self.motion[1];
        self.position[2] += self.motion[2];

        if self.fuse > 0 {
            self.fuse -= 1;
        }
        if self.fuse == 0 {
            return Some(self.position);
        }
        None
    }
}

// ── FallingBlock ────────────────────────────────────────────────────────────

/// Port de `.reference/PocketMine-MP/src/entity/object/FallingBlock.php`.
#[derive(Debug, Clone)]
pub struct FallingBlockEntity {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    pub block_runtime_id: u32,
    pub age_ticks: u32,
}

impl FallingBlockEntity {
    pub fn new(
        entity_unique_id: i64,
        entity_runtime_id: u64,
        position: [f32; 3],
        block_runtime_id: u32,
    ) -> Self {
        Self {
            entity_unique_id,
            entity_runtime_id,
            position,
            motion: [0.0, 0.0, 0.0],
            block_runtime_id,
            age_ticks: 0,
        }
    }

    /// Tick physique. Retourne `true` quand l'entité doit se figer en bloc
    /// (`block_runtime_id` à placer à la position actuelle).
    /// `is_block_at` est un closure permettant de tester si un bloc solide
    /// est sous l'entité à une position donnée.
    pub fn tick<F: Fn(i32, i32, i32) -> bool>(&mut self, is_block_at: F) -> bool {
        self.age_ticks += 1;
        if self.age_ticks > FALLING_BLOCK_MAX_AGE {
            return true;
        }
        self.motion[1] -= DEFAULT_GRAVITY;
        self.motion[1] *= 1.0 - DEFAULT_DRAG;
        self.position[0] += self.motion[0];
        self.position[1] += self.motion[1];
        self.position[2] += self.motion[2];

        // Collision sol : bloc solide un peu en dessous.
        let feet = [
            self.position[0].floor() as i32,
            (self.position[1] - 0.05).floor() as i32,
            self.position[2].floor() as i32,
        ];
        is_block_at(feet[0], feet[1], feet[2])
    }
}

// ── ExperienceOrb ───────────────────────────────────────────────────────────

/// Port de `.reference/PocketMine-MP/src/entity/object/ExperienceOrb.php`.
/// Stores XP reward + age. Attracted toward nearest player within range.
#[derive(Debug, Clone)]
pub struct ExperienceOrbEntity {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    pub xp_value: u32,
    pub age_ticks: u32,
    /// Entity runtime ID du joueur qui va ramasser (set quand un joueur est à
    /// portée d'attraction). PMMP `ExperienceOrb::targetPlayerRuntimeId`.
    pub target_player_runtime_id: Option<u64>,
}

impl ExperienceOrbEntity {
    pub fn new(
        entity_unique_id: i64,
        entity_runtime_id: u64,
        position: [f32; 3],
        xp_value: u32,
    ) -> Self {
        Self {
            entity_unique_id,
            entity_runtime_id,
            position,
            motion: [0.0, 0.0, 0.0],
            xp_value,
            age_ticks: 0,
            target_player_runtime_id: None,
        }
    }

    /// Tick. Retourne `PickupResult::Pickup(player_id, xp_amount)` si
    /// le joueur le ramasse ce tick, `PickupResult::Despawn` si âge max,
    /// sinon `None`.
    /// `find_closest_player` retourne (runtime_id, distance, player_pos) du
    /// joueur le plus proche à moins de `XP_ORB_ATTRACT_RANGE`.
    pub fn tick<F>(&mut self, find_closest_player: F) -> OrbTickResult
    where
        F: Fn([f32; 3]) -> Option<(u64, f32, [f32; 3])>,
    {
        self.age_ticks += 1;
        if self.age_ticks > XP_ORB_DESPAWN_TICKS {
            return OrbTickResult::Despawn;
        }

        // Attraction vers le joueur le plus proche.
        if let Some((pid, dist, ppos)) = find_closest_player(self.position) {
            if dist < XP_ORB_PICKUP_RANGE {
                return OrbTickResult::Pickup(pid, self.xp_value);
            }
            self.target_player_runtime_id = Some(pid);
            // Vecteur vers le joueur normalisé puis * 0.1.
            let dx = ppos[0] - self.position[0];
            let dy = ppos[1] - self.position[1];
            let dz = ppos[2] - self.position[2];
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            let factor = (1.0 - dist / XP_ORB_ATTRACT_RANGE).max(0.0) * 0.1;
            self.motion[0] = dx / len * factor;
            self.motion[1] = dy / len * factor;
            self.motion[2] = dz / len * factor;
        } else {
            self.target_player_runtime_id = None;
            // Gravity only.
            self.motion[0] *= 1.0 - DEFAULT_DRAG;
            self.motion[2] *= 1.0 - DEFAULT_DRAG;
            self.motion[1] -= DEFAULT_GRAVITY;
        }
        self.position[0] += self.motion[0];
        self.position[1] += self.motion[1];
        self.position[2] += self.motion[2];
        OrbTickResult::Live
    }
}

pub enum OrbTickResult {
    Live,
    Pickup(u64, u32),
    Despawn,
}

// ── Manager — stocke les 3 types dans un état partagé ───────────────────────

/// Manager qui regroupe toutes les passives entities d'un monde.
#[derive(Default)]
pub struct PassiveEntityManager {
    pub tnt: HashMap<u64, PrimedTntEntity>,
    pub falling_blocks: HashMap<u64, FallingBlockEntity>,
    pub xp_orbs: HashMap<u64, ExperienceOrbEntity>,
}

impl PassiveEntityManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn_tnt(&mut self, entity: PrimedTntEntity) -> u64 {
        let id = entity.entity_runtime_id;
        self.tnt.insert(id, entity);
        id
    }

    pub fn spawn_falling_block(&mut self, entity: FallingBlockEntity) -> u64 {
        let id = entity.entity_runtime_id;
        self.falling_blocks.insert(id, entity);
        id
    }

    pub fn spawn_xp_orb(&mut self, entity: ExperienceOrbEntity) -> u64 {
        let id = entity.entity_runtime_id;
        self.xp_orbs.insert(id, entity);
        id
    }

    pub fn remove(&mut self, runtime_id: u64) {
        self.tnt.remove(&runtime_id);
        self.falling_blocks.remove(&runtime_id);
        self.xp_orbs.remove(&runtime_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tnt_explodes_after_fuse() {
        let mut tnt = PrimedTntEntity::new(1, 1, [0.0, 64.0, 0.0]);
        // 80 ticks fuse. Tick 80 fois.
        let mut exploded_pos = None;
        for _ in 0..80 {
            if let Some(pos) = tnt.tick() {
                exploded_pos = Some(pos);
                break;
            }
        }
        assert!(
            exploded_pos.is_some(),
            "TNT should have exploded by tick 80"
        );
    }

    #[test]
    fn xp_orb_despawns() {
        let mut orb = ExperienceOrbEntity::new(1, 1, [0.0, 64.0, 0.0], 5);
        let nop = |_pos: [f32; 3]| None;
        let mut despawned = false;
        for _ in 0..(XP_ORB_DESPAWN_TICKS + 1) {
            if matches!(orb.tick(nop), OrbTickResult::Despawn) {
                despawned = true;
                break;
            }
        }
        assert!(despawned);
    }

    #[test]
    fn xp_orb_picked_up_when_player_near() {
        let mut orb = ExperienceOrbEntity::new(1, 1, [0.0, 64.0, 0.0], 7);
        let near = |pos: [f32; 3]| {
            // joueur à distance 0.5
            Some((42u64, 0.5, [pos[0] + 0.5, pos[1], pos[2]]))
        };
        match orb.tick(near) {
            OrbTickResult::Pickup(pid, xp) => {
                assert_eq!(pid, 42);
                assert_eq!(xp, 7);
            }
            _ => panic!("expected pickup"),
        }
    }

    #[test]
    fn falling_block_lands_when_ground_below() {
        let mut fb = FallingBlockEntity::new(1, 1, [0.5, 65.0, 0.5], 1);
        let has_ground = |_x: i32, y: i32, _z: i32| y < 64;
        let mut landed = false;
        for _ in 0..100 {
            if fb.tick(has_ground) {
                landed = true;
                break;
            }
        }
        assert!(landed);
    }
}
