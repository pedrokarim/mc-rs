//! Controllers — traduisent la [`Memory`] en mouvement/rotation réels du mob.
//! Port de `server/.../ai/controller/` d'Allay (`WalkController`, `LookController`).

use super::{ControlCtx, Controller};
use crate::mob_entities::is_supporting_block;

/// Délai (ticks) entre deux sauts (port `WalkController.JUMP_COOL_DOWN`).
const JUMP_COOL_DOWN: u32 = 10;
/// Impulsion verticale d'un saut (blocs/tick). À ajuster en jeu.
const JUMP_IMPULSE: f32 = 0.42;
/// Hauteur de marche franchissable sans sauter.
const STEP_HEIGHT: f64 = 0.5;

/// Yaw (degrés) depuis un vecteur direction horizontal — port exact
/// `MathUtils.getYawFromVector`.
pub fn yaw_from_vector(dx: f64, dz: f64) -> f64 {
    let length = dx * dx + dz * dz;
    if length == 0.0 {
        return 0.0;
    }
    let yaw = (-dx / length.sqrt()).asin().to_degrees();
    if -dz > 0.0 {
        180.0 - yaw
    } else if yaw.abs() < 1e-10 {
        0.0
    } else {
        yaw
    }
}

/// Pitch (degrés) depuis un vecteur direction 3D — port exact
/// `MathUtils.getPitchFromVector`.
pub fn pitch_from_vector(dx: f64, dy: f64, dz: f64) -> f64 {
    let length = dx * dx + dy * dy + dz * dz;
    if length == 0.0 {
        return 0.0;
    }
    let pitch = (-dy / length.sqrt()).asin().to_degrees();
    if pitch.abs() < 1e-10 {
        0.0
    } else {
        pitch
    }
}

/// Marche au sol : pousse le mob vers `move_dir_end`, saute pour franchir une
/// marche/obstacle. Port `WalkController`.
pub struct WalkController {
    jump_cooldown: u32,
}

impl Default for WalkController {
    fn default() -> Self {
        Self {
            jump_cooldown: JUMP_COOL_DOWN,
        }
    }
}

