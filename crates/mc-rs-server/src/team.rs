//! Team system — scoreboard teams.

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Team {
    pub name: String,
    pub display_name: String,
    pub color: u8,
    pub prefix: String,
    pub suffix: String,
    pub allow_friendly_fire: bool,
    pub see_friendly_invisibles: bool,
    pub members: HashSet<String>,
    pub death_message_visibility: Visibility,
    pub collision_rule: CollisionRule,
    pub name_tag_visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Always,
    Never,
    HideForOtherTeams,
    HideForOwnTeam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionRule {
    Always,
    Never,
    PushOtherTeams,
    PushOwnTeam,
}

impl Team {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: String::new(),
            color: 15,
            prefix: String::new(),
            suffix: String::new(),
            allow_friendly_fire: true,
            see_friendly_invisibles: false,
            members: HashSet::new(),
            death_message_visibility: Visibility::Always,
            collision_rule: CollisionRule::Always,
            name_tag_visibility: Visibility::Always,
        }
    }

    pub fn add_member(&mut self, name: String) -> bool {
        self.members.insert(name)
    }

    pub fn remove_member(&mut self, name: &str) -> bool {
        self.members.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_member() {
        let mut t = Team::new("red");
        assert!(t.add_member("Steve".to_string()));
    }
}
