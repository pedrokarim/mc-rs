//! Executors — la logique exécutée par les behaviors. Port de
//! `server/.../ai/executor/` d'Allay (MeleeAttack, FlatRandomRoam) + un Panic/Flee.

use rand::Rng;

use super::behavior::Executor;
use super::memory::Memory;
use super::{AiEffect, ExecCtx};
use crate::entity::EntityBase;

/// Distance² « cible atteinte » pour le roam.
const TARGET_REACHED_DIST_SQ: f64 = 1.0;
/// Ticks sans progrès avant d'abandonner une cible de roam.
const MAX_STUCK_TICKS: u32 = 40;

/// Position 3D d'un joueur par runtime id (cherchée dans le snapshot).
fn player_pos(players: &[super::PlayerSnapshot], runtime_id: u64) -> Option<[f32; 3]> {
    players
        .iter()
        .find(|p| p.runtime_id == runtime_id && p.is_attackable())
        .map(|p| p.position)
}

/// Le mob est-il blessé (HP courant < HP max) ?
fn is_injured(base: &EntityBase) -> bool {
    base.attributes
        .iter()
        .find(|a| a.name == "minecraft:health")
        .map(|a| a.current < a.max)
        .unwrap_or(false)
}

/// Distance² 3D entre deux positions.
fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = (a[0] - b[0]) as f64;
    let dy = (a[1] - b[1]) as f64;
    let dz = (a[2] - b[2]) as f64;
    dx * dx + dy * dy + dz * dz
}

// ---------------------------------------------------------------------------
// Attaque mêlée (chasse + frappe) — port `MeleeAttackExecutor`
// ---------------------------------------------------------------------------

pub struct MeleeAttackExecutor {
    speed: f32,
    max_sense_range_sq: f64,
    attack_range_sq: f64,
    cooldown: u32,
    attack_tick: u32,
    last_target_block: Option<[i32; 3]>,
}

impl MeleeAttackExecutor {
    pub fn new(speed: f32, max_sense_range: f64, attack_range: f64, cooldown: u32) -> Self {
        Self {
            speed,
            max_sense_range_sq: max_sense_range * max_sense_range,
            attack_range_sq: attack_range * attack_range,
            cooldown,
            attack_tick: 0,
            last_target_block: None,
        }
    }
}

impl Executor for MeleeAttackExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.attack_tick = 0;
        self.last_target_block = None;
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        self.attack_tick += 1;

        let Some(target_id) = ctx.memory.nearest_player else {
            return false;
        };
        let Some(tpos) = player_pos(ctx.players, target_id) else {
            return false;
        };

        let d2 = dist_sq(ctx.base.position, tpos);
        if d2 > self.max_sense_range_sq {
            return false;
        }

        // Chasse : pose la cible de mouvement (le RouteFinder s'en occupe) + la
        // cible de regard (yeux du joueur, hauteur approximée).
        ctx.memory.move_target = Some([tpos[0] as f64, tpos[1] as f64, tpos[2] as f64]);
        ctx.memory.look_target = Some([tpos[0] as f64, (tpos[1] + 1.62) as f64, tpos[2] as f64]);

        // Force un recalcul de route si la cible a changé de bloc.
        let tblock = [
            tpos[0].floor() as i32,
            tpos[1].floor() as i32,
            tpos[2].floor() as i32,
        ];
        if self.last_target_block != Some(tblock) {
            ctx.memory.route_update_required = true;
            self.last_target_block = Some(tblock);
        }

        // Frappe si à portée et cooldown écoulé.
        if d2 <= self.attack_range_sq && self.attack_tick > self.cooldown {
            let damage = ctx.kind.attack_damage();
            if damage > 0.0 {
                ctx.effects.push(AiEffect::Attack {
                    attacker_runtime_id: ctx.base.entity_runtime_id,
                    attacker_position: ctx.base.position,
                    target_runtime_id: target_id,
                    damage,
                });
            }
            self.attack_tick = 0;
        }

        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.move_target = None;
        memory.look_target = None;
        memory.clear_move_direction();
        self.last_target_block = None;
    }
}

// ---------------------------------------------------------------------------
// Creeper : chasse + amorçage (fuse) + explosion
// ---------------------------------------------------------------------------

