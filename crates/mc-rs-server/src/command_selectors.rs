//! Target selectors (@p, @e, @a, @r, @s) + filters.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorKind {
    NearestPlayer, // @p
    AllEntities,   // @e
    AllPlayers,    // @a
    RandomPlayer,  // @r
    Self_,         // @s
    NearestEntity, // @n (1.19.4+)
}

impl SelectorKind {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "@p" => Some(Self::NearestPlayer),
            "@e" => Some(Self::AllEntities),
            "@a" => Some(Self::AllPlayers),
            "@r" => Some(Self::RandomPlayer),
            "@s" => Some(Self::Self_),
            "@n" => Some(Self::NearestEntity),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectorArgs {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub radius_max: Option<f64>,
    pub radius_min: Option<f64>,
    pub count: Option<u32>,
    pub type_filter: Option<String>,
    pub name_filter: Option<String>,
    pub tag_filter: Option<String>,
    pub team_filter: Option<String>,
    pub gamemode_filter: Option<u8>,
    pub level_min: Option<u32>,
    pub level_max: Option<u32>,
    pub sort: SelectorSort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectorSort {
    #[default]
    Nearest,
    Furthest,
    Random,
    Arbitrary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_selectors() {
        assert_eq!(
            SelectorKind::from_str("@p"),
            Some(SelectorKind::NearestPlayer)
        );
    }
}
