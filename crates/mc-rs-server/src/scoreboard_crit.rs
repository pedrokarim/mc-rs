//! Scoreboard criteria.

#[derive(Debug, Clone)]
pub struct ScoreboardObjective {
    pub name: String,
    pub display_name: String,
    pub criterion: String,
}

/// Standard criteria.
pub fn standard_criteria() -> &'static [&'static str] {
    &[
        "dummy",
        "deathCount",
        "playerKillCount",
        "totalKillCount",
        "health",
        "xp",
        "level",
        "food",
        "air",
        "armor",
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveRenderType {
    Integer,
    Hearts,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_dummy_criterion() {
        assert!(standard_criteria().contains(&"dummy"));
    }
}