/// Le creeper traque le joueur ; à portée d'amorçage il s'arrête et amorce
/// (fuse). Si le joueur s'éloigne, le fuse se réinitialise. À la fin du fuse, il
/// émet [`AiEffect::Explode`] (le mob est ensuite retiré par `main.rs`).
pub struct CreeperSwellExecutor {
    speed: f32,
    max_sense_range_sq: f64,
    ignite_range_sq: f64,
    fuse_max: u32,
    fuse_tick: u32,
}

impl CreeperSwellExecutor {
    pub fn new(speed: f32, max_sense_range: f64, ignite_range: f64, fuse_max: u32) -> Self {
        Self {
            speed,
            max_sense_range_sq: max_sense_range * max_sense_range,
            ignite_range_sq: ignite_range * ignite_range,
            fuse_max,
            fuse_tick: 0,
        }
    }
}

impl Executor for CreeperSwellExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.fuse_tick = 0;
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        let Some(target_id) = ctx.memory.nearest_player else {
            return false;
        };
        let Some(tpos) = player_pos(ctx.players, target_id) else {
            return false;
        };
        let d2 = dist_sq(ctx.base.position, tpos);
        if d2 > self.max_sense_range_sq {
            return false;
        }

        ctx.memory.look_target = Some([tpos[0] as f64, (tpos[1] + 1.62) as f64, tpos[2] as f64]);

        if d2 <= self.ignite_range_sq {
            // À portée : on s'arrête et on amorce.
            ctx.memory.move_target = None;
            ctx.memory.clear_move_direction();
            self.fuse_tick += 1;
            if self.fuse_tick >= self.fuse_max {
                ctx.effects.push(AiEffect::Explode {
                    attacker_runtime_id: ctx.base.entity_runtime_id,
                    center: ctx.base.position,
                });
                return false; // explose → le mob sera retiré
            }
        } else {
            // Joueur hors de portée : on désamorce et on continue de traquer.
            self.fuse_tick = 0;
            ctx.memory.move_target = Some([tpos[0] as f64, tpos[1] as f64, tpos[2] as f64]);
            ctx.memory.route_update_required = true;
        }

        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.fuse_tick = 0;
        memory.move_target = None;
        memory.look_target = None;
        memory.clear_move_direction();
    }
}

// ---------------------------------------------------------------------------
// Skeleton : tir à l'arc (kite + visée balistique)
// ---------------------------------------------------------------------------

/// Hauteur des yeux du tireur (≈ skeleton).
const SHOOTER_EYE: f32 = 1.62;
/// Gravité d'une flèche (doit correspondre à `arrow_entity::ARROW_GRAVITY`),
/// utilisée pour compenser la chute lors de la visée.
const ARROW_GRAVITY_COMP: f32 = 0.05;

/// Le squelette garde ses distances (recule si trop près, s'approche si trop
/// loin), vise le joueur et tire des flèches via [`AiEffect::ShootArrow`].
pub struct BowAttackExecutor {
    speed: f32,
    max_sense_range_sq: f64,
    min_range_sq: f64,
    shoot_range_sq: f64,
    arrow_speed: f32,
    damage: f32,
    cooldown: u32,
    draw_tick: u32,
}

impl BowAttackExecutor {
    pub fn new(
        speed: f32,
        max_sense_range: f64,
        min_range: f64,
        shoot_range: f64,
        arrow_speed: f32,
        damage: f32,
        cooldown: u32,
    ) -> Self {
        Self {
            speed,
            max_sense_range_sq: max_sense_range * max_sense_range,
            min_range_sq: min_range * min_range,
            shoot_range_sq: shoot_range * shoot_range,
            arrow_speed,
            damage,
            cooldown,
            draw_tick: 0,
        }
    }
}

