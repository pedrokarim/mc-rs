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
                // « downloadingfinished » (ex-HAVE_ALL_PACKS) : on ne télécharge
                // aucun pack, le serveur répond par ResourcePackStack.
                self.send_resource_pack_response(PackResponse::DownloadingFinished);
                trace!("ResourcePacksInfo → réponse downloadingfinished");
            }
            packet_id::RESOURCE_PACK_STACK => {
                // « resourcepackstackfinished » (ex-COMPLETED) → le serveur
                // enchaîne sur PreSpawn (StartGame…).
                self.send_resource_pack_response(PackResponse::ResourcePackStackFinished);
                trace!("ResourcePackStack → réponse resourcepackstackfinished");
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

    /// Envoie ResourcePackClientResponse avec la réponse donnée (aucun pack
    /// demandé : le bot ne télécharge jamais rien).
    fn send_resource_pack_response(&mut self, response: PackResponse) {
        let payload = encode_resource_pack_client_response(response, &[]);
        self.send(packet_id::RESOURCE_PACK_CLIENT_RESPONSE, &payload);
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
    /// `delta` = déplacement depuis le tick précédent. `input_flags` = masque
    /// des flags PlayerAuthInput (bit n = flag n, n < 66). Aucune action
    /// (transaction, requête de stack, actions de bloc, véhicule) n'est jointe.
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
        let payload = encode_player_auth_input(&AuthInput {
            pos,
            pitch,
            yaw,
            head_yaw,
            move_vec,
            delta,
            input_flags,
            tick: self.game_tick,
        });
        self.send(packet_id::PLAYER_AUTH_INPUT, &payload);
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

/// Réponse du client à la séquence de packs de ressources (protocole 2169).
///
/// Variante étiquetée : l'index VarUInt32 vaut la valeur de l'énumération,
/// suivi du nom en minuscules. Réf. gophertunnel
/// `packet/resource_pack_client_response.go` (constantes `iota` depuis 0) et
/// Cloudburst `ResourcePackClientResponseSerializer_v2168` (`ordinal() - 1`).
/// Correspondance avec l'ancien u8 (1001) : REFUSED(1) → 0, SEND_PACKS(2) → 1,
/// HAVE_ALL_PACKS(3) → 2, COMPLETED(4) → 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackResponse {
    /// Refus des packs (ex-REFUSED).
    Cancel = 0,
    /// Demande de téléchargement des packs listés (ex-SEND_PACKS).
    Downloading = 1,
    /// Tous les packs sont présents (ex-HAVE_ALL_PACKS).
    DownloadingFinished = 2,
    /// Pile de packs appliquée, prêt pour StartGame (ex-COMPLETED).
    ResourcePackStackFinished = 3,
}

impl PackResponse {
    /// Nom transmis après l'index (énumération sans « Enum-as-Value » dans le
    /// schéma Mojang, donc sérialisée en chaîne).
    pub fn wire_name(self) -> &'static str {
        match self {
            PackResponse::Cancel => "cancel",
            PackResponse::Downloading => "downloading",
            PackResponse::DownloadingFinished => "downloadingfinished",
            PackResponse::ResourcePackStackFinished => "resourcepackstackfinished",
        }
    }
}

/// Encode ResourcePackClientResponse (payload seul, protocole 2169) :
/// `VarUInt32 index + string nom`, puis **uniquement** pour `Downloading` :
/// `VarUInt32 n + n × string` (`"<uuid>_<version>"`). `packs` est ignoré pour
/// les autres réponses, comme chez gophertunnel et Cloudburst.
pub fn encode_resource_pack_client_response(response: PackResponse, packs: &[String]) -> Vec<u8> {
    let name = response.wire_name();
    let mut w = ProtoWriter::with_capacity(name.len() + 4);
    w.write_var_u32(response as u32);
    w.write_string(name);
    if response == PackResponse::Downloading {
        w.write_var_u32(packs.len() as u32);
        for pack in packs {
            w.write_string(pack);
        }
    }
    w.into_bytes()
}

/// Nombre de flags PlayerAuthInput connus en 2169 (0..=65, `InternalUpdate`
/// = 65 étant le dernier). Réf. gophertunnel `InputFlagCount`.
pub const INPUT_FLAG_COUNT: u32 = 66;

/// Champs d'un PlayerAuthInput « mouvement seul » tel qu'émis par le bot.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthInput {
    /// Position rapportée par le client.
    pub pos: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub head_yaw: f32,
    /// Vecteur de déplacement (X/Z issus de ZQSD).
    pub move_vec: [f32; 2],
    /// Déplacement depuis le tick précédent.
    pub delta: [f32; 3],
    /// Masque des flags : bit n = flag n (seuls les bits < 66 sont émis).
    pub input_flags: u128,
    /// Tick client.
    pub tick: u64,
}

