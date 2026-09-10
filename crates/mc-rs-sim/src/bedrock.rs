//! Client protocole Bedrock au-dessus de `RakClient`.
//!
//! Gère le cadrage (batch + compression + chiffrement) exactement comme le
//! serveur mc-rs (`connection/mod.rs`), et pilote la séquence de login :
//!
//! ```text
//! RequestNetworkSettings → NetworkSettings → Login →
//! ServerToClientHandshake → ClientToServerHandshake (chiffré) → …
//! ```
//!
//! Le chiffrement (AES-256-CTR fakeGCM) et la compression (zlib/none) sont
//! activés aux mêmes instants que côté serveur, sinon les compteurs et le
//! cadrage se désynchronisent.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use tracing::{debug, trace};

use mc_rs_crypto::encrypt::EncryptionContext;
use mc_rs_crypto::{ecdh, jwt};
use mc_rs_proto::batch::{self, CompressionAlgorithm};
use mc_rs_proto::codec;
use mc_rs_proto::io::{ProtoReader, ProtoWriter};
use mc_rs_proto::packets::packet_id;

use crate::identity::SimIdentity;
use crate::net::{RakClient, RakError};

/// Version protocole Bedrock cible : celle du serveur, jamais recopiée en dur.
pub const PROTOCOL_VERSION: i32 = mc_rs_proto::CURRENT_PROTOCOL;

/// Étape de la séquence de connexion Bedrock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    /// RequestNetworkSettings envoyé, on attend NetworkSettings.
    NetworkSettings,
    /// Login envoyé, on attend ServerToClientHandshake.
    Handshake,
    /// Chiffrement activé : ClientToServerHandshake envoyé, on attend la suite
    /// (PlayStatus / ResourcePacksInfo). Atteindre cette étape = chiffrement
    /// bidirectionnel vérifié.
    Encrypted,
    /// Séquence de packs de ressources (Phase 3).
    ResourcePacks,
    /// En jeu (Phase 3).
    InGame,
}

/// Erreurs de la couche Bedrock.
#[derive(Debug, thiserror::Error)]
pub enum BedrockError {
    #[error(transparent)]
    Rak(#[from] RakError),
    #[error("timeout à l'étape {0:?}")]
    Timeout(LoginState),
    #[error("handshake serveur invalide: {0}")]
    BadHandshake(String),
    #[error("déconnecté par le serveur")]
    Disconnected,
}

/// Client Bedrock complet (RakNet + protocole).
pub struct BedrockClient {
    rak: RakClient,
    pub identity: SimIdentity,

    compression_enabled: bool,
    encryption: Option<EncryptionContext>,

    pub state: LoginState,
    /// Vrai dès qu'un paquet chiffré du serveur a été déchiffré avec succès.
    pub encryption_verified: bool,

    // ── État de jeu capté depuis StartGame (utile Phase 4) ──
    /// Runtime ID de notre entité joueur (pour PlayerAuthInput).
    pub runtime_id: u64,
    /// Position de spawn reçue.
    pub position: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    /// Nombre de LevelChunk reçus (diagnostic de charge).
    pub chunks_received: usize,
    /// Compteur de game-tick pour PlayerAuthInput.
    game_tick: u64,

    // ── Chronométrage du flux de connexion ──
    flow_start: Option<Instant>,
    /// Délai jusqu'au chiffrement vérifié.
    pub encrypted_at: Option<Duration>,
    /// Délai jusqu'à l'état InGame.
    pub ingame_at: Option<Duration>,
}

impl BedrockClient {
    /// Construit le client sur une session RakNet déjà connectée.
    pub fn new(rak: RakClient, identity: SimIdentity) -> Self {
        Self {
            rak,
            identity,
            compression_enabled: false,
            encryption: None,
            state: LoginState::NetworkSettings,
            encryption_verified: false,
            runtime_id: 0,
            position: [0.0; 3],
            pitch: 0.0,
            yaw: 0.0,
            chunks_received: 0,
            game_tick: 0,
            flow_start: None,
            encrypted_at: None,
            ingame_at: None,
        }
    }