impl Executor for BowAttackExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.draw_tick = 0;
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        let Some(target_id) = ctx.memory.nearest_player else {
            return false;
        };
        let Some(tpos) = player_pos(ctx.players, target_id) else {
            return false;
        };
        let d2 = dist_sq(ctx.base.position, tpos);
        if d2 > self.max_sense_range_sq {
            return false;
        }

        ctx.memory.look_target = Some([tpos[0] as f64, (tpos[1] + 1.62) as f64, tpos[2] as f64]);

        if d2 < self.min_range_sq {
            // Trop près : on recule (kite).
            let dx = (ctx.base.position[0] - tpos[0]) as f64;
            let dz = (ctx.base.position[2] - tpos[2]) as f64;
            let len = (dx * dx + dz * dz).sqrt().max(0.001);
            ctx.memory.move_target = Some([
                ctx.base.position[0] as f64 + dx / len * 6.0,
                ctx.base.position[1] as f64,
                ctx.base.position[2] as f64 + dz / len * 6.0,
            ]);
            ctx.memory.route_update_required = true;
            self.draw_tick = 0;
        } else if d2 > self.shoot_range_sq {
            // Trop loin : on s'approche.
            ctx.memory.move_target = Some([tpos[0] as f64, tpos[1] as f64, tpos[2] as f64]);
            ctx.memory.route_update_required = true;
            self.draw_tick = 0;
        } else {
            // À bonne distance : on s'arrête, on bande l'arc, puis on tire.
            ctx.memory.move_target = None;
            ctx.memory.clear_move_direction();
            self.draw_tick += 1;
            if self.draw_tick >= self.cooldown {
                self.draw_tick = 0;
                let from = [
                    ctx.base.position[0],
                    ctx.base.position[1] + SHOOTER_EYE,
                    ctx.base.position[2],
                ];
                let target = [tpos[0], tpos[1] + 1.0, tpos[2]];
                let dx = target[0] - from[0];
                let dy = target[1] - from[1];
                let dz = target[2] - from[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
                let horiz = (dx * dx + dz * dz).sqrt();
                // Visée directe + compensation de la chute sur le temps de vol.
                let flight_ticks = horiz / self.arrow_speed;
                let vx = dx / dist * self.arrow_speed;
                let vz = dz / dist * self.arrow_speed;
                let vy = dy / dist * self.arrow_speed + 0.5 * ARROW_GRAVITY_COMP * flight_ticks;
                ctx.effects.push(AiEffect::ShootArrow {
                    shooter_runtime_id: ctx.base.entity_runtime_id,
                    from,
                    velocity: [vx, vy, vz],
                    damage: self.damage,
                });
            }
        }

        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.draw_tick = 0;
        memory.move_target = None;
        memory.look_target = None;
        memory.clear_move_direction();
    }
}

// ---------------------------------------------------------------------------
// Errance aléatoire — port `FlatRandomRoamExecutor`
// ---------------------------------------------------------------------------

pub struct FlatRandomRoamExecutor {
    speed: f32,
    max_roam_range: i32,
    has_target: bool,
    stuck_tick: u32,
    best_dist_sq: f64,
}

impl FlatRandomRoamExecutor {
    pub fn new(speed: f32, max_roam_range: i32) -> Self {
        Self {
            speed,
            max_roam_range,
            has_target: false,
            stuck_tick: 0,
            best_dist_sq: f64::MAX,
        }
    }

    fn find_new_target(&mut self, base: &EntityBase, memory: &mut Memory) {
        let mut rng = rand::thread_rng();
        let dx = rng.gen_range(-self.max_roam_range..=self.max_roam_range) as f64;
        let dz = rng.gen_range(-self.max_roam_range..=self.max_roam_range) as f64;
        let target = [
            base.position[0] as f64 + dx,
            base.position[1] as f64,
            base.position[2] as f64 + dz,
        ];
        memory.move_target = Some(target);
        memory.route_update_required = true;
        self.has_target = true;
        self.stuck_tick = 0;
        self.best_dist_sq = f64::MAX;
    }
}

