//! Player list tab menu.

#[derive(Debug, Clone)]
pub struct TabListEntry {
    pub uuid: String,
    pub name: String,
    pub display_name: Option<String>,
    pub xuid: Option<String>,
    pub ping: u32,
    pub gamemode: u8,
    pub skin_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct TabList {
    pub entries: Vec<TabListEntry>,
    pub header: String,
    pub footer: String,
}

impl TabList {
    pub fn add(&mut self, entry: TabListEntry) {
        self.entries.retain(|e| e.uuid != entry.uuid);
        self.entries.push(entry);
    }

    pub fn remove(&mut self, uuid: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.uuid != uuid);
        before != self.entries.len()
    }

    pub fn set_header_footer(&mut self, header: String, footer: String) {
        self.header = header;
        self.footer = footer;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove() {
        let mut t = TabList::default();
        t.add(TabListEntry {
            uuid: "abc".into(),
            name: "Steve".into(),
            display_name: None,
            xuid: None,
            ping: 30,
            gamemode: 0,
            skin_data: None,
        });
        assert_eq!(t.entries.len(), 1);
        assert!(t.remove("abc"));
    }
}
