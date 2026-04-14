//! Whitelist management.

use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct Whitelist {
    pub enabled: bool,
    pub entries: HashSet<String>,
}

impl Whitelist {
    pub fn add(&mut self, name: &str) -> bool {
        self.entries.insert(name.to_lowercase())
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.entries.remove(&name.to_lowercase())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains(&name.to_lowercase())
    }

    pub fn can_join(&self, name: &str, is_op: bool) -> bool {
        if !self.enabled {
            return true;
        }
        is_op || self.contains(name)
    }
}

/// Ban list — name + reason + expiry.
#[derive(Debug, Clone)]
pub struct BanEntry {
    pub name: String,
    pub reason: String,
    pub expires_at: Option<u64>, // Unix timestamp
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct BanList {
    pub bans: Vec<BanEntry>,
}

impl BanList {
    pub fn ban(&mut self, name: &str, reason: &str, expiry: Option<u64>) {
        self.unban(name);
        self.bans.push(BanEntry {
            name: name.to_string(),
            reason: reason.to_string(),
            expires_at: expiry,
            source: "Server".to_string(),
        });
    }

    pub fn unban(&mut self, name: &str) -> bool {
        let before = self.bans.len();
        self.bans.retain(|b| !b.name.eq_ignore_ascii_case(name));
        before != self.bans.len()
    }

    pub fn is_banned(&self, name: &str, now: u64) -> bool {
        self.bans.iter().any(|b|
            b.name.eq_ignore_ascii_case(name) && b.expires_at.map(|e| e > now).unwrap_or(true)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_bypass_whitelist() {
        let w = Whitelist::default();
        assert!(w.can_join("anyone", true));
    }

    #[test]
    fn unban_works() {
        let mut b = BanList::default();
        b.ban("Steve", "test", None);
        assert!(b.unban("steve"));
    }
}
