//! Behavior pack manifest.json parsing.

use serde::Deserialize;

/// Top-level manifest.json structure.
#[derive(Debug, Clone, Deserialize)]
pub struct BehaviorPackManifest {
    /// Format version — can be integer (2) or string ("2").
    pub format_version: serde_json::Value,
    pub header: ManifestHeader,
    #[serde(default)]
    pub modules: Vec<ManifestModule>,
    #[serde(default)]
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestHeader {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub uuid: String,
    /// Semantic version as `[major, minor, patch]`.
    pub version: Vec<u32>,
    #[serde(default)]
    pub min_engine_version: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestModule {
    #[serde(rename = "type")]
    pub module_type: String,
    pub uuid: String,
    pub version: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestDependency {
    pub uuid: String,
    pub version: Vec<u32>,
}

impl BehaviorPackManifest {
    /// Parse a manifest from a JSON string.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid manifest.json: {e}"))
    }

    /// Version as a dot-separated string, e.g. "1.0.0".
    pub fn version_string(&self) -> String {
        self.header
            .version
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let json = r#"{
            "format_version": 2,
            "header": {
                "name": "Test Pack",
                "description": "A test behavior pack",
                "uuid": "12345678-1234-1234-1234-123456789012",
                "version": [1, 2, 3],
                "min_engine_version": [1, 21, 0]
            },
            "modules": [
                {
                    "type": "data",
                    "uuid": "87654321-4321-4321-4321-210987654321",
                    "version": [1, 0, 0]
                }
            ],
            "dependencies": []
        }"#;
        let m = BehaviorPackManifest::parse(json).unwrap();
        assert_eq!(m.header.name, "Test Pack");
        assert_eq!(m.header.uuid, "12345678-1234-1234-1234-123456789012");
        assert_eq!(m.version_string(), "1.2.3");
        assert_eq!(m.modules.len(), 1);
        assert_eq!(m.modules[0].module_type, "data");
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "format_version": "2",
            "header": {
                "name": "Minimal",
                "uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "version": [1, 0, 0]
            }
        }"#;
        let m = BehaviorPackManifest::parse(json).unwrap();
        assert_eq!(m.header.name, "Minimal");
        assert_eq!(m.header.description, "");
        assert!(m.modules.is_empty());
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn parse_invalid_json() {
        assert!(BehaviorPackManifest::parse("not json {{{").is_err());
        assert!(BehaviorPackManifest::parse(r#"{"format_version": 2}"#).is_err());
    }
}