    /// Vrai une fois l'état InGame atteint.
    pub fn is_in_game(&self) -> bool {
        self.state == LoginState::InGame
    }

    /// Adresse locale de la session (diagnostic).
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.rak.local_addr()
    }

    pub fn is_disconnected(&self) -> bool {
        self.rak.is_disconnected()
    }

    pub fn last_rtt_ms(&self) -> Option<f64> {
        self.rak.last_rtt_ms
    }

    // ── Cadrage : envoi ──

    /// Encode et envoie un paquet de jeu, avec le cadrage adapté à l'étape
    /// courante (compression + chiffrement).
    pub fn send(&mut self, pkt_id: u32, payload: &[u8]) {
        let pkt = codec::encode_packet(pkt_id, payload);

        // Corps du batch : avec octet d'algo une fois la compression négociée,
        // sans avant (comme le serveur en état SessionStart).
        let inner = if self.compression_enabled {
            // On envoie non compressé (algo None = 0xFF) : le serveur lit
            // l'octet d'algo et décode sans compression. Évite de zlib côté bot.
            batch::encode_batch(&[pkt], CompressionAlgorithm::None, 0)
        } else {
            let mut w = ProtoWriter::with_capacity(pkt.len() + 5);
            w.write_var_u32(pkt.len() as u32);
            w.write_raw(&pkt);
            w.into_bytes()
        };

        let body = if let Some(ctx) = self.encryption.as_mut() {
            ctx.encrypt(&inner)
        } else {
            inner
        };

        let mut framed = Vec::with_capacity(1 + body.len());
        framed.push(0xFE);
        framed.extend_from_slice(&body);
        self.rak.send_game_batch(framed);
    }

    // ── Cadrage : réception ──

    /// Décode un batch brut reçu (`0xFE …`) en liste de (packet_id, payload).
    fn decode_incoming(&mut self, raw: &[u8]) -> Vec<(u32, Vec<u8>)> {
        if raw.is_empty() || raw[0] != 0xFE {
            return Vec::new();
        }
        let payload = &raw[1..];

        let decrypted = if let Some(ctx) = self.encryption.as_mut() {
            match ctx.decrypt(payload) {
                Ok(d) => {
                    if !self.encryption_verified {
                        self.encryption_verified = true;
                        self.encrypted_at = self.flow_start.map(|s| s.elapsed());
                    }
                    d
                }
                Err(e) => {
                    trace!("échec déchiffrement: {e}");
                    return Vec::new();
                }
            }
        } else {
            payload.to_vec()
        };

        if decrypted.is_empty() {
            return Vec::new();
        }

        let (algo, data) = if !self.compression_enabled {
            (CompressionAlgorithm::None, &decrypted[..])
        } else {
            let algo =
                CompressionAlgorithm::from_u8(decrypted[0]).unwrap_or(CompressionAlgorithm::None);
            (algo, &decrypted[1..])
        };

        let packets = match batch::decode_batch(data, algo) {
            Ok(p) => p,
            Err(e) => {
                trace!("décodage batch échoué: {e}");
                return Vec::new();
            }
        };

        let mut out = Vec::with_capacity(packets.len());
        for pkt in packets {
            let mut reader = ProtoReader::new(&pkt);
            if let Ok((id, _, _)) = codec::decode_packet_header(&mut reader) {
                out.push((id, reader.read_remaining()));
            }
        }
        out
    }

    // ── Boucle ──

    /// Une itération : E/S RakNet + traitement des paquets reçus.
    pub async fn pump(&mut self, max_wait: Duration) -> Result<(), BedrockError> {
        self.rak.pump(max_wait).await?;
        while let Some(raw) = self.rak.next_incoming() {
            let packets = self.decode_incoming(&raw);
            for (id, payload) in packets {
                self.handle_packet(id, &payload);
            }
        }
        Ok(())
    }

