//! Sound categories — Bedrock audio buses.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Blocks,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
    UI,
    Dimensions,
}

impl SoundCategory {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Record => "record",
            Self::Weather => "weather",
            Self::Blocks => "block",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Player => "player",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
            Self::UI => "ui",
            Self::Dimensions => "dimension",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_identifier() {
        assert_eq!(SoundCategory::Master.identifier(), "master");
    }
}