impl Executor for FlatRandomRoamExecutor {
    fn on_start(&mut self, memory: &mut Memory, base: &mut EntityBase) {
        self.has_target = false;
        memory.movement_speed = self.speed;
        self.find_new_target(base, memory);
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        // Regard « idle » : on fixe le joueur le plus proche s'il y en a un.
        match ctx.memory.nearest_player.and_then(|pid| {
            ctx.players.iter().find(|p| p.runtime_id == pid).copied()
        }) {
            Some(p) => {
                ctx.memory.look_target =
                    Some([p.position[0] as f64, (p.position[1] + 1.62) as f64, p.position[2] as f64]);
            }
            None => ctx.memory.look_target = None,
        }

        if let Some(target) = ctx.memory.move_target {
            let dx = target[0] - ctx.base.position[0] as f64;
            let dz = target[2] - ctx.base.position[2] as f64;
            let d2 = dx * dx + dz * dz;
            if d2 < TARGET_REACHED_DIST_SQ {
                self.has_target = false; // arrivé → nouvelle cible au prochain tick
            } else if d2 + 0.0625 < self.best_dist_sq {
                self.best_dist_sq = d2;
                self.stuck_tick = 0;
            } else {
                self.stuck_tick += 1;
                if self.stuck_tick >= MAX_STUCK_TICKS {
                    self.has_target = false; // coincé → abandonne
                }
            }
        } else {
            self.has_target = false;
        }

        if !self.has_target {
            let base = &*ctx.base;
            let mut rng = rand::thread_rng();
            let dx = rng.gen_range(-self.max_roam_range..=self.max_roam_range) as f64;
            let dz = rng.gen_range(-self.max_roam_range..=self.max_roam_range) as f64;
            ctx.memory.move_target = Some([
                base.position[0] as f64 + dx,
                base.position[1] as f64,
                base.position[2] as f64 + dz,
            ]);
            ctx.memory.route_update_required = true;
            self.has_target = true;
            self.stuck_tick = 0;
            self.best_dist_sq = f64::MAX;
        }

        true // roam tourne indéfiniment (priorité basse)
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.move_target = None;
        memory.clear_move_direction();
        self.has_target = false;
    }
}

// ---------------------------------------------------------------------------
// Panique / fuite (mobs passifs blessés)
// ---------------------------------------------------------------------------

/// Le mob fuit en ligne droite à l'opposé du joueur le plus proche tant qu'il
/// est blessé. Inspiré du `PanicBehavior` vanilla (simplifié).
pub struct PanicFleeExecutor {
    speed: f32,
    flee_distance: f64,
}

impl PanicFleeExecutor {
    pub fn new(speed: f32, flee_distance: f64) -> Self {
        Self {
            speed,
            flee_distance,
        }
    }

    /// Vrai si le mob est blessé ET un joueur est proche (évaluateur de la fuite).
    pub fn should_flee(memory: &Memory, base: &EntityBase, _players: &[super::PlayerSnapshot]) -> bool {
        memory.nearest_player.is_some() && is_injured(base)
    }
}

impl Executor for PanicFleeExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        let Some(target_id) = ctx.memory.nearest_player else {
            return false;
        };
        let Some(tpos) = player_pos(ctx.players, target_id) else {
            return false;
        };
        if !is_injured(ctx.base) {
            return false; // plus blessé → arrête de paniquer
        }

        // Direction opposée au joueur, normalisée.
        let dx = (ctx.base.position[0] - tpos[0]) as f64;
        let dz = (ctx.base.position[2] - tpos[2]) as f64;
        let len = (dx * dx + dz * dz).sqrt().max(0.001);
        let flee = [
            ctx.base.position[0] as f64 + dx / len * self.flee_distance,
            ctx.base.position[1] as f64,
            ctx.base.position[2] as f64 + dz / len * self.flee_distance,
        ];
        ctx.memory.move_target = Some(flee);
        ctx.memory.route_update_required = true;
        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.move_target = None;
        memory.clear_move_direction();
    }
}

// ---------------------------------------------------------------------------
// Tempt : suivre un joueur tenant la nourriture du mob (passifs)
// ---------------------------------------------------------------------------

/// Le mob suit le joueur le plus proche (dans `range`) tenant un item tentant
/// (network id dans `food_ids`). Comportement passif vanilla.
pub struct TemptExecutor {
    speed: f32,
    food_ids: Vec<i32>,
    range_sq: f64,
}

impl TemptExecutor {
    pub fn new(speed: f32, food_ids: Vec<i32>, range: f64) -> Self {
        Self {
            speed,
            food_ids,
            range_sq: range * range,
        }
    }

    /// Position du joueur tentant le plus proche (vivant, item tentant, en portée).
    fn tempting_player(
        food_ids: &[i32],
        range_sq: f64,
        base: &EntityBase,
        players: &[super::PlayerSnapshot],
    ) -> Option<[f32; 3]> {
        players
            .iter()
            .filter(|p| p.alive && food_ids.contains(&p.held_item))
            .map(|p| (p.position, dist_sq(base.position, p.position)))
            .filter(|(_, d)| *d <= range_sq)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(pos, _)| pos)
    }
}

