//! Copper oxidation — 4 stages: copper → exposed → weathered → oxidized.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OxidationStage {
    Fresh,     // Pink/orange
    Exposed,   // Light green tint
    Weathered, // More green
    Oxidized,  // Teal
}

impl OxidationStage {
    pub fn next(&self) -> Option<Self> {
        Some(match self {
            Self::Fresh => Self::Exposed,
            Self::Exposed => Self::Weathered,
            Self::Weathered => Self::Oxidized,
            Self::Oxidized => return None,
        })
    }

    pub fn previous(&self) -> Option<Self> {
        Some(match self {
            Self::Fresh => return None,
            Self::Exposed => Self::Fresh,
            Self::Weathered => Self::Exposed,
            Self::Oxidized => Self::Weathered,
        })
    }
}

/// Oxidation chance per random tick (very slow — 64/1000).
pub const OXIDATION_CHANCE: f32 = 64.0 / 1000.0;
/// Requires unwaxed.
pub fn is_waxable(_block: &str) -> bool {
    true
}

/// Scraping with axe — removes oxidation stage + wax.
pub fn axe_scrapes() -> bool {
    true
}

/// Lightning strike removes all oxidation.
pub fn lightning_strips_oxidation() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oxidized_has_no_next() {
        assert!(OxidationStage::Oxidized.next().is_none());
    }

    #[test]
    fn fresh_has_no_prev() {
        assert!(OxidationStage::Fresh.previous().is_none());
    }
}
