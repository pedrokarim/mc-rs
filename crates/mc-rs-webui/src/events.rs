//! Events serveur et lignes de log exposés aux clients WebSocket.

use serde::Serialize;

/// Event dispatché par le serveur vers le broadcast channel webui.
/// Variant simple pour V1 : extensible sans breaking change.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WebEvent {
    PlayerJoin {
        name: String,
        addr: String,
        xuid: String,
    },
    PlayerQuit {
        name: String,
        addr: String,
    },
    PlayerChat {
        name: String,
        message: String,
    },
    PlayerDeath {
        name: String,
        cause: String,
    },
    PlayerGamemodeChange {
        name: String,
        new_mode: i32,
    },
    ServerShutdown,
    /// Action admin effectuée via webui (loggée séparément en DB mais aussi
    /// broadcast pour visibilité live multi-admin).
    AdminAction {
        actor: String,
        action: String,
        detail: String,
    },
}

/// Une ligne de log tracing formatée, prête à streamer.
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    /// Timestamp local (HH:MM:SS.mmm).
    pub ts: String,
    /// Niveau : "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR".
    pub level: String,
    /// Nom du module émetteur.
    pub target: String,
    /// Message formaté sans timestamp ni niveau (le client les réaffiche).
    pub message: String,
}
