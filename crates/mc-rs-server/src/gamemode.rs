//! Gamemode — port PMMP `src/player/GameMode.php` + vanilla mode rules.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl GameMode {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival,
        }
    }

    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }

    pub fn is_creative(&self) -> bool {
        matches!(self, Self::Creative)
    }

    pub fn is_spectator(&self) -> bool {
        matches!(self, Self::Spectator)
    }

    pub fn allows_flight(&self) -> bool {
        matches!(self, Self::Creative | Self::Spectator)
    }

    pub fn takes_damage(&self) -> bool {
        matches!(self, Self::Survival | Self::Adventure)
    }

    pub fn can_break_blocks(&self) -> bool {
        matches!(self, Self::Survival | Self::Creative) // Adventure needs can_break NBT
    }

    pub fn can_place_blocks(&self) -> bool {
        matches!(self, Self::Survival | Self::Creative) // Adventure needs can_place NBT
    }

    pub fn can_interact_entities(&self) -> bool {
        !matches!(self, Self::Spectator)
    }

    pub fn can_pickup_items(&self) -> bool {
        matches!(self, Self::Survival | Self::Creative | Self::Adventure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectator_no_damage() {
        assert!(!GameMode::Spectator.takes_damage());
    }

    #[test]
    fn adventure_no_break_by_default() {
        // Adventure peut break avec can_break NBT mais pas par défaut.
        assert!(!GameMode::Adventure.can_break_blocks());
    }

    #[test]
    fn creative_fly_allowed() {
        assert!(GameMode::Creative.allows_flight());
        assert!(!GameMode::Survival.allows_flight());
    }
}
