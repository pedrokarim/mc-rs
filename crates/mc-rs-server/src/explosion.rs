//! Explosion — port de `.reference/PocketMine-MP/src/world/Explosion.php`.
//!
//! Calcule les blocs détruits + damage radial. Utilisé par PrimedTNT,
//! creeper, bed (nether), respawn anchor (overworld), ghast fireball, wither.

/// Source d'explosion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplosionSource {
    Tnt,
    Creeper,
    Bed,
    RespawnAnchor,
    Ghast,
    Wither,
    EndCrystal,
    Custom,
}

impl ExplosionSource {
    /// Rayon d'explosion PMMP `PrimedTNT::RADIUS` / Bedrock values.
    pub fn default_radius(&self) -> f32 {
        match self {
            Self::Tnt => 4.0,
            Self::Creeper => 3.0,
            Self::Bed => 5.0,
            Self::RespawnAnchor => 5.0,
            Self::Ghast => 1.0,
            Self::Wither => 7.0,
            Self::EndCrystal => 6.0,
            Self::Custom => 4.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Explosion {
    pub source: ExplosionSource,
    pub center: [f32; 3],
    pub radius: f32,
    pub break_blocks: bool,
    pub fire_chance: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ExplosionResult {
    /// Positions des blocs détruits (à remplacer par air).
    pub blocks_destroyed: Vec<[i32; 3]>,
    /// Entités impactées avec leur damage.
    pub entity_damages: Vec<(u64, f32)>,
    /// Positions où du feu doit être placé.
    pub fire_positions: Vec<[i32; 3]>,
}

impl Explosion {
    pub fn new(source: ExplosionSource, center: [f32; 3]) -> Self {
        Self {
            source,
            center,
            radius: source.default_radius(),
            break_blocks: true,
            fire_chance: match source {
                ExplosionSource::Ghast | ExplosionSource::Wither => 1.0 / 3.0,
                _ => 0.0,
            },
        }
    }

    /// Test si une entité à `position` est dans la zone d'explosion.
    /// Retourne `Some(damage)` avec le damage calculé selon la distance.
    pub fn entity_damage(&self, position: [f32; 3]) -> Option<f32> {
        let dx = position[0] - self.center[0];
        let dy = position[1] - self.center[1];
        let dz = position[2] - self.center[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist > self.radius * 2.0 {
            return None;
        }
        // PMMP formule simplifiée (entity_attack damage).
        let impact = (1.0 - dist / (self.radius * 2.0)).max(0.0);
        let base_damage = (impact * impact * 7.0 + impact) * self.radius * 2.0;
        if base_damage > 0.0 {
            Some(base_damage)
        } else {
            None
        }
    }

    /// Calcule les blocs à détruire dans un rayon sphérique simplifié.
    /// PMMP fait du raycast ; ici on approxime via itération sphère.
    /// `is_block_breakable` : closure décidant si un bloc résiste (bedrock, etc.).
    /// `FnMut` pour autoriser un prédicat qui lit le monde via `&mut ChunkCache`.
    pub fn compute_result<F>(&self, mut is_block_breakable: F) -> ExplosionResult
    where
        F: FnMut(i32, i32, i32) -> bool,
    {
        let mut result = ExplosionResult::default();
        if !self.break_blocks {
            return result;
        }
        let r = self.radius.ceil() as i32;
        let cx = self.center[0].floor() as i32;
        let cy = self.center[1].floor() as i32;
        let cz = self.center[2].floor() as i32;
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    let bx = cx + dx;
                    let by = cy + dy;
                    let bz = cz + dz;
                    let d = ((dx * dx + dy * dy + dz * dz) as f32).sqrt();
                    if d > self.radius {
                        continue;
                    }
                    if is_block_breakable(bx, by, bz) {
                        result.blocks_destroyed.push([bx, by, bz]);
                        // Chance de fire au bord de l'explosion.
                        if self.fire_chance > 0.0 && d > self.radius * 0.7 {
                            result.fire_positions.push([bx, by + 1, bz]);
                        }
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tnt_radius_4() {
        let e = Explosion::new(ExplosionSource::Tnt, [0.0, 64.0, 0.0]);
        assert_eq!(e.radius, 4.0);
    }

    #[test]
    fn damage_decreases_with_distance() {
        let e = Explosion::new(ExplosionSource::Tnt, [0.0, 0.0, 0.0]);
        let close = e.entity_damage([1.0, 0.0, 0.0]).unwrap();
        let far = e.entity_damage([5.0, 0.0, 0.0]).unwrap();
        assert!(close > far);
    }

    #[test]
    fn breaks_blocks_in_radius() {
        let e = Explosion::new(ExplosionSource::Tnt, [0.0, 0.0, 0.0]);
        let result = e.compute_result(|_x, _y, _z| true);
        assert!(!result.blocks_destroyed.is_empty());
    }
}
