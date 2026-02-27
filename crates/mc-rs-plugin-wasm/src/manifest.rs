//! WASM plugin manifest (plugin.toml) parsing.

use serde::Deserialize;

/// Parsed WASM plugin manifest.
#[derive(Debug, Clone)]
pub struct WasmPluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub wasm_file: String,
    pub fuel_per_event: u64,
    pub fuel_per_command: u64,
    pub fuel_per_task: u64,
    pub fuel_on_enable: u64,
    pub max_memory_pages: u64,
}

#[derive(Deserialize)]
struct PluginToml {
    plugin: PluginSection,
    #[serde(default)]
    limits: LimitsSection,
}

#[derive(Deserialize)]
struct PluginSection {
    name: String,
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    wasm_file: String,
}

#[derive(Deserialize)]
struct LimitsSection {
    #[serde(default = "default_fuel_event")]
    fuel_per_event: u64,
    #[serde(default = "default_fuel_command")]
    fuel_per_command: u64,
    #[serde(default = "default_fuel_task")]
    fuel_per_task: u64,
    #[serde(default = "default_fuel_enable")]
    fuel_on_enable: u64,
    #[serde(default = "default_memory_pages")]
    max_memory_pages: u64,
}

// Default functions for serde
fn default_fuel_event() -> u64 {
    1_000_000
}
fn default_fuel_command() -> u64 {
    1_000_000
}
fn default_fuel_task() -> u64 {
    500_000
}
fn default_fuel_enable() -> u64 {
    5_000_000
}
fn default_memory_pages() -> u64 {
    256
} // 16 MB

// Default impl for LimitsSection (all defaults)
impl Default for LimitsSection {
    fn default() -> Self {
        Self {
            fuel_per_event: default_fuel_event(),
            fuel_per_command: default_fuel_command(),
            fuel_per_task: default_fuel_task(),
            fuel_on_enable: default_fuel_enable(),
            max_memory_pages: default_memory_pages(),
        }
    }
}

impl WasmPluginManifest {
    /// Parse a plugin.toml file content into a WasmPluginManifest.
    pub fn parse(toml_content: &str) -> Result<Self, toml::de::Error> {
        let parsed: PluginToml = toml::from_str(toml_content)?;
        Ok(Self {
            name: parsed.plugin.name,
            version: parsed.plugin.version,
            description: parsed.plugin.description,
            author: parsed.plugin.author,
            wasm_file: parsed.plugin.wasm_file,
            fuel_per_event: parsed.limits.fuel_per_event,
            fuel_per_command: parsed.limits.fuel_per_command,
            fuel_per_task: parsed.limits.fuel_per_task,
            fuel_on_enable: parsed.limits.fuel_on_enable,
            max_memory_pages: parsed.limits.max_memory_pages,
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
name = "test-plugin"
version = "1.0.0"
description = "A test plugin"
author = "TestDev"
wasm_file = "test.wasm"

[limits]
fuel_per_event = 2000000
fuel_per_command = 3000000
fuel_per_task = 1000000
fuel_on_enable = 10000000
max_memory_pages = 512
"#;
        let m = WasmPluginManifest::parse(toml).unwrap();
        assert_eq!(m.name, "test-plugin");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.description, "A test plugin");
        assert_eq!(m.author, "TestDev");
        assert_eq!(m.wasm_file, "test.wasm");
        assert_eq!(m.fuel_per_event, 2_000_000);
        assert_eq!(m.fuel_per_command, 3_000_000);
        assert_eq!(m.fuel_per_task, 1_000_000);
        assert_eq!(m.fuel_on_enable, 10_000_000);
        assert_eq!(m.max_memory_pages, 512);
    }

    #[test]
    fn parse_minimal_manifest_uses_defaults() {
        let toml = r#"
[plugin]
name = "minimal"
version = "0.1.0"
wasm_file = "plugin.wasm"
"#;
        let m = WasmPluginManifest::parse(toml).unwrap();
        assert_eq!(m.name, "minimal");
        assert_eq!(m.description, "");
        assert_eq!(m.author, "");
        assert_eq!(m.fuel_per_event, 1_000_000);
        assert_eq!(m.fuel_per_command, 1_000_000);
        assert_eq!(m.fuel_per_task, 500_000);
        assert_eq!(m.fuel_on_enable, 5_000_000);
        assert_eq!(m.max_memory_pages, 256);
    }

    #[test]
    fn parse_missing_name_fails() {
        let toml = r#"
[plugin]
version = "1.0.0"
wasm_file = "plugin.wasm"
"#;
        assert!(WasmPluginManifest::parse(toml).is_err());
    }

    #[test]
    fn parse_missing_wasm_file_fails() {
        let toml = r#"
[plugin]
name = "test"
version = "1.0.0"
"#;
        assert!(WasmPluginManifest::parse(toml).is_err());
    }

    #[test]
    fn parse_partial_limits() {
        let toml = r#"
[plugin]
name = "partial"
version = "1.0.0"
wasm_file = "p.wasm"

[limits]
fuel_per_event = 999
"#;
        let m = WasmPluginManifest::parse(toml).unwrap();
        assert_eq!(m.fuel_per_event, 999);
        assert_eq!(m.fuel_per_command, 1_000_000); // default
        assert_eq!(m.max_memory_pages, 256); // default
    }
}
