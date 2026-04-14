//! Player sprint / sneak / crouch state.

/// Sprint speed multiplier (1.3x).
pub const SPRINT_MULTIPLIER: f64 = 1.3;
/// Sneak speed multiplier (0.3x).
pub const SNEAK_MULTIPLIER: f64 = 0.3;
/// Swim speed (creative/swim mode).
pub const SWIM_MULTIPLIER: f64 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStance {
    Standing,
    Sneaking,
    Sprinting,
    Swimming,
    Sleeping,
    Gliding,
    Crawling,
    Riding,
}

impl PlayerStance {
    /// Collision box height.
    pub fn height(&self) -> f32 {
        match self {
            Self::Standing | Self::Sprinting => 1.8,
            Self::Sneaking => 1.5,
            Self::Swimming | Self::Gliding | Self::Crawling => 0.6,
            Self::Sleeping => 0.2,
            Self::Riding => 1.8,
        }
    }

    /// Can open containers?
    pub fn can_interact(&self) -> bool {
        !matches!(self, Self::Swimming | Self::Gliding | Self::Sleeping)
    }
}

/// Sprinting needs hunger > 6.
pub const SPRINT_MIN_HUNGER: u8 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sneaking_shorter() {
        assert!(PlayerStance::Sneaking.height() < PlayerStance::Standing.height());
    }
}