impl Executor for TemptExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        let Some(tpos) = Self::tempting_player(&self.food_ids, self.range_sq, ctx.base, ctx.players)
        else {
            return false;
        };
        ctx.memory.move_target = Some([tpos[0] as f64, tpos[1] as f64, tpos[2] as f64]);
        ctx.memory.look_target = Some([tpos[0] as f64, (tpos[1] + 1.62) as f64, tpos[2] as f64]);
        ctx.memory.route_update_required = true;
        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.move_target = None;
        memory.look_target = None;
        memory.clear_move_direction();
    }
}

// ---------------------------------------------------------------------------
// Errance volante 3D (mobs volants : chauve-souris, perroquet, allay, abeille)
// ---------------------------------------------------------------------------

/// Erre en 3D : choisit des cibles aléatoires autour de la position courante
/// (XZ dans `range`, Y légèrement variable). Le `FlyController` y déplace le mob.
pub struct FlyRoamExecutor {
    speed: f32,
    range: i32,
}

impl FlyRoamExecutor {
    pub fn new(speed: f32, range: i32) -> Self {
        Self { speed, range }
    }

    fn pick_target(&self, base: &EntityBase, memory: &mut Memory) {
        let mut rng = rand::thread_rng();
        let dx = rng.gen_range(-self.range..=self.range) as f64;
        let dz = rng.gen_range(-self.range..=self.range) as f64;
        let dy = rng.gen_range(-2..=4) as f64; // dérive verticale douce
        memory.move_target = Some([
            base.position[0] as f64 + dx,
            base.position[1] as f64 + dy,
            base.position[2] as f64 + dz,
        ]);
    }
}

impl Executor for FlyRoamExecutor {
    fn on_start(&mut self, memory: &mut Memory, base: &mut EntityBase) {
        memory.movement_speed = self.speed;
        self.pick_target(base, memory);
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        let reached = ctx
            .memory
            .move_target
            .map(|t| {
                let dx = t[0] - ctx.base.position[0] as f64;
                let dy = t[1] - ctx.base.position[1] as f64;
                let dz = t[2] - ctx.base.position[2] as f64;
                dx * dx + dy * dy + dz * dz < 1.0
            })
            .unwrap_or(true);
        if reached {
            self.pick_target(ctx.base, ctx.memory);
        }
        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        memory.move_target = None;
    }
}

// ---------------------------------------------------------------------------
// Tireur volant (ghast / blaze) : plane à distance et lance des boules de feu
// ---------------------------------------------------------------------------

pub struct FlyShootExecutor {
    speed: f32,
    max_sense_range_sq: f64,
    shoot_range_sq: f64,
    fireball_speed: f32,
    damage: f32,
    cooldown: u32,
    fireball_type: &'static str,
    tick: u32,
}

impl FlyShootExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        speed: f32,
        max_sense_range: f64,
        shoot_range: f64,
        fireball_speed: f32,
        damage: f32,
        cooldown: u32,
        fireball_type: &'static str,
    ) -> Self {
        Self {
            speed,
            max_sense_range_sq: max_sense_range * max_sense_range,
            shoot_range_sq: shoot_range * shoot_range,
            fireball_speed,
            damage,
            cooldown,
            fireball_type,
            tick: 0,
        }
    }
}

impl Executor for FlyShootExecutor {
    fn on_start(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.tick = 0;
        memory.movement_speed = self.speed;
    }