impl WalkController {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Controller for WalkController {
    fn control(&mut self, ctx: &mut ControlCtx) {
        self.jump_cooldown = self.jump_cooldown.saturating_add(1);

        let Some(end) = ctx.memory.move_dir_end else {
            return;
        };
        let speed = ctx.memory.movement_speed;
        let base = &mut *ctx.base;

        // Ne pas écraser une forte vélocité horizontale (knockback en cours).
        let (vx, vz) = (base.velocity[0], base.velocity[2]);
        if vx * vx + vz * vz > speed * speed * 0.4756 {
            return;
        }

        let dx = end[0] - base.position[0] as f64;
        let dz = end[2] - base.position[2] as f64;
        let h2 = dx * dx + dz * dz;
        let h = h2.sqrt();
        if h < 0.01 {
            return;
        }

        // Vitesse effective bridée pour ne pas dépasser le waypoint.
        let eff = (speed as f64).min(h);
        let factor = eff / h;
        let target_mx = (dx * factor) as f32;
        let target_mz = (dz * factor) as f32;

        // Logique de saut (port conditions A & B).
        let mut motion_y = 0.0_f32;
        if base.on_ground && self.jump_cooldown >= JUMP_COOL_DOWN {
            let dy = end[1] - base.position[1] as f64;
            // A : waypoint plus haut qu'une marche, et horizontalement proche
            // (seuil 2.25 car la route inclut des voisins diagonaux ~√2).
            let mut should_jump = dy > STEP_HEIGHT && h2 < 2.25;
            // B : prochaine avance bloquée par un bloc solide alors qu'on monte.
            if !should_jump && dy > 0.0 {
                should_jump = block_ahead_solid(ctx, dx / h, dz / h);
            }
            if should_jump {
                motion_y = JUMP_IMPULSE;
                self.jump_cooldown = 0;
            }
        }

        // addMotion(targetMx - vx, motionY, targetMz - vz) → vélocité horizontale
        // posée à la cible, impulsion de saut ajoutée à la verticale.
        ctx.base.velocity[0] = target_mx;
        ctx.base.velocity[2] = target_mz;
        if motion_y != 0.0 {
            ctx.base.velocity[1] += motion_y;
        }
    }
}

/// Y a-t-il un bloc solide juste devant le mob (au niveau des pieds) dans la
/// direction de marche normalisée `(nx, nz)` ?
fn block_ahead_solid(ctx: &mut ControlCtx, nx: f64, nz: f64) -> bool {
    let pos = ctx.base.position;
    let ax = (pos[0] as f64 + nx * 0.6).floor() as i32;
    let az = (pos[2] as f64 + nz * 0.6).floor() as i32;
    let ay = pos[1].floor() as i32;
    is_supporting_block(ctx.chunk_cache.get_block(ax, ay, az))
}

/// Vol 3D : déplace le mob directement vers `memory.move_target` en 3 dimensions
/// (sans route ni gravité) et l'oriente vers son mouvement. Pour les volants.
#[derive(Default)]
pub struct FlyController;

impl FlyController {
    pub fn new() -> Self {
        Self
    }
}

impl Controller for FlyController {
    fn control(&mut self, ctx: &mut ControlCtx) {
        let Some(target) = ctx.memory.move_target else {
            ctx.base.velocity = [0.0, 0.0, 0.0];
            return;
        };
        let base = &mut *ctx.base;
        let dx = target[0] - base.position[0] as f64;
        let dy = target[1] - base.position[1] as f64;
        let dz = target[2] - base.position[2] as f64;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist < 0.3 {
            base.velocity = [0.0, 0.0, 0.0];
            return;
        }
        let f = (ctx.memory.movement_speed as f64 / dist).min(1.0);
        base.velocity = [(dx * f) as f32, (dy * f) as f32, (dz * f) as f32];
        // Orientation vers la direction de vol.
        base.yaw = yaw_from_vector(dx, dz) as f32;
        base.body_yaw = base.yaw;
        base.head_yaw = base.yaw;
        base.pitch = pitch_from_vector(dx, dy, dz) as f32;
    }
}

/// Oriente le mob : `look_at_route` aligne le corps sur la direction de marche,
/// `look_at_target` aligne la tête/le pitch sur la cible de regard. Port
/// `LookController`.
pub struct LookController {
    look_at_target: bool,
    look_at_route: bool,
}

impl LookController {
    pub fn new(look_at_target: bool, look_at_route: bool) -> Self {
        Self {
            look_at_target,
            look_at_route,
        }
    }
}

impl Controller for LookController {
    fn control(&mut self, ctx: &mut ControlCtx) {
        let pos = ctx.base.position;
        let (x, y, z) = (pos[0] as f64, pos[1] as f64, pos[2] as f64);
        let mut body_yaw = ctx.base.yaw as f64;
        let mut head_yaw = ctx.base.head_yaw as f64;
        let mut pitch = ctx.base.pitch as f64;

        if self.look_at_route && ctx.memory.has_move_direction() {
            if let Some(end) = ctx.memory.move_dir_end {
                let (rx, ry, rz) = (end[0] - x, end[1] - y, end[2] - z);
                body_yaw = yaw_from_vector(rx, rz);
                if !self.look_at_target {
                    head_yaw = body_yaw;
                    pitch = pitch_from_vector(rx, ry, rz);
                }
            }
        }

        if self.look_at_target {
            if let Some(target) = ctx.memory.look_target {
                let eye = y + eye_height(ctx.kind) as f64;
                let (tx, ty, tz) = (target[0] - x, target[1] - eye, target[2] - z);
                pitch = pitch_from_vector(tx, ty, tz);
                head_yaw = yaw_from_vector(tx, tz);
                body_yaw = head_yaw;
            }
        }

        ctx.base.yaw = body_yaw as f32;
        ctx.base.body_yaw = body_yaw as f32;
        ctx.base.head_yaw = head_yaw as f32;
        ctx.base.pitch = pitch as f32;
    }
}

/// Hauteur des yeux approximative d'un mob (≈ 0.9 × hauteur de hitbox).
fn eye_height(kind: crate::mob_entities::MobKind) -> f32 {
    let (_w, h) = kind.size();
    h * 0.9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_faces_positive_z_as_zero() {
        // Direction +Z (sud) → yaw 0 (convention Bedrock/Allay).
        assert!(yaw_from_vector(0.0, 1.0).abs() < 1e-6);
    }

    #[test]
    fn yaw_faces_negative_x() {
        // Direction -X → yaw 90.
        assert!((yaw_from_vector(-1.0, 0.0) - 90.0).abs() < 1e-6);
    }

    #[test]
    fn pitch_down_is_positive() {
        // Regarder vers le bas (dy < 0) → pitch positif.
        assert!(pitch_from_vector(0.0, -1.0, 0.0) > 0.0);
    }
}
