//! Amethyst — cluster growth + sound.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmethystStage {
    SmallBud,
    MediumBud,
    LargeBud,
    Cluster,
}

impl AmethystStage {
    pub fn next(&self) -> Option<Self> {
        Some(match self {
            Self::SmallBud => Self::MediumBud,
            Self::MediumBud => Self::LargeBud,
            Self::LargeBud => Self::Cluster,
            Self::Cluster => return None,
        })
    }

    /// Drops shards when fully grown (4 shards).
    pub fn drop_shards(&self) -> u32 {
        match self {
            Self::Cluster => 4,
            _ => 0,
        }
    }

    /// Growth chance per random tick (low).
    pub fn growth_chance() -> f32 {
        0.05
    }

    /// Growth requires adjacent budding amethyst.
    pub fn block_id(&self) -> u16 {
        match self {
            Self::SmallBud => 725,
            Self::MediumBud => 726,
            Self::LargeBud => 727,
            Self::Cluster => 728,
        }
    }
}

/// Amethyst block resonance — used for allay duplication.
pub const RESONANCE_RANGE: f64 = 16.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_is_last() {
        assert!(AmethystStage::Cluster.next().is_none());
    }

    #[test]
    fn cluster_drops_shards() {
        assert_eq!(AmethystStage::Cluster.drop_shards(), 4);
    }
}
