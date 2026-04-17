//! Fish — Cod, Salmon, Pufferfish, TropicalFish + bucket capture.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishKind {
    Cod,
    Salmon,
    Pufferfish,
    TropicalFish,
}

impl FishKind {
    pub fn bucket_item(&self) -> &'static str {
        match self {
            Self::Cod => "minecraft:cod_bucket",
            Self::Salmon => "minecraft:salmon_bucket",
            Self::Pufferfish => "minecraft:pufferfish_bucket",
            Self::TropicalFish => "minecraft:tropical_fish_bucket",
        }
    }

    pub fn raw_drop(&self) -> &'static str {
        match self {
            Self::Cod => "minecraft:raw_cod",
            Self::Salmon => "minecraft:raw_salmon",
            Self::Pufferfish => "minecraft:pufferfish",
            Self::TropicalFish => "minecraft:tropical_fish",
        }
    }

    pub fn is_hostile(&self) -> bool {
        matches!(self, Self::Pufferfish)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PufferfishState {
    Small,  // 0
    Medium, // 1
    Full,   // 2
}

#[derive(Debug, Clone)]
pub struct Pufferfish {
    pub state: PufferfishState,
    pub inflate_cooldown: u32,
}

/// Inflation starts when enemy within (3.5 blocks vanilla).
pub const INFLATE_RANGE: f64 = 3.5;

impl Pufferfish {
    pub fn new() -> Self {
        Self {
            state: PufferfishState::Small,
            inflate_cooldown: 0,
        }
    }

    pub fn update_state(&mut self, enemy_nearby: bool) {
        if enemy_nearby {
            self.state = match self.state {
                PufferfishState::Small => PufferfishState::Medium,
                _ => PufferfishState::Full,
            };
            self.inflate_cooldown = 40;
        } else if self.inflate_cooldown > 0 {
            self.inflate_cooldown -= 1;
        } else {
            self.state = match self.state {
                PufferfishState::Full => PufferfishState::Medium,
                _ => PufferfishState::Small,
            };
        }
    }

    /// Damage when touched (state=Full = 2 poison).
    pub fn touch_damage(&self) -> f32 {
        match self.state {
            PufferfishState::Small => 0.0,
            PufferfishState::Medium => 1.0,
            PufferfishState::Full => 2.0,
        }
    }
}

impl Default for Pufferfish {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cod_drops_raw_cod() {
        assert_eq!(FishKind::Cod.raw_drop(), "minecraft:raw_cod");
    }

    #[test]
    fn pufferfish_inflates_near_enemy() {
        let mut p = Pufferfish::new();
        p.update_state(true);
        assert_ne!(p.state, PufferfishState::Small);
    }
}
