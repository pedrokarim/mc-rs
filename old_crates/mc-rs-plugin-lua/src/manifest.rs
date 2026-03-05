//! Lua plugin manifest parsing (plugin.toml).

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RawManifest {
    pub plugin: PluginSection,
    #[serde(default)]
    pub limits: LimitsSection,
}

#[derive(Debug, Deserialize)]
pub struct PluginSection {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_main")]
    pub main: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_main() -> String {
    "main.lua".to_string()
}

#[derive(Debug, Deserialize)]
pub struct LimitsSection {
    #[serde(default = "default_memory_mb")]
    pub memory_mb: usize,
    #[serde(default = "default_instruction_limit")]
    pub instruction_limit: u32,
}

impl Default for LimitsSection {
    fn default() -> Self {
        Self {
            memory_mb: default_memory_mb(),
            instruction_limit: default_instruction_limit(),
        }
    }
}

fn default_memory_mb() -> usize {
    16
}

fn default_instruction_limit() -> u32 {
    1_000_000
}

/// Parsed Lua plugin manifest.
pub struct LuaPluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub main: String,
    pub memory_mb: usize,
    pub instruction_limit: u32,
}

impl LuaPluginManifest {
    pub fn parse(toml_content: &str) -> Result<Self, String> {
        let raw: RawManifest =
            toml::from_str(toml_content).map_err(|e| format!("invalid plugin.toml: {e}"))?;
        Ok(Self {
            name: raw.plugin.name,
            version: raw.plugin.version,
            author: raw.plugin.author,
            description: raw.plugin.description,
            main: raw.plugin.main,
            memory_mb: raw.limits.memory_mb,
            instruction_limit: raw.limits.instruction_limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[plugin]
name = "TestPlugin"
version = "2.0.0"
author = "Dev"
description = "A test"
main = "init.lua"

[limits]
memory_mb = 32
instruction_limit = 500000
"#;
        let m = LuaPluginManifest::parse(toml).unwrap();
        assert_eq!(m.name, "TestPlugin");
        assert_eq!(m.version, "2.0.0");
        assert_eq!(m.author, "Dev");
        assert_eq!(m.main, "init.lua");
        assert_eq!(m.memory_mb, 32);
        assert_eq!(m.instruction_limit, 500_000);
    }

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[plugin]
name = "Minimal"
"#;
        let m = LuaPluginManifest::parse(toml).unwrap();
        assert_eq!(m.name, "Minimal");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.main, "main.lua");
        assert_eq!(m.memory_mb, 16);
        assert_eq!(m.instruction_limit, 1_000_000);
    }

    #[test]
    fn parse_invalid_manifest() {
        assert!(LuaPluginManifest::parse("not valid toml {{{").is_err());
        assert!(LuaPluginManifest::parse("[plugin]\n").is_err()); // missing name
    }
}
