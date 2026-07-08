//! `SimPlayer` — un joueur simulé.
//!
//! Enchaîne le handshake RakNet (Phase 1) et le login Bedrock chiffré
//! (Phase 2). Les phases suivantes (spawn, mouvement) enrichissent la boucle
//! `run` sans changer la forme d'usage.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::bedrock::{BedrockClient, BedrockError};
use crate::identity::SimIdentity;
use crate::net::RakClient;

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
    /// Durée du login Bedrock (RakNet connecté → chiffrement vérifié).
    pub login: Option<Duration>,
    /// RTT observé en fin de session, si mesuré.
    pub last_rtt_ms: Option<f64>,
    /// Erreur éventuelle.
    pub error: Option<String>,
}

/// Un joueur simulé, propriétaire de sa connexion.
pub struct SimPlayer {
    cfg: SimConfig,
    client: BedrockClient,
    handshake: Duration,
    login: Duration,
}

impl SimPlayer {
    /// Se connecte au serveur : handshake RakNet puis login Bedrock chiffré.
    pub async fn connect(cfg: SimConfig) -> Result<Self, BedrockError> {
        let t0 = Instant::now();
        let rak = RakClient::open(cfg.server_addr, cfg.mtu).await?;
        let handshake = t0.elapsed();

        let identity = SimIdentity::generate(&cfg.name);
        let mut client = BedrockClient::new(rak, identity);

        let t1 = Instant::now();
        client.login().await?;
        let login = t1.elapsed();

        debug!(
            "[{}] connecté + login chiffré (rak {:?}, login {:?})",
            cfg.name, handshake, login
        );
        Ok(Self {
            cfg,
            client,
            handshake,
            login,
        })
    }

    /// Fait vivre le joueur pendant `duration` : maintient la session (ping,
    /// ACK/NACK, déchiffrement) et draine les paquets reçus.
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
        }

        self.client.disconnect().await;

        SimOutcome {
            name: self.cfg.name.clone(),
            handshake: Some(self.handshake),
            login: Some(self.login),
            last_rtt_ms: self.client.last_rtt_ms(),
            error,
        }
    }
}