/// Encode un PlayerAuthInput (payload seul) au format 2169.
///
/// Ordre exact de gophertunnel `packet/player_auth_input.go` `Marshal` (et de
/// Cloudburst `PlayerAuthInputSerializer_v2168.serialize`) :
///
/// ```text
/// f32 pitch, f32 yaw, Vec3 position, Vec2 moveVector, f32 headYaw,
/// InputData (bool présent + VarUInt32 n + n × VarInt32 id),
/// VarUInt32 inputMode, VarUInt32 playMode, VarInt32 interactionModel,
/// f32 interactPitch, f32 interactYaw, VarUInt64 tick, Vec3 delta,
/// 5 × double optionnel (01 00 si absent) : transaction d'usage d'objet,
///   requête de stack, actions de bloc, rotation de véhicule, véhicule prédit,
/// Vec2 analogMoveVector, Vec3 cameraOrientation, Vec2 rawMoveVector
/// ```
pub fn encode_player_auth_input(input: &AuthInput) -> Vec<u8> {
    let mut w = ProtoWriter::with_capacity(96);
    w.write_f32_le(input.pitch);
    w.write_f32_le(input.yaw);
    w.write_f32_le(input.pos[0]);
    w.write_f32_le(input.pos[1]);
    w.write_f32_le(input.pos[2]);
    w.write_f32_le(input.move_vec[0]);
    w.write_f32_le(input.move_vec[1]);
    w.write_f32_le(input.head_yaw);
    write_input_flags(&mut w, input.input_flags);
    w.write_var_u32(1); // input_mode = Mouse
    w.write_var_u32(0); // play_mode = Normal
    w.write_var_i32(0); // interaction_model = Touch (VarInt32 zigzag en 2169)
    w.write_f32_le(0.0); // interact_pitch
    w.write_f32_le(0.0); // interact_yaw
    w.write_var_u64(input.tick);
    w.write_f32_le(input.delta[0]);
    w.write_f32_le(input.delta[1]);
    w.write_f32_le(input.delta[2]);
    // Les 5 blocs optionnels « doubles » : optionnel externe toujours présent
    // (1), optionnel interne absent (0). Réf. gophertunnel `DoubleOptionalFunc`.
    for _ in 0..5 {
        w.write_bool(true);
        w.write_bool(false);
    }
    // Clavier/souris : pas de stick analogique.
    w.write_f32_le(0.0);
    w.write_f32_le(0.0);
    let camera = direction_vector(input.pitch, input.yaw);
    w.write_f32_le(camera[0]);
    w.write_f32_le(camera[1]);
    w.write_f32_le(camera[2]);
    // Vecteur brut = vecteur de déplacement (pas de modificateur appliqué).
    w.write_f32_le(input.move_vec[0]);
    w.write_f32_le(input.move_vec[1]);
    w.into_bytes()
}

/// Écrit la liste des flags PlayerAuthInput au format 2169 : `bool présent`
/// (toujours 1, comme Cloudburst), `VarUInt32 n`, puis `n × VarInt32` (zigzag)
/// des ids par ordre croissant. Réf. gophertunnel `InputFlagList`. Les bits
/// hors `0..66` sont ignorés : le serveur rejetterait l'id.
fn write_input_flags(w: &mut ProtoWriter, flags: u128) {
    let ids: Vec<i32> = (0..INPUT_FLAG_COUNT)
        .filter(|&bit| flags & (1u128 << bit) != 0)
        .map(|bit| bit as i32)
        .collect();
    w.write_bool(true);
    w.write_var_u32(ids.len() as u32);
    for id in ids {
        w.write_var_i32(id);
    }
}