    fn execute(&mut self, ctx: &mut ExecCtx) -> bool {
        self.tick += 1;
        let Some(target_id) = ctx.memory.nearest_player else {
            return false;
        };
        let Some(tpos) = player_pos(ctx.players, target_id) else {
            return false;
        };
        let d2 = dist_sq(ctx.base.position, tpos);
        if d2 > self.max_sense_range_sq {
            return false;
        }

        // Plane : approche si trop loin, sinon reste sur place (vol stationnaire).
        if d2 > self.shoot_range_sq {
            ctx.memory.move_target =
                Some([tpos[0] as f64, (tpos[1] + 3.0) as f64, tpos[2] as f64]);
        } else {
            // Vol stationnaire à hauteur de tir.
            ctx.memory.move_target = Some([
                ctx.base.position[0] as f64,
                ctx.base.position[1] as f64,
                ctx.base.position[2] as f64,
            ]);
            // Tir périodique vers les yeux du joueur.
            if self.tick >= self.cooldown {
                self.tick = 0;
                let from = [
                    ctx.base.position[0],
                    ctx.base.position[1] + 0.5,
                    ctx.base.position[2],
                ];
                let target = [tpos[0], tpos[1] + 1.0, tpos[2]];
                let dx = target[0] - from[0];
                let dy = target[1] - from[1];
                let dz = target[2] - from[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
                let f = self.fireball_speed / dist;
                ctx.effects.push(AiEffect::ShootFireball {
                    shooter_runtime_id: ctx.base.entity_runtime_id,
                    from,
                    velocity: [dx * f, dy * f, dz * f],
                    damage: self.damage,
                    actor_type: self.fireball_type,
                });
            }
        }
        true
    }

    fn on_stop(&mut self, memory: &mut Memory, _base: &mut EntityBase) {
        self.tick = 0;
        memory.move_target = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{ExecCtx, PlayerSnapshot};
    use crate::mob_entities::MobKind;

    fn zombie_base(pos: [f32; 3]) -> EntityBase {
        use crate::entity::{health_attributes, living_metadata};
        EntityBase::new(
            "minecraft:zombie",
            "zombie",
            "Zombie",
            pos,
            health_attributes(20.0),
            living_metadata(0.6, 1.9, None),
        )
    }

    #[test]
    fn melee_emits_attack_when_in_range_after_cooldown() {
        let mut exec = MeleeAttackExecutor::new(0.23, 16.0, 2.0, 5);
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.23);
        memory.nearest_player = Some(7);
        let players = vec![PlayerSnapshot {
            runtime_id: 7,
            position: [1.0, 64.0, 0.0], // distance 1 < portée 2
            gamemode: 0,
            alive: true,
            held_item: 0,
        }];
        let mut effects = Vec::new();

        // Avant la fin du cooldown : pas d'attaque.
        for _ in 0..5 {
            let mut ctx = ExecCtx {
                base: &mut base,
                kind: MobKind::Zombie,
                memory: &mut memory,
                players: &players,
                effects: &mut effects,
            };
            exec.execute(&mut ctx);
        }
        assert!(effects.is_empty(), "pas d'attaque avant la fin du cooldown");

        // Tick suivant : attack_tick > cooldown → frappe.
        let mut ctx = ExecCtx {
            base: &mut base,
            kind: MobKind::Zombie,
            memory: &mut memory,
            players: &players,
            effects: &mut effects,
        };
        exec.execute(&mut ctx);
        assert_eq!(effects.len(), 1, "une attaque émise");
        match effects[0] {
            AiEffect::Attack {
                damage,
                target_runtime_id,
                ..
            } => {
                assert_eq!(target_runtime_id, 7);
                assert_eq!(damage, 3.0);
            }
            _ => panic!("attendu une attaque mêlée"),
        }
    }

    #[test]
    fn melee_stops_without_target() {
        let mut exec = MeleeAttackExecutor::new(0.23, 16.0, 2.0, 5);
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.23); // nearest_player = None
        let mut effects = Vec::new();
        let mut ctx = ExecCtx {
            base: &mut base,
            kind: MobKind::Zombie,
            memory: &mut memory,
            players: &[],
            effects: &mut effects,
        };
        assert!(!exec.execute(&mut ctx), "sans cible → behavior terminé");
    }