    /// Traite un paquet de jeu reçu (machine à états login → spawn).
    fn handle_packet(&mut self, pkt_id: u32, payload: &[u8]) {
        match pkt_id {
            packet_id::NETWORK_SETTINGS => {
                // La compression est désormais active pour toute la suite.
                self.compression_enabled = true;
                self.send_login();
                self.state = LoginState::Handshake;
                debug!("→ Login envoyé (compression activée)");
            }
            packet_id::SERVER_TO_CLIENT_HANDSHAKE => {
                if let Err(e) = self.on_server_handshake(payload) {
                    debug!("handshake échoué: {e}");
                }
            }
            packet_id::PLAY_STATUS => {
                // i32_be status : 0=LoginSuccess, 3=PlayerSpawn.
                let status = if payload.len() >= 4 {
                    i32::from_be_bytes(payload[0..4].try_into().unwrap())
                } else {
                    -1
                };
                match status {
                    0 => trace!("PlayStatus(LoginSuccess) — chiffrement OK"),
                    3 => {
                        // PlayerSpawn : on signale qu'on est prêt → InGame.
                        self.send_set_local_player_initialized();
                        self.state = LoginState::InGame;
                        self.ingame_at = self.flow_start.map(|s| s.elapsed());
                        debug!("→ InGame (SetLocalPlayerAsInitialized envoyé)");
                    }
                    other => trace!("PlayStatus({other})"),
                }
            }
            packet_id::RESOURCE_PACKS_INFO => {
                if self.state == LoginState::Encrypted {
                    self.state = LoginState::ResourcePacks;
                }
                // HAVE_ALL_PACKS (on ne télécharge aucun pack).
                self.send_resource_pack_response(3);
                trace!("ResourcePacksInfo → réponse HAVE_ALL_PACKS");
            }
            packet_id::RESOURCE_PACK_STACK => {
                // COMPLETED → le serveur enchaîne sur PreSpawn (StartGame…).
                self.send_resource_pack_response(4);
                trace!("ResourcePackStack → réponse COMPLETED");
            }
            packet_id::START_GAME => {
                self.parse_start_game(payload);
                // Le client demande son rayon de chunks après StartGame.
                self.send_request_chunk_radius(self.rak_view_distance());
                debug!(
                    "StartGame reçu (runtime_id={}, pos={:?}) → RequestChunkRadius",
                    self.runtime_id, self.position
                );
            }
            packet_id::LEVEL_CHUNK => {
                self.chunks_received += 1;
            }
            other => {
                trace!("paquet 0x{other:03X} ignoré (état {:?})", self.state);
            }
        }
    }

    /// Rayon de chunks demandé par le bot (petit par défaut : on ne rend rien).
    fn rak_view_distance(&self) -> i32 {
        4
    }

    /// Parse les champs de tête de StartGame (runtime_id + position).
    fn parse_start_game(&mut self, payload: &[u8]) {
        let mut r = ProtoReader::new(payload);
        let _unique = r.read_var_i64();
        if let Ok(rid) = r.read_var_u64() {
            self.runtime_id = rid;
        }
        let _gamemode = r.read_var_i32();
        if let (Ok(x), Ok(y), Ok(z)) = (r.read_f32_le(), r.read_f32_le(), r.read_f32_le()) {
            self.position = [x, y, z];
        }
        if let (Ok(p), Ok(yw)) = (r.read_f32_le(), r.read_f32_le()) {
            self.pitch = p;
            self.yaw = yw;
        }
    }

    /// Envoie le paquet Login (chaîne self-signed + client data).
    fn send_login(&mut self) {
        let auth = self.identity.auth_info_json();
        let client_data = self.identity.client_data_jwt();
        let payload = encode_login(PROTOCOL_VERSION, &auth, &client_data);
        self.send(packet_id::LOGIN, &payload);
    }

