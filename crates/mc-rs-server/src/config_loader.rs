//! Config loading (server.toml/properties).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub properties: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key)?.parse().ok()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            "true" | "yes" | "1" | "on" => Some(true),
            "false" | "no" | "0" | "off" => Some(false),
            _ => None,
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(key.into(), value.into());
    }

    /// Parse a simple KEY=VALUE format (server.properties).
    pub fn parse_properties(&mut self, text: &str) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                self.set(k.trim(), v.trim());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_props() {
        let mut c = Config::new();
        c.parse_properties("port=19132\n# comment\nwhite-list=true\n");
        assert_eq!(c.get("port"), Some("19132"));
        assert_eq!(c.get_bool("white-list"), Some(true));
    }
}
