use base64::Engine;
use tracing::{debug, info, warn};

use mc_rs_crypto::ecdh;
use mc_rs_crypto::jwt;
use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::login::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

use crate::player_data;

use super::{Connection, ConnectionState};

impl Connection {
    pub(super) fn handle_request_network_settings(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        let Ok(pkt) = RequestNetworkSettings::decode(reader) else {
            return Vec::new();
        };

        info!(
            "[{}] RequestNetworkSettings: protocol={}",
            self.addr, pkt.protocol_version
        );

        if pkt.protocol_version != 975 {
            warn!(
                "[{}] Incompatible protocol: {} (expected 975)",
                self.addr, pkt.protocol_version
            );
            let disconnect = Disconnect {
                reason: DisconnectReason::Unknown,
                message: Some("Incompatible protocol version".to_string()),
            };
            return vec![self.encode_raw_packet(packet_id::DISCONNECT, &disconnect.encode())];
        }

        let settings = NetworkSettings::default_settings();
        let response = self.encode_raw_packet(packet_id::NETWORK_SETTINGS, &settings.encode());

        self.state = ConnectionState::Login;
        debug!("[{}] -> Login state", self.addr);

        vec![response]
    }

    pub(super) fn handle_login(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = Login::decode(reader) else {
            warn!("[{}] Failed to decode Login packet", self.addr);
            return Vec::new();
        };

        info!("[{}] Login: protocol={}", self.addr, pkt.protocol_version);

        // Parse authInfoJson to extract identity and public key
        match jwt::extract_login_identity(&pkt.chain_data) {
            Ok(identity) => {
                self.client_pub_key_b64 = Some(identity.public_key_b64);
                self.display_name = Some(identity.display_name);
                self.xuid = if identity.xuid.is_empty() {
                    None
                } else {
                    Some(identity.xuid)
                };
                if !identity.uuid_str.is_empty() {
                    self.uuid = uuid::Uuid::parse_str(&identity.uuid_str).ok();
                }
                // Load saved player data if exists
                if let Some(ref xuid) = self.xuid {
                    if let Some(save) = player_data::load_player(xuid) {
                        self.position = [
                            save.position[0] as f32,
                            save.position[1] as f32,
                            save.position[2] as f32,
                        ];
                        self.yaw = save.rotation[0];
                        self.pitch = save.rotation[1];
                        self.gamemode = save.gamemode;
                        if let Some(spawn_position) = save.spawn_position {
                            self.spawn_position = [
                                spawn_position[0] as f32,
                                spawn_position[1] as f32,
                                spawn_position[2] as f32,
                            ];
                        }
                        self.inventory = save.inventory.into_runtime();
                        info!(
                            "[{}] Restored position: {:.1}, {:.1}, {:.1} (gamemode={})",
                            self.addr,
                            self.position[0],
                            self.position[1],
                            self.position[2],
                            self.gamemode
                        );
                    }
                }

                info!(
                    "[{}] Player: {} (xuid={}, auth={})",
                    self.addr,
                    self.display_name.as_deref().unwrap_or("?"),
                    self.xuid.as_deref().unwrap_or("none"),
                    if identity.authenticated {
                        "xbox"
                    } else {
                        "offline"
                    },
                );
            }
            Err(e) => {
                warn!("[{}] Login identity parse failed: {}", self.addr, e);
                // Fallback: try to get the key from the client data JWT header
                if !pkt.client_data_jwt.is_empty() {
                    if let Ok(decoded) = jwt::decode_jwt(&pkt.client_data_jwt) {
                        if let Some(key) = decoded.header.get("x5u").and_then(|v| v.as_str()) {
                            debug!("[{}] Got client key from client_data JWT x5u", self.addr);
                            self.client_pub_key_b64 = Some(key.to_string());
                        }
                    }
                }
            }
        }

        // Parse skin depuis client_data_jwt (claims = SkinData PMMP). Pas critique
        // si ça échoue — broadcast tombera sur SerializedSkin::default.
        if !pkt.client_data_jwt.is_empty() {
            if let Ok(decoded) = jwt::decode_jwt(&pkt.client_data_jwt) {
                if let Some(skin) = crate::skins::Skin::from_client_data(&decoded.claims) {
                    debug!(
                        "[{}] Skin parsed: id={} {}x{} ({} bytes)",
                        self.addr,
                        skin.skin_id,
                        skin.skin_width,
                        skin.skin_height,
                        skin.skin_data.len()
                    );
                    self.skin = Some(skin);
                }
            }
        }

        // Set up encryption
        let Some(ref client_pub_b64) = self.client_pub_key_b64 else {
            warn!("[{}] No client public key found", self.addr);
            return Vec::new();
        };

        let client_pub_key = match ecdh::parse_client_public_key(client_pub_b64) {
            Ok(key) => key,
            Err(e) => {
                warn!(
                    "[{}] Failed to parse client public key: {} (key_b64_len={}, first_chars={})",
                    self.addr,
                    e,
                    client_pub_b64.len(),
                    &client_pub_b64[..client_pub_b64.len().min(40)]
                );
                return Vec::new();
            }
        };

        // Generate salt
        let mut salt = [0u8; 16];
        rand::Rng::fill(&mut rand::thread_rng(), &mut salt);

        // Derive AES key
        let aes_key = self.server_keypair.derive_aes_key(&client_pub_key, &salt);

        // Create handshake JWT
        let server_pub_b64 = self.server_keypair.public_key_base64();
        let salt_b64 = base64::engine::general_purpose::STANDARD.encode(salt);
        let keypair = self.server_keypair.clone();
        let handshake_jwt =
            jwt::create_handshake_jwt(&server_pub_b64, &salt_b64, |data| keypair.sign(data));

        let handshake_pkt = ServerToClientHandshake { jwt: handshake_jwt };
        let response = self.encode_compressed_packet(
            packet_id::SERVER_TO_CLIENT_HANDSHAKE,
            &handshake_pkt.encode(),
        );

        // DON'T enable encryption yet -- the ServerToClientHandshake must be sent unencrypted.
        // Store the key to activate AFTER this packet is sent.
        self.pending_encryption_key = Some(aes_key);

        self.state = ConnectionState::Handshake;
        debug!("[{}] -> Handshake state (encryption pending)", self.addr);

        vec![response]
    }