    /// Traite ServerToClientHandshake : dérive la clé AES, active le chiffrement,
    /// répond ClientToServerHandshake (chiffré).
    fn on_server_handshake(&mut self, payload: &[u8]) -> Result<(), BedrockError> {
        let mut reader = ProtoReader::new(payload);
        let jwt_str = reader
            .read_string()
            .map_err(|_| BedrockError::BadHandshake("lecture JWT".into()))?;

        let decoded = jwt::decode_jwt(&jwt_str)
            .map_err(|e| BedrockError::BadHandshake(format!("decode JWT: {e}")))?;

        let server_pub_b64 = decoded.header["x5u"]
            .as_str()
            .ok_or_else(|| BedrockError::BadHandshake("x5u manquant".into()))?;
        let salt_b64 = decoded.claims["salt"]
            .as_str()
            .ok_or_else(|| BedrockError::BadHandshake("salt manquant".into()))?;

        let server_pub = ecdh::parse_client_public_key(server_pub_b64)
            .map_err(|e| BedrockError::BadHandshake(format!("clé serveur: {e}")))?;
        let salt = STANDARD
            .decode(salt_b64)
            .or_else(|_| URL_SAFE_NO_PAD.decode(salt_b64))
            .map_err(|e| BedrockError::BadHandshake(format!("salt b64: {e}")))?;

        // Clé AES = SHA-256(salt + ECDH(clé_bot, clé_serveur)). Symétrique avec
        // le calcul serveur.
        let aes_key = self.identity.keypair.derive_aes_key(&server_pub, &salt);

        // Active le chiffrement AVANT d'envoyer ClientToServerHandshake : ce
        // paquet est le premier chiffré (compteur 0), comme le serveur l'attend.
        self.encryption = Some(EncryptionContext::new(aes_key));
        self.send(packet_id::CLIENT_TO_SERVER_HANDSHAKE, &[]);
        self.state = LoginState::Encrypted;
        debug!("→ ClientToServerHandshake envoyé (chiffrement activé)");
        Ok(())
    }

    /// Envoie ResourcePackClientResponse avec le `status` donné, 0 packs.
    /// (3 = HAVE_ALL_PACKS, 4 = COMPLETED.)
    fn send_resource_pack_response(&mut self, status: u8) {
        let mut w = ProtoWriter::with_capacity(4);
        w.write_u8(status);
        w.write_u16_le(0); // aucun pack demandé
        self.send(packet_id::RESOURCE_PACK_CLIENT_RESPONSE, &w.into_bytes());
    }

    /// Envoie RequestChunkRadius.
    fn send_request_chunk_radius(&mut self, radius: i32) {
        let mut w = ProtoWriter::with_capacity(6);
        w.write_var_i32(radius);
        w.write_u8(radius.clamp(0, 255) as u8); // max_radius (ignoré par le serveur)
        self.send(packet_id::REQUEST_CHUNK_RADIUS, &w.into_bytes());
    }

    /// Envoie SetLocalPlayerAsInitialized (var_u64 runtime_id).
    fn send_set_local_player_initialized(&mut self) {
        let mut w = ProtoWriter::with_capacity(9);
        w.write_var_u64(self.runtime_id);
        self.send(packet_id::SET_LOCAL_PLAYER_AS_INITIALIZED, &w.into_bytes());
    }

    // ── Actions en jeu (Phase 4) ──

    /// Envoie un PlayerAuthInput (mouvement/orientation). C'est le paquet le
    /// plus fréquent d'un vrai client (≈20/s) : il pilote le suivi de position,
    /// le chunk loading dynamique et les broadcasts de mouvement côté serveur.
    ///
    /// `delta` = déplacement depuis le tick précédent. Aucun flag d'action
    /// (bits 34/35/36) n'est posé, donc le paquet s'arrête après le delta.
    #[allow(clippy::too_many_arguments)]
    pub fn send_player_auth_input(
        &mut self,
        pos: [f32; 3],
        pitch: f32,
        yaw: f32,
        head_yaw: f32,
        move_vec: [f32; 2],
        delta: [f32; 3],
        input_flags: u128,
    ) {
        self.game_tick += 1;
        let mut w = ProtoWriter::with_capacity(80);
        w.write_f32_le(pitch);
        w.write_f32_le(yaw);
        w.write_f32_le(pos[0]);
        w.write_f32_le(pos[1]);
        w.write_f32_le(pos[2]);
        w.write_f32_le(move_vec[0]);
        w.write_f32_le(move_vec[1]);
        w.write_f32_le(head_yaw);
        write_bitset(&mut w, input_flags);
        w.write_var_u32(1); // input_mode = Mouse
        w.write_var_u32(0); // play_mode = Normal
        w.write_var_u32(0); // interaction_mode = Touch/None
        w.write_f32_le(0.0); // interact_rotation_x
        w.write_f32_le(0.0); // interact_rotation_y
        w.write_var_u64(self.game_tick);
        w.write_f32_le(delta[0]);
        w.write_f32_le(delta[1]);
        w.write_f32_le(delta[2]);
        self.send(packet_id::PLAYER_AUTH_INPUT, &w.into_bytes());
    }