/// Vecteur de regard normalisé à partir de pitch/yaw en degrés. Formule de
/// PMMP `Entity::getDirectionVector` (sert d'orientation caméra).
fn direction_vector(pitch: f32, yaw: f32) -> [f32; 3] {
    let (pitch, yaw) = (pitch.to_radians(), yaw.to_radians());
    let xz = pitch.cos();
    [-xz * yaw.sin(), -pitch.sin(), xz * yaw.cos()]
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

    #[test]
    fn test_resource_pack_response_downloading_finished() {
        let payload = encode_resource_pack_client_response(PackResponse::DownloadingFinished, &[]);
        let mut expected = vec![0x02, 19];
        expected.extend_from_slice(b"downloadingfinished");
        assert_eq!(payload, expected);
    }

    #[test]
    fn test_resource_pack_response_stack_finished_ignores_packs() {
        // La liste n'est émise que pour « downloading ».
        let payload = encode_resource_pack_client_response(
            PackResponse::ResourcePackStackFinished,
            &["x_1.0.0".to_string()],
        );
        let mut expected = vec![0x03, 25];
        expected.extend_from_slice(b"resourcepackstackfinished");
        assert_eq!(payload, expected);
    }

    #[test]
    fn test_resource_pack_response_downloading_lists_packs() {
        let packs = ["a_1.0.0".to_string(), "b_2.0.0".to_string()];
        let payload = encode_resource_pack_client_response(PackResponse::Downloading, &packs);
        let mut r = ProtoReader::new(&payload);
        assert_eq!(r.read_var_u32().unwrap(), 1);
        assert_eq!(r.read_string().unwrap(), "downloading");
        assert_eq!(r.read_var_u32().unwrap(), 2);
        assert_eq!(r.read_string().unwrap(), "a_1.0.0");
        assert_eq!(r.read_string().unwrap(), "b_2.0.0");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_player_auth_input_layout_2169() {
        // Flags 25 (StartSprinting) et 65 (InternalUpdate, nouveau en 2169) ;
        // le bit 70 est hors plage et doit être ignoré.
        let flags = (1u128 << 25) | (1u128 << 65) | (1u128 << 70);
        let input = AuthInput {
            pos: [1.0, 2.0, 3.0],
            pitch: 0.0,
            yaw: 90.0,
            head_yaw: 90.0,
            move_vec: [0.0, 1.0],
            delta: [0.5, 0.0, -0.5],
            input_flags: flags,
            tick: 300,
        };
        let payload = encode_player_auth_input(&input);

        // Tête : pitch, yaw, position, moveVector, headYaw = 8 × f32 LE.
        assert_eq!(&payload[0..4], &0.0f32.to_le_bytes());
        assert_eq!(&payload[4..8], &90.0f32.to_le_bytes());
        assert_eq!(&payload[28..32], &90.0f32.to_le_bytes());
        // InputData : présent, 2 ids, zigzag(25) = 50, zigzag(65) = 130 (82 01),
        // puis inputMode = 1 (Mouse).
        assert_eq!(&payload[32..38], &[0x01, 0x02, 50, 0x82, 0x01, 0x01]);

        let mut r = ProtoReader::new(&payload);
        for expected in [0.0, 90.0, 1.0, 2.0, 3.0, 0.0, 1.0, 90.0] {
            assert_eq!(r.read_f32_le().unwrap(), expected);
        }
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_var_u32().unwrap(), 2);
        assert_eq!(r.read_var_i32().unwrap(), 25);
        assert_eq!(r.read_var_i32().unwrap(), 65);
        assert_eq!(r.read_var_u32().unwrap(), 1); // inputMode = Mouse
        assert_eq!(r.read_var_u32().unwrap(), 0); // playMode = Normal
        assert_eq!(r.read_var_i32().unwrap(), 0); // interactionModel
        assert_eq!(r.read_f32_le().unwrap(), 0.0);
        assert_eq!(r.read_f32_le().unwrap(), 0.0);
        assert_eq!(r.read_var_u64().unwrap(), 300);
        for expected in [0.5, 0.0, -0.5] {
            assert_eq!(r.read_f32_le().unwrap(), expected);
        }
        // 5 doubles optionnels : 01 00.
        for _ in 0..5 {
            assert!(r.read_bool().unwrap());
            assert!(!r.read_bool().unwrap());
        }
        // analogMoveVector
        assert_eq!(r.read_f32_le().unwrap(), 0.0);
        assert_eq!(r.read_f32_le().unwrap(), 0.0);
        // cameraOrientation (yaw 90 → -X, cf. PMMP getDirectionVector)
        let camera = [
            r.read_f32_le().unwrap(),
            r.read_f32_le().unwrap(),
            r.read_f32_le().unwrap(),
        ];
        assert!((camera[0] + 1.0).abs() < 1e-6);
        assert!(camera[1].abs() < 1e-6);
        assert!(camera[2].abs() < 1e-6);
        // rawMoveVector = moveVector
        assert_eq!(r.read_f32_le().unwrap(), 0.0);
        assert_eq!(r.read_f32_le().unwrap(), 1.0);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_player_auth_input_without_flags() {
        let payload = encode_player_auth_input(&AuthInput::default());
        // Flags : présent + liste vide (01 00), puis inputMode 1.
        assert_eq!(&payload[32..35], &[0x01, 0x00, 0x01]);
        // 8 f32 + 01 00 + 3 varints + 2 f32 + tick + 3 f32 + 5 × 01 00 + 7 f32.
        assert_eq!(payload.len(), 32 + 2 + 3 + 8 + 1 + 12 + 10 + 28);
    }
}