    pub(super) fn handle_client_to_server_handshake(
        &mut self,
        _reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        info!(
            "[{}] ClientToServerHandshake received -- encryption verified",
            self.addr
        );

        // Send PlayStatus(LOGIN_SUCCESS)
        let play_status = PlayStatus {
            status: PlayStatusType::LoginSuccess,
        };
        let response = self.encode_compressed_packet(packet_id::PLAY_STATUS, &play_status.encode());

        self.state = ConnectionState::ResourcePacks;
        debug!("[{}] -> ResourcePacks state", self.addr);

        // Also send ResourcePacksInfo immediately
        let mut responses = vec![response];
        responses.extend(self.send_resource_packs_info());
        responses
    }

    pub(super) fn handle_resource_pack_client_response(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        let Ok(status) = reader.read_u8() else {
            return Vec::new();
        };
        // Le client envoie aussi la liste des pack IDs qu'il veut DL
        // (string array). Chaque ID a la forme "<uuid>_<version>".
        let pack_count = reader.read_u16_le().unwrap_or(0);
        let mut requested_packs: Vec<String> = Vec::with_capacity(pack_count as usize);
        for _ in 0..pack_count {
            if let Ok(id) = reader.read_string() {
                requested_packs.push(id);
            }
        }

        debug!(
            "[{}] ResourcePackClientResponse: status={} requested={}",
            self.addr,
            status,
            requested_packs.len()
        );

        match status {
            // 1 = REFUSED → le client ne veut pas du serveur, il se déco.
            1 => {
                info!("[{}] Resource packs refused — disconnecting", self.addr);
                Vec::new()
            }
            // 2 = SEND_PACKS — pour chaque pack demandé, envoyer
            // ResourcePackDataInfo. Le client suivra avec
            // ResourcePackChunkRequest pour chaque chunk.
            2 => {
                info!(
                    "[{}] Client requesting {} resource packs",
                    self.addr,
                    requested_packs.len()
                );
                let mut out = Vec::with_capacity(requested_packs.len());
                for id in requested_packs {
                    // Le client envoie "<uuid>_<version>" — on ne matche que
                    // sur l'UUID (préfixe).
                    let uuid_part = id.split('_').next().unwrap_or(&id);
                    if let Some(pack) = self
                        .resource_packs
                        .iter()
                        .find(|p| p.uuid().eq_ignore_ascii_case(uuid_part))
                    {
                        out.push(self.encode_resource_pack_data_info(pack));
                    } else {
                        warn!(
                            "[{}] Client requested unknown resource pack id={}",
                            self.addr, id
                        );
                    }
                }
                out
            }
            3 => {
                // HAVE_ALL_PACKS -> send ResourcePackStack
                info!(
                    "[{}] HAVE_ALL_PACKS — sending ResourcePackStack ({} packs)",
                    self.addr,
                    self.resource_packs.len()
                );
                self.send_resource_pack_stack()
            }
            4 => {
                // COMPLETED -> transition to PreSpawn
                info!("[{}] Resource packs completed", self.addr);
                self.state = ConnectionState::PreSpawn;
                debug!("[{}] -> PreSpawn state", self.addr);
                self.send_pre_spawn_packets()
            }
            _ => {
                debug!(
                    "[{}] Unexpected resource pack status: {}",
                    self.addr, status
                );
                Vec::new()
            }
        }
    }