    #[test]
    fn creeper_explodes_after_fuse_in_range() {
        let mut exec = CreeperSwellExecutor::new(0.25, 16.0, 3.0, 3); // fuse 3 ticks
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.25);
        memory.nearest_player = Some(9);
        let players = vec![PlayerSnapshot {
            runtime_id: 9,
            position: [1.0, 64.0, 0.0], // distance 1 < portée d'amorçage 3
            gamemode: 0,
            alive: true,
            held_item: 0,
        }];
        let mut effects = Vec::new();
        let mut alive = true;
        for _ in 0..3 {
            let mut ctx = ExecCtx {
                base: &mut base,
                kind: MobKind::Creeper,
                memory: &mut memory,
                players: &players,
                effects: &mut effects,
            };
            alive = exec.execute(&mut ctx);
        }
        assert!(!alive, "explose à la fin du fuse → behavior terminé");
        assert_eq!(effects.len(), 1, "un effet Explode émis");
        assert!(matches!(effects[0], AiEffect::Explode { .. }));
    }

    #[test]
    fn creeper_does_not_explode_out_of_range() {
        let mut exec = CreeperSwellExecutor::new(0.25, 16.0, 3.0, 3);
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.25);
        memory.nearest_player = Some(9);
        let players = vec![PlayerSnapshot {
            runtime_id: 9,
            position: [8.0, 64.0, 0.0], // hors portée d'amorçage → traque, pas de fuse
            gamemode: 0,
            alive: true,
            held_item: 0,
        }];
        let mut effects = Vec::new();
        for _ in 0..10 {
            let mut ctx = ExecCtx {
                base: &mut base,
                kind: MobKind::Creeper,
                memory: &mut memory,
                players: &players,
                effects: &mut effects,
            };
            assert!(exec.execute(&mut ctx), "continue de traquer hors de portée");
        }
        assert!(effects.is_empty(), "pas d'explosion hors de portée d'amorçage");
    }

    #[test]
    fn roam_sets_a_move_target_on_start() {
        let mut exec = FlatRandomRoamExecutor::new(0.2, 8);
        let mut base = zombie_base([100.0, 64.0, 100.0]);
        let mut memory = Memory::new(0.2);
        exec.on_start(&mut memory, &mut base);
        assert!(memory.move_target.is_some(), "le roam pose une cible au démarrage");
    }

    #[test]
    fn fly_shooter_emits_fireball_in_range() {
        let mut exec =
            FlyShootExecutor::new(0.25, 20.0, 14.0, 0.8, 5.0, 3, "minecraft:small_fireball");
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.25);
        memory.nearest_player = Some(2);
        let players = vec![PlayerSnapshot {
            runtime_id: 2,
            position: [5.0, 64.0, 0.0], // distance 5 < portée 14
            gamemode: 0,
            alive: true,
            held_item: 0,
        }];
        let mut effects = Vec::new();
        for _ in 0..4 {
            let mut ctx = ExecCtx {
                base: &mut base,
                kind: MobKind::Blaze,
                memory: &mut memory,
                players: &players,
                effects: &mut effects,
            };
            exec.execute(&mut ctx);
        }
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, AiEffect::ShootFireball { .. })),
            "tire une boule de feu une fois en portée + cooldown écoulé"
        );
    }

    #[test]
    fn tempt_follows_player_holding_food() {
        let wheat = crate::item_registry::network_id("minecraft:wheat").expect("wheat existe");
        let mut exec = TemptExecutor::new(0.2, vec![wheat], 10.0);
        let mut base = zombie_base([0.0, 64.0, 0.0]);
        let mut memory = Memory::new(0.2);
        let mut effects = Vec::new();

        // Joueur tenant du blé à 4 blocs → le mob le suit.
        let holding = vec![PlayerSnapshot {
            runtime_id: 3,
            position: [4.0, 64.0, 0.0],
            gamemode: 0,
            alive: true,
            held_item: wheat,
        }];
        {
            let mut ctx = ExecCtx {
                base: &mut base,
                kind: MobKind::Cow,
                memory: &mut memory,
                players: &holding,
                effects: &mut effects,
            };
            assert!(exec.execute(&mut ctx), "suit le joueur tenant la nourriture");
        }
        assert!(memory.move_target.is_some(), "cible de déplacement posée vers le joueur");

        // Joueur sans nourriture → le tempt s'arrête.
        let empty = vec![PlayerSnapshot {
            held_item: 0,
            ..holding[0]
        }];
        let mut ctx = ExecCtx {
            base: &mut base,
            kind: MobKind::Cow,
            memory: &mut memory,
            players: &empty,
            effects: &mut effects,
        };
        assert!(!exec.execute(&mut ctx), "sans nourriture tenue → pas de tempt");
    }
}
