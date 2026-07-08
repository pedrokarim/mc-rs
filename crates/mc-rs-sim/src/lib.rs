//! `mc-rs-sim` — simulation de population de joueurs pour un serveur Minecraft
//! Bedrock écrit en Rust (mc-rs).
//!
//! Simule des joueurs réels qui se connectent au serveur (handshake RakNet,
//! login offline auto-signé, chiffrement, spawn, mouvement) afin d'observer le
//! comportement du serveur sous une population croissante.
//!
//! ## Cible
//!
//! Conçu pour pointer vers **ton propre serveur** (localhost / ton IP). Ne pas
//! utiliser contre une infrastructure tierce.

pub mod bedrock;
pub mod identity;
pub mod metrics;
pub mod net;
pub mod player;
pub mod probe;

pub use bedrock::{BedrockClient, BedrockError, LoginState};
pub use identity::SimIdentity;
pub use net::{RakClient, RakError};
pub use player::{SimConfig, SimOutcome, SimPlayer};
