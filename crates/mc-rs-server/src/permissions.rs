//! Permission system.

use std::collections::HashSet;

/// Permission levels (vanilla + PMMP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpLevel {
    None = 0,
    Visitor = 1,
    Member = 2,
    Admin = 3,
    Operator = 4,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    pub op_level: u8,
    pub allowed: HashSet<String>,
    pub denied: HashSet<String>,
}

impl PermissionSet {
    pub fn new(op_level: u8) -> Self {
        Self {
            op_level,
            allowed: HashSet::new(),
            denied: HashSet::new(),
        }
    }

    pub fn has(&self, perm: &str) -> bool {
        if self.denied.contains(perm) {
            return false;
        }
        if self.allowed.contains(perm) {
            return true;
        }
        self.allowed.contains("*")
    }

    /// Built-in perms for commands.
    pub fn required_op_level(command: &str) -> u8 {
        match command {
            "/stop" | "/op" | "/deop" | "/whitelist" => 4,
            "/gamemode" | "/gamerule" | "/difficulty" => 2,
            "/tp" | "/teleport" | "/summon" | "/kick" | "/ban" => 2,
            "/time" | "/weather" => 2,
            "/give" | "/effect" | "/enchant" => 2,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_grants_all() {
        let mut p = PermissionSet::new(0);
        p.allowed.insert("*".to_string());
        assert!(p.has("anything"));
    }

    #[test]
    fn denied_beats_star() {
        let mut p = PermissionSet::new(0);
        p.allowed.insert("*".to_string());
        p.denied.insert("nope".to_string());
        assert!(!p.has("nope"));
    }
}
