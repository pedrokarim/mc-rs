//! `SimPlayer` — un joueur simulé.
//!
//! Enchaîne le handshake RakNet (Phase 1), le login Bedrock chiffré (Phase 2),
//! le spawn jusqu'à InGame (Phase 3), puis un comportement en jeu réaliste
//! (mouvement + chat, Phase 4) qui génère la charge sur le serveur.

use std::f32::consts::TAU;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{debug, warn};

use crate::bedrock::{BedrockClient, BedrockError};
use crate::identity::SimIdentity;
use crate::net::RakClient;

/// Comportement en jeu du joueur simulé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    /// Reste immobile mais envoie PlayerAuthInput chaque tick (comme un vrai
    /// client au repos).
    Idle,
    /// Se déplace (heading qui dérive lentement) → chunk loading dynamique +
    /// broadcasts de mouvement.
    Wander,
    /// Wander + chat périodique → broadcasts de chat en plus.
    Active,
}

impl Behavior {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "idle" => Some(Self::Idle),
            "wander" => Some(Self::Wander),
            "active" => Some(Self::Active),
            _ => None,
        }
    }
}

/// Configuration d'un joueur simulé.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Adresse du serveur cible.
    pub server_addr: SocketAddr,
    /// Nom d'affichage du joueur simulé.
    pub name: String,
    /// MTU visée pour le handshake RakNet.
    pub mtu: u16,
    /// Comportement en jeu.
    pub behavior: Behavior,
}

impl SimConfig {
    pub fn new(server_addr: SocketAddr, name: impl Into<String>) -> Self {
        Self {
            server_addr,
            name: name.into(),
            mtu: 1400,
            behavior: Behavior::Wander,
        }
    }
}

/// Résultat de fin de vie d'un joueur simulé.
#[derive(Debug, Clone)]
pub struct SimOutcome {
    pub name: String,
    /// Durée du handshake RakNet (None si échec).
    pub handshake: Option<Duration>,
    /// Délai jusqu'au chiffrement vérifié (depuis le début du flux Bedrock).
    pub login: Option<Duration>,
    /// Délai jusqu'à l'état InGame (depuis le début du flux Bedrock).
    pub spawn: Option<Duration>,
    /// Nombre de LevelChunk reçus pendant la session.
    pub chunks: usize,
    /// Nombre de PlayerAuthInput envoyés.
    pub inputs_sent: u64,
    /// Nombre de messages de chat envoyés.
    pub chats_sent: u64,
    /// RTT observé en fin de session, si mesuré.
    pub last_rtt_ms: Option<f64>,
    /// Erreur éventuelle.
    pub error: Option<String>,
}

/// Vitesse de marche (blocs/seconde), proche de la valeur vanilla (~4.3).
const WALK_SPEED: f32 = 4.3;
/// Cadence d'envoi de PlayerAuthInput (20 TPS = 50 ms).
const INPUT_PERIOD: Duration = Duration::from_millis(50);

/// Un joueur simulé, propriétaire de sa connexion.
pub struct SimPlayer {
    cfg: SimConfig,
    client: BedrockClient,
    handshake: Duration,
}

impl SimPlayer {
    /// Se connecte au serveur : handshake RakNet, login chiffré, puis spawn
    /// jusqu'à l'état InGame.
    pub async fn connect(cfg: SimConfig) -> Result<Self, BedrockError> {
        let t0 = Instant::now();
        let rak = RakClient::open(cfg.server_addr, cfg.mtu).await?;
        let handshake = t0.elapsed();

        let identity = SimIdentity::generate(&cfg.name);
        let mut client = BedrockClient::new(rak, identity);
        client.join().await?;

        debug!(
            "[{}] InGame (rak {:?}, login {:?}, spawn {:?}, {} chunks)",
            cfg.name, handshake, client.encrypted_at, client.ingame_at, client.chunks_received
        );
        Ok(Self {
            cfg,
            client,
            handshake,
        })
    }

    /// Fait vivre le joueur pendant `duration` : maintient la session et joue
    /// le comportement configuré (mouvement + chat).
    pub async fn run(&mut self, duration: Duration) -> SimOutcome {
        let deadline = Instant::now() + duration;
        let mut error = None;

        // État de mouvement, ancré sur la position de spawn.
        let mut pos = self.client.position;
        let base_y = pos[1];
        // StdRng est Send (contrairement à ThreadRng), requis car la boucle
        // traverse des `.await` dans une tâche tokio multi-thread.
        let mut rng = StdRng::from_entropy();
        let mut heading: f32 = rng.gen_range(0.0..TAU);

        let mut last_input = Instant::now();
        let mut last_heading_change = Instant::now();
        let heading_period = Duration::from_millis(1500);
        // Chat périodique (seulement en Active), décalé par joueur.
        let mut next_chat = Instant::now() + Duration::from_secs(rng.gen_range(3..9));
        let mut inputs_sent = 0u64;
        let mut chats_sent = 0u64;

        while Instant::now() < deadline {
            if self.client.is_disconnected() {
                error = Some("déconnecté par le serveur".to_string());
                break;
            }
            if let Err(e) = self.client.pump(INPUT_PERIOD).await {
                warn!("[{}] erreur pump: {e}", self.cfg.name);
                error = Some(e.to_string());
                break;
            }

            // ── Envoi de PlayerAuthInput à cadence fixe ──
            let now = Instant::now();
            let dt = now.duration_since(last_input);
            if dt >= INPUT_PERIOD {
                last_input = now;
                let dt_s = dt.as_secs_f32();

                let (new_pos, move_vec, yaw) = match self.cfg.behavior {
                    Behavior::Idle => (pos, [0.0, 0.0], heading.to_degrees()),
                    Behavior::Wander | Behavior::Active => {
                        // Dérive lente du cap.
                        if now.duration_since(last_heading_change) >= heading_period {
                            last_heading_change = now;
                            heading += rng.gen_range(-0.6..0.6);
                        }
                        let step = WALK_SPEED * dt_s;
                        let np = [
                            pos[0] + heading.sin() * step,
                            base_y, // Y constant : évite fall damage / anti-fly
                            pos[2] + heading.cos() * step,
                        ];
                        (np, [0.0, 1.0], heading.to_degrees())
                    }
                };

                let delta = [
                    new_pos[0] - pos[0],
                    new_pos[1] - pos[1],
                    new_pos[2] - pos[2],
                ];
                pos = new_pos;
                self.client
                    .send_player_auth_input(pos, 0.0, yaw, yaw, move_vec, delta, 0);
                inputs_sent += 1;
            }

            // ── Chat périodique (Active) ──
            if self.cfg.behavior == Behavior::Active && Instant::now() >= next_chat {
                self.client
                    .send_chat(&format!("hello from {}", self.cfg.name));
                chats_sent += 1;
                next_chat = Instant::now() + Duration::from_secs(rng.gen_range(5..12));
            }
        }

        self.client.disconnect().await;

        SimOutcome {
            name: self.cfg.name.clone(),
            handshake: Some(self.handshake),
            login: self.client.encrypted_at,
            spawn: self.client.ingame_at,
            chunks: self.client.chunks_received,
            inputs_sent,
            chats_sent,
            last_rtt_ms: self.client.last_rtt_ms(),
            error,
        }
    }
}