    /// Handle ResourcePackChunkRequest (C→S, 0x54). Le client demande un
    /// chunk spécifique d'un pack. On répond avec ResourcePackChunkData.
    pub(super) fn handle_resource_pack_chunk_request(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        let req = match mc_rs_proto::packets::world::ResourcePackChunkRequest::decode(reader) {
            Ok(r) => r,
            Err(e) => {
                debug!(
                    "[{}] Failed to decode ResourcePackChunkRequest: {:?}",
                    self.addr, e
                );
                return Vec::new();
            }
        };
        debug!(
            "[{}] ResourcePackChunkRequest pack_id={} chunk={}",
            self.addr, req.pack_id, req.chunk_index
        );

        let uuid_part = req.pack_id.split('_').next().unwrap_or(&req.pack_id);
        let Some(pack) = self
            .resource_packs
            .iter()
            .find(|p| p.uuid().eq_ignore_ascii_case(uuid_part))
        else {
            warn!(
                "[{}] Chunk requested for unknown pack id={}",
                self.addr, req.pack_id
            );
            return Vec::new();
        };

        let chunk_size = crate::pack_encoder::CHUNK_SIZE as usize;
        let offset = (req.chunk_index as u64) * (chunk_size as u64);
        let slice = pack.chunk(req.chunk_index, chunk_size);
        let response = mc_rs_proto::packets::world::ResourcePackChunkData {
            pack_id: pack.uuid().to_string(),
            chunk_index: req.chunk_index,
            offset,
            data: slice.to_vec(),
        };
        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACK_CHUNK_DATA, &response.encode())]
    }

    // -- Resource pack helpers --

    fn send_resource_packs_info(&self) -> Vec<Vec<u8>> {
        let packs = self.resource_packs.as_ref();
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(128 + 96 * packs.len());
        writer.write_bool(true); // must_accept — force server pack priority
        writer.write_bool(false); // has_addons
        writer.write_bool(false); // has_scripts
        writer.write_bool(false); // force_disable_vibrant_visuals
                                  // worldTemplateId (nil UUID = 2 x i64_le zero)
        writer.write_i64_le(0);
        writer.write_i64_le(0);
        writer.write_string(""); // worldTemplateVersion
        writer.write_u16_le(packs.len() as u16); // count

        for pack in packs {
            // PMMP putUUID = 2 x i64_le reversed des bytes UUID. On encode
            // via le format canonique 8 bytes / 8 bytes reverse.
            write_uuid_pmmp(&mut writer, pack.uuid());
            writer.write_string(&pack.version_string());
            writer.write_u64_le(pack.size());
            writer.write_string(""); // encryptionKey
            writer.write_string(""); // subPackName
            writer.write_string(""); // contentId
            writer.write_bool(false); // hasScripts
            writer.write_bool(false); // isAddonPack
            writer.write_bool(false); // isRtxCapable
            writer.write_string(""); // cdnUrl
        }

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACKS_INFO, writer.as_bytes())]
    }

    fn send_resource_pack_stack(&self) -> Vec<Vec<u8>> {
        let packs = self.resource_packs.as_ref();
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(64 + 48 * packs.len());
        writer.write_bool(true); // must_accept — force stack priority
        writer.write_var_u32(packs.len() as u32);
        for pack in packs {
            // PMMP ResourcePackStackEntry : pack_id en STRING ici (pas UUID
            // binaire), version en STRING, subPackName en STRING.
            writer.write_string(pack.uuid());
            writer.write_string(&pack.version_string());
            writer.write_string(""); // subPackName
        }
        writer.write_string("1.26.20"); // baseGameVersion
        writer.write_u32_le(0); // experiments count
        writer.write_bool(false); // hasPreviouslyUsedExperiments
        writer.write_bool(false); // useVanillaEditorPacks

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACK_STACK, writer.as_bytes())]
    }

    fn encode_resource_pack_data_info(
        &self,
        pack: &crate::resource_pack::ResourcePack,
    ) -> Vec<u8> {
        let chunk_size = crate::pack_encoder::CHUNK_SIZE as u32;
        let total = pack.size();
        let chunk_count = crate::pack_encoder::num_chunks(total) as u32;
        let info = mc_rs_proto::packets::world::ResourcePackDataInfo {
            pack_id: pack.uuid().to_string(),
            max_chunk_size: chunk_size,
            chunk_count,
            compressed_pack_size: total,
            sha256: pack.sha256, // RAW 32 bytes (cf. PMMP hash_file ..., true).
            is_premium: false,
            pack_type: 0, // Resources
        };
        self.encode_compressed_packet(packet_id::RESOURCE_PACK_DATA_INFO, &info.encode())
    }
}

/// PMMP `CommonTypes::putUUID` : 2 longs little-endian, bytes 7..0 puis 15..8.
/// Pour un UUID canonique "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx", on extrait
/// les 16 bytes raw puis on les reverse par moitié.
fn write_uuid_pmmp(writer: &mut mc_rs_proto::io::ProtoWriter, uuid_str: &str) {
    let bytes = uuid::Uuid::parse_str(uuid_str)
        .map(|u| *u.as_bytes())
        .unwrap_or([0u8; 16]);
    let mut p1 = [0u8; 8];
    let mut p2 = [0u8; 8];
    p1.copy_from_slice(&bytes[0..8]);
    p2.copy_from_slice(&bytes[8..16]);
    p1.reverse();
    p2.reverse();
    writer.write_raw(&p1);
    writer.write_raw(&p2);
}
