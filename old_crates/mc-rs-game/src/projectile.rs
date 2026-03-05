//! Projectile physics and damage calculations for arrows and tridents.

/// Result of one physics step: `(position, velocity, pitch_deg, yaw_deg)`.
pub type StepResult = ((f32, f32, f32), (f32, f32, f32), f32, f32);

/// Configuration for a projectile type.
pub struct ProjectileConfig {
    /// Gravity applied per tick (blocks/tick²).
    pub gravity: f32,
    /// Air drag multiplier (applied to velocity each tick).
    pub drag: f32,
    /// Base launch speed (blocks/tick).
    pub base_speed: f32,
    /// Maximum ticks a stuck projectile lives before despawn.
    pub max_stuck_age: u32,
    /// Hit detection radius.
    pub bb_radius: f32,
}

/// Configuration for arrows.
pub fn arrow_config() -> ProjectileConfig {
    ProjectileConfig {
        gravity: 0.05,
        drag: 0.99,
        base_speed: 3.0,
        max_stuck_age: 1200, // 60 seconds
        bb_radius: 0.25,
    }
}

/// Configuration for tridents.
pub fn trident_config() -> ProjectileConfig {
    ProjectileConfig {
        gravity: 0.05,
        drag: 0.99,
        base_speed: 2.5,
        max_stuck_age: 1200,
        bb_radius: 0.25,
    }
}

/// Calculate arrow damage based on charge ticks (0–20) and Power enchantment level.
///
/// - Minimum charge (0–4 ticks): 1.0 damage
/// - Full charge (20 ticks): 6.0 damage
/// - Power adds 1.25 per level
pub fn arrow_damage(charge_ticks: u32, power_level: i16) -> f32 {
    let charge = (charge_ticks.min(20) as f32) / 20.0; // 0.0 – 1.0
    let base = 1.0 + charge * 5.0; // 1.0 – 6.0
    let power_bonus = power_level as f32 * 1.25;
    base + power_bonus
}

/// Calculate launch velocity from pitch/yaw (degrees) and speed.
///
/// Bedrock convention: pitch negative = looking up, yaw 0 = south (+Z).
pub fn launch_velocity(pitch: f32, yaw: f32, speed: f32) -> (f32, f32, f32) {
    let pitch_rad = pitch.to_radians();
    let yaw_rad = yaw.to_radians();
    let horizontal = pitch_rad.cos();
    let vx = -yaw_rad.sin() * horizontal * speed;
    let vy = -pitch_rad.sin() * speed;
    let vz = yaw_rad.cos() * horizontal * speed;
    (vx, vy, vz)
}

/// Step projectile physics for one tick.
///
/// Returns `(new_position, new_velocity, new_pitch_deg, new_yaw_deg)`.
pub fn step_projectile(
    pos: (f32, f32, f32),
    vel: (f32, f32, f32),
    config: &ProjectileConfig,
) -> StepResult {
    // Apply gravity
    let vy = vel.1 - config.gravity;

    // Apply drag
    let vx = vel.0 * config.drag;
    let vy = vy * config.drag;
    let vz = vel.2 * config.drag;

    // Update position
    let nx = pos.0 + vx;
    let ny = pos.1 + vy;
    let nz = pos.2 + vz;

    // Calculate rotation from velocity
    let horizontal = (vx * vx + vz * vz).sqrt();
    let pitch = -(vy.atan2(horizontal)).to_degrees();
    let yaw = (-vx).atan2(vz).to_degrees();

    ((nx, ny, nz), (vx, vy, vz), pitch, yaw)
}

/// Check if a projectile at `pos` collides with any entity in the list.
///
/// Each entity is `(runtime_id, x, y, z, bb_width, bb_height)`.
/// Returns the `runtime_id` of the first hit entity, skipping `shooter_rid`.
pub fn check_entity_collision(
    pos: (f32, f32, f32),
    entities: &[(u64, f32, f32, f32, f32, f32)],
    shooter_rid: u64,
    hit_radius: f32,
) -> Option<u64> {
    for &(rid, ex, ey, ez, width, height) in entities {
        if rid == shooter_rid {
            continue;
        }
        let half_w = width / 2.0 + hit_radius;
        let dx = (pos.0 - ex).abs();
        let dz = (pos.2 - ez).abs();
        if dx > half_w || dz > half_w {
            continue;
        }
        // Vertical check: entity AABB from ey to ey+height
        if pos.1 >= ey - hit_radius && pos.1 <= ey + height + hit_radius {
            return Some(rid);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_velocity_straight_ahead() {
        // pitch=0, yaw=0 → straight south (+Z)
        let (vx, vy, vz) = launch_velocity(0.0, 0.0, 3.0);
        assert!(vx.abs() < 0.01);
        assert!(vy.abs() < 0.01);
        assert!((vz - 3.0).abs() < 0.01);
    }

    #[test]
    fn launch_velocity_up() {
        // pitch=-90 → straight up
        let (vx, vy, vz) = launch_velocity(-90.0, 0.0, 3.0);
        assert!(vx.abs() < 0.01);
        assert!((vy - 3.0).abs() < 0.01);
        assert!(vz.abs() < 0.01);
    }

    #[test]
    fn launch_velocity_down() {
        // pitch=90 → straight down
        let (vx, vy, vz) = launch_velocity(90.0, 0.0, 3.0);
        assert!(vx.abs() < 0.01);
        assert!((vy - (-3.0)).abs() < 0.01);
        assert!(vz.abs() < 0.01);
    }

    #[test]
    fn step_projectile_gravity() {
        let config = arrow_config();
        let pos = (0.0, 100.0, 0.0);
        let vel = (0.0, 0.0, 1.0);
        let (new_pos, new_vel, _pitch, _yaw) = step_projectile(pos, vel, &config);

        // Gravity should make vy negative
        assert!(new_vel.1 < 0.0);
        // Position should move forward in Z
        assert!(new_pos.2 > 0.0);
        // Y should decrease (gravity)
        assert!(new_pos.1 < 100.0);
    }

    #[test]
    fn arrow_damage_min_charge() {
        let dmg = arrow_damage(0, 0);
        assert!((dmg - 1.0).abs() < 0.01);
    }

    #[test]
    fn arrow_damage_full_charge() {
        let dmg = arrow_damage(20, 0);
        assert!((dmg - 6.0).abs() < 0.01);
    }

    #[test]
    fn arrow_damage_power_bonus() {
        // Full charge + Power III = 6.0 + 3.75 = 9.75
        let dmg = arrow_damage(20, 3);
        assert!((dmg - 9.75).abs() < 0.01);
    }

    #[test]
    fn entity_collision_hit() {
        let entities = vec![(42, 5.0, 10.0, 5.0, 0.6, 1.8)];
        let result = check_entity_collision((5.1, 10.5, 5.0), &entities, 1, 0.25);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn entity_collision_miss() {
        let entities = vec![(42, 5.0, 10.0, 5.0, 0.6, 1.8)];
        let result = check_entity_collision((10.0, 10.0, 10.0), &entities, 1, 0.25);
        assert!(result.is_none());
    }

    #[test]
    fn entity_collision_skip_shooter() {
        let entities = vec![(1, 5.0, 10.0, 5.0, 0.6, 1.8)];
        // Shooter is entity 1 — should be skipped
        let result = check_entity_collision((5.0, 10.5, 5.0), &entities, 1, 0.25);
        assert!(result.is_none());
    }
}
