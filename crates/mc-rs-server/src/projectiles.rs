//! Projectiles — port sélectif de `.reference/PocketMine-MP/src/entity/projectile/*`.
//!
//! Arrow, Egg, Snowball, EnderPearl, FishingHook, Trident, ThrownPotion, etc.
//! Tick physics : gravity, drag, collision ground/entity.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileKind {
    Arrow,
    Egg,
    Snowball,
    EnderPearl,
    ExperienceBottle,
    FishingHook,
    SplashPotion,
    LingeringPotion,
    Trident,
    FireCharge,
    WitherSkull,
    FireworkRocket,
    ShulkerBullet,
}

impl ProjectileKind {
    pub fn network_identifier(&self) -> &'static str {
        match self {
            Self::Arrow => "minecraft:arrow",
            Self::Egg => "minecraft:egg",
            Self::Snowball => "minecraft:snowball",
            Self::EnderPearl => "minecraft:ender_pearl",
            Self::ExperienceBottle => "minecraft:xp_bottle",
            Self::FishingHook => "minecraft:fishing_hook",
            Self::SplashPotion => "minecraft:splash_potion",
            Self::LingeringPotion => "minecraft:lingering_potion",
            Self::Trident => "minecraft:thrown_trident",
            Self::FireCharge => "minecraft:small_fireball",
            Self::WitherSkull => "minecraft:wither_skull",
            Self::FireworkRocket => "minecraft:fireworks_rocket",
            Self::ShulkerBullet => "minecraft:shulker_bullet",
        }
    }

    /// Base damage (sans vitesse). PMMP `Projectile::getBaseDamage()`.
    pub fn base_damage(&self) -> f32 {
        match self {
            Self::Arrow => 2.0,
            Self::Trident => 8.0,
            Self::WitherSkull => 5.0,
            Self::FireCharge => 5.0,
            _ => 0.0, // snowball, egg etc. ne font pas de damage (sauf blaze)
        }
    }

    /// Gravité appliquée par tick. PMMP `Projectile::getGravity()`.
    pub fn gravity(&self) -> f32 {
        match self {
            Self::Arrow | Self::Trident => 0.05,
            Self::Egg | Self::Snowball | Self::EnderPearl | Self::ExperienceBottle => 0.03,
            Self::SplashPotion | Self::LingeringPotion => 0.05,
            Self::FishingHook => 0.03,
            Self::FireCharge | Self::WitherSkull | Self::ShulkerBullet => 0.0,
            Self::FireworkRocket => -0.01, // remontée
        }
    }

    /// Drag (résistance air). PMMP `Projectile::getDrag()`.
    pub fn drag(&self) -> f32 {
        match self {
            Self::Arrow | Self::Trident => 0.01,
            _ => 0.01,
        }
    }
}

/// État d'un projectile en vol.
#[derive(Debug, Clone)]
pub struct ProjectileEntity {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub kind: ProjectileKind,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    /// Entity qui a lancé le projectile (pour ne pas se toucher soi-même).
    pub shooter_runtime_id: Option<u64>,
    /// Ticks vécus. Despawn after ~1200 (60s).
    pub age_ticks: u32,
    /// Flagged pour landing/despawn.
    pub flagged_for_despawn: bool,
}

impl ProjectileEntity {
    pub fn new(
        entity_unique_id: i64,
        entity_runtime_id: u64,
        kind: ProjectileKind,
        position: [f32; 3],
        motion: [f32; 3],
        shooter_runtime_id: Option<u64>,
    ) -> Self {
        Self {
            entity_unique_id,
            entity_runtime_id,
            kind,
            position,
            motion,
            shooter_runtime_id,
            age_ticks: 0,
            flagged_for_despawn: false,
        }
    }

    /// Tick physique. Retourne `true` si le projectile a touché un bloc.
    /// `is_block_at` : closure pour tester collision.
    pub fn tick<F: Fn(i32, i32, i32) -> bool>(&mut self, is_block_at: F) -> bool {
        self.age_ticks += 1;
        if self.age_ticks > 1200 {
            self.flagged_for_despawn = true;
            return false;
        }

        let drag = self.kind.drag();
        let gravity = self.kind.gravity();

        self.motion[0] *= 1.0 - drag;
        self.motion[2] *= 1.0 - drag;
        self.motion[1] = (self.motion[1] * (1.0 - drag)) - gravity;
        self.position[0] += self.motion[0];
        self.position[1] += self.motion[1];
        self.position[2] += self.motion[2];

        let block_hit = is_block_at(
            self.position[0].floor() as i32,
            self.position[1].floor() as i32,
            self.position[2].floor() as i32,
        );
        if block_hit {
            self.motion = [0.0, 0.0, 0.0];
            // Les arrows restent fichés ; les autres despawn.
            if !matches!(self.kind, ProjectileKind::Arrow | ProjectileKind::Trident) {
                self.flagged_for_despawn = true;
            }
        }
        block_hit
    }

    /// Damage effectif basé sur la vitesse actuelle. PMMP `ArrowHit::getResultDamage`.
    pub fn current_damage(&self) -> f32 {
        let speed = (self.motion[0] * self.motion[0]
            + self.motion[1] * self.motion[1]
            + self.motion[2] * self.motion[2])
            .sqrt();
        self.kind.base_damage() * speed.max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_has_gravity() {
        let mut arrow = ProjectileEntity::new(
            1,
            1,
            ProjectileKind::Arrow,
            [0.0, 100.0, 0.0],
            [1.0, 0.0, 0.0],
            None,
        );
        let no_blocks = |_x: i32, _y: i32, _z: i32| false;
        for _ in 0..20 {
            arrow.tick(no_blocks);
        }
        // Y devrait être inférieur (tombé par gravité).
        assert!(arrow.position[1] < 100.0);
        assert!(arrow.position[0] > 0.0); // avancé en X
    }

    #[test]
    fn arrow_stops_on_block() {
        let mut arrow = ProjectileEntity::new(
            1,
            1,
            ProjectileKind::Arrow,
            [0.0, 100.0, 0.0],
            [1.0, 0.0, 0.0],
            None,
        );
        let wall = |x: i32, _y: i32, _z: i32| x >= 2;
        for _ in 0..20 {
            arrow.tick(wall);
        }
        assert_eq!(arrow.motion, [0.0, 0.0, 0.0]);
        // Arrow sticks, not despawn.
        assert!(!arrow.flagged_for_despawn);
    }

    #[test]
    fn snowball_despawns_on_hit() {
        let mut snow = ProjectileEntity::new(
            1,
            1,
            ProjectileKind::Snowball,
            [0.0, 100.0, 0.0],
            [1.0, 0.0, 0.0],
            None,
        );
        let wall = |x: i32, _y: i32, _z: i32| x >= 2;
        for _ in 0..20 {
            snow.tick(wall);
        }
        assert!(snow.flagged_for_despawn);
    }

    #[test]
    fn projectile_identifier() {
        assert_eq!(ProjectileKind::Arrow.network_identifier(), "minecraft:arrow");
        assert_eq!(ProjectileKind::EnderPearl.network_identifier(), "minecraft:ender_pearl");
    }
}
