//! Handle instancié côté `mc-rs-server` et passé à [`crate::serve`].
//!
//! Contient tous les canaux d'I/O avec la main loop :
//! - `console_tx`    : envoie des lignes de commande à `dispatch_command_line`
//! - `snapshot`      : Arc<RwLock<...>> lu pour dashboards + APIs
//! - `event_tx`      : broadcast events serveur (PlayerJoin, Chat, etc.)
//! - `log_tx`        : broadcast lignes de log tracing (streaming console)

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::events::{LogLine, WebEvent};
use crate::snapshot::ServerSnapshot;

#[derive(Clone)]
pub struct WebUiHandle {
    pub console_tx: mpsc::UnboundedSender<String>,
    pub snapshot: Arc<RwLock<ServerSnapshot>>,
    pub event_tx: broadcast::Sender<WebEvent>,
    pub log_tx: broadcast::Sender<LogLine>,
}

impl WebUiHandle {
    pub fn new(
        console_tx: mpsc::UnboundedSender<String>,
        snapshot: Arc<RwLock<ServerSnapshot>>,
        event_tx: broadcast::Sender<WebEvent>,
        log_tx: broadcast::Sender<LogLine>,
    ) -> Self {
        Self {
            console_tx,
            snapshot,
            event_tx,
            log_tx,
        }
    }
}