    /// Envoie un message de chat (broadcast serveur à tous les joueurs).
    pub fn send_chat(&mut self, message: &str) {
        let payload = mc_rs_proto::packets::player::Text::chat(
            &self.identity.display_name,
            message,
            &self.identity.xuid,
        );
        self.send(packet_id::TEXT, &payload);
    }

    /// Amorce la séquence de connexion : RequestNetworkSettings.
    fn start_login(&mut self) {
        self.flow_start = Some(Instant::now());
        let mut proto = ProtoWriter::with_capacity(4);
        proto.write_i32_be(PROTOCOL_VERSION);
        self.send(packet_id::REQUEST_NETWORK_SETTINGS, &proto.into_bytes());
    }

    /// Pilote toute la séquence login → spawn jusqu'à l'état InGame.
    pub async fn join(&mut self) -> Result<(), BedrockError> {
        self.start_login();

        let deadline = Instant::now() + Duration::from_secs(20);
        while !self.is_in_game() {
            if self.rak.is_disconnected() {
                return Err(BedrockError::Disconnected);
            }
            if Instant::now() >= deadline {
                return Err(BedrockError::Timeout(self.state));
            }
            self.pump(Duration::from_millis(20)).await?;
        }
        Ok(())
    }

    /// Coupe proprement la session.
    pub async fn disconnect(&mut self) {
        self.rak.disconnect().await;
    }
}

/// Écrit un bitset u128 au format PlayerAuthInput (7 bits/octet, bit 7 =
/// continuation). Miroir de `read_bitset` côté proto. `0` → un seul octet.
fn write_bitset(w: &mut ProtoWriter, mut v: u128) {
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        w.write_u8(byte);
        if v == 0 {
            break;
        }
    }
}

/// Encode le paquet Login (payload uniquement, sans header de paquet).
///
/// Layout (miroir de `Login::decode`) :
/// `i32_be(protocol) + byte_array(payload)` où
/// `payload = i32_le(len(chain)) + chain + i32_le(len(client)) + client`.
pub fn encode_login(protocol: i32, chain_data: &str, client_data_jwt: &str) -> Vec<u8> {
    let mut payload = ProtoWriter::with_capacity(chain_data.len() + client_data_jwt.len() + 8);
    payload.write_i32_le(chain_data.len() as i32);
    payload.write_raw(chain_data.as_bytes());
    payload.write_i32_le(client_data_jwt.len() as i32);
    payload.write_raw(client_data_jwt.as_bytes());
    let payload = payload.into_bytes();

    let mut w = ProtoWriter::with_capacity(payload.len() + 8);
    w.write_i32_be(protocol);
    w.write_byte_array(&payload);
    w.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_proto::packets::login::Login;

    #[test]
    fn test_login_roundtrip_with_server_decoder() {
        let id = SimIdentity::generate("RoundTrip");
        let auth = id.auth_info_json();
        let client_data = id.client_data_jwt();
        let payload = encode_login(PROTOCOL_VERSION, &auth, &client_data);

        // Le paquet Login se décode via le décodeur du serveur.
        let mut reader = ProtoReader::new(&payload);
        let login = Login::decode(&mut reader).expect("decode login");
        assert_eq!(login.protocol_version, PROTOCOL_VERSION);
        assert_eq!(login.chain_data, auth);
        assert_eq!(login.client_data_jwt, client_data);

        // Et l'identité en ressort correctement.
        let identity = jwt::extract_login_identity(&login.chain_data).unwrap();
        assert_eq!(identity.display_name, "RoundTrip");
        assert!(!identity.authenticated);
    }
}
