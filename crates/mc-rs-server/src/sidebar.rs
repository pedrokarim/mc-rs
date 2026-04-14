//! Sidebar scoreboard UI.

#[derive(Debug, Clone)]
pub struct Sidebar {
    pub title: String,
    pub entries: Vec<(String, i32)>, // (text, score)
    pub max_entries: usize,
}

impl Sidebar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            entries: Vec::new(),
            max_entries: 15,
        }
    }

    pub fn add_entry(&mut self, text: String, score: i32) {
        if self.entries.len() >= self.max_entries {
            return;
        }
        self.entries.push((text, score));
        self.entries.sort_by(|a, b| b.1.cmp(&a.1));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Scoreboard display slot positions.
    pub const SLOT_SIDEBAR: &'static str = "sidebar";
    pub const SLOT_LIST: &'static str = "list";
    pub const SLOT_BELOW_NAME: &'static str = "belowname";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_by_score() {
        let mut s = Sidebar::new("Test");
        s.add_entry("low".into(), 1);
        s.add_entry("high".into(), 10);
        assert_eq!(s.entries[0].0, "high");
    }
}
