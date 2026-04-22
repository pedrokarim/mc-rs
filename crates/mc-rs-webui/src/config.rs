//! Section `[webui]` de `server.toml`.
//!
//! Défauts sûrs : `enabled = false`, `bind = 127.0.0.1:8080`, SQLite local.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebUiConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_bind")]
    pub bind: String,

    /// URL de connexion DB : `sqlite://...`, `postgres://...`, `mongodb://...`.
    /// Par défaut : fichier SQLite local dans le dossier du serveur.
    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_session_hours")]
    pub session_duration_hours: u64,

    #[serde(default)]
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub cert_path: String,
    #[serde(default)]
    pub key_path: String,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_database_url() -> String {
    "sqlite://webui.db".to_string()
}

fn default_session_hours() -> u64 {
    24
}

impl Default for WebUiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_bind(),
            database_url: default_database_url(),
            session_duration_hours: default_session_hours(),
            tls: TlsConfig::default(),
        }
    }
}
