//! Entity tag system — /tag command + scoreboard.

use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct TagSet {
    pub tags: HashSet<String>,
}

impl TagSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, tag: &str) -> bool {
        self.tags.insert(tag.to_string())
    }

    pub fn remove(&mut self, tag: &str) -> bool {
        self.tags.remove(tag)
    }

    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    pub fn list(&self) -> Vec<&str> {
        self.tags.iter().map(|s| s.as_str()).collect()
    }
}

/// Max tags per entity (1024 vanilla).
pub const MAX_TAGS: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove() {
        let mut t = TagSet::new();
        assert!(t.add("boss"));
        assert!(t.has("boss"));
        assert!(t.remove("boss"));
        assert!(!t.has("boss"));
    }

    #[test]
    fn duplicate_add_false() {
        let mut t = TagSet::new();
        t.add("x");
        assert!(!t.add("x"));
    }
}
