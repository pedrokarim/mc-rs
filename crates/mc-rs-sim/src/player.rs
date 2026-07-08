//! `SimPlayer` — un joueur simulé.
//!
//! Phase 1 : se connecte au niveau RakNet et maintient la session vivante.
//! Les phases suivantes (login/chiffrement, spawn, mouvement) viennent enrichir
//! ce type sans changer sa forme d'usage.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::net::{RakClient, RakError};

/// Configuration d'un joueur simulé.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Adresse du serveur cible.
    pub server_addr: SocketAddr,
    /// Nom d'affichage du joueur simulé.
    pub name: String,
    /// MTU visée pour le handshake RakNet.
    pub mtu: u16,
}

impl SimConfig {
    pub fn new(server_addr: SocketAddr, name: impl Into<String>) -> Self {
        Self {
            server_addr,
            name: name.into(),
            mtu: 1400,
        }
    }
}

/// Résultat de fin de vie d'un joueur simulé.
#[derive(Debug, Clone)]
pub struct SimOutcome {
    pub name: String,
    /// Durée du handshake RakNet (None si échec).
    pub handshake: Option<Duration>,
    /// RTT observé en fin de session, si mesuré.
    pub last_rtt_ms: Option<f64>,
    /// Erreur éventuelle.
    pub error: Option<String>,
}

/// Un joueur simulé, propriétaire de sa connexion.
pub struct SimPlayer {
    cfg: SimConfig,
    client: RakClient,
    handshake: Duration,
}

impl SimPlayer {
    /// Se connecte au serveur (handshake RakNet complet).
    pub async fn connect(cfg: SimConfig) -> Result<Self, RakError> {
        let t0 = Instant::now();
        let client = RakClient::open(cfg.server_addr, cfg.mtu).await?;
        let handshake = t0.elapsed();
        debug!("[{}] connecté (handshake {:?})", cfg.name, handshake);
        Ok(Self {
            cfg,
            client,
            handshake,
        })
    }

    /// Fait vivre le joueur pendant `duration` : maintient la session (ping,
    /// ACK/NACK) et draine les paquets de jeu reçus (ignorés en Phase 1).
    pub async fn run(&mut self, duration: Duration) -> SimOutcome {
        let deadline = Instant::now() + duration;
        let mut error = None;

        while Instant::now() < deadline {
            if self.client.is_disconnected() {
                error = Some("déconnecté par le serveur".to_string());
                break;
            }
            if let Err(e) = self.client.pump(Duration::from_millis(50)).await {
                warn!("[{}] erreur pump: {e}", self.cfg.name);
                error = Some(e.to_string());
                break;
            }
            // Draine les batches reçus (décodage protocole = phases suivantes).
            while self.client.next_incoming().is_some() {}
        }

        let _ = self.client.disconnect().await;

        SimOutcome {
            name: self.cfg.name.clone(),
            handshake: Some(self.handshake),
            last_rtt_ms: self.client.last_rtt_ms,
            error,
        }
    }
}
