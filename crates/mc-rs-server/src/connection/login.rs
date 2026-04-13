use base64::Engine;
use tracing::{debug, info, warn};

use mc_rs_crypto::ecdh;
use mc_rs_crypto::jwt;
use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::login::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::{ItemStack, ItemStackWrapper};

use crate::player_data;

use super::spawn::hub_menu_item_id;
use super::{Connection, ConnectionState, HUB_MENU_SLOT};

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

        if pkt.protocol_version != 944 {
            warn!(
                "[{}] Incompatible protocol: {} (expected 944)",
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
                self.ensure_hub_menu_item();

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

        debug!(
            "[{}] ResourcePackClientResponse: status={}",
            self.addr, status
        );

        match status {
            3 => {
                // HAVE_ALL_PACKS -> send ResourcePackStack
                self.send_resource_pack_stack()
            }
            4 => {
                // COMPLETED -> transition to PreSpawn
                info!("[{}] Resource packs completed", self.addr);
                self.state = ConnectionState::PreSpawn;
                self.ensure_hub_menu_item();
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

    // -- Resource pack helpers --

    fn send_resource_packs_info(&self) -> Vec<Vec<u8>> {
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(64);
        writer.write_bool(false); // must_accept
        writer.write_bool(false); // has_addons
        writer.write_bool(false); // has_scripts
        writer.write_bool(false); // force_disable_vibrant_visuals
                                  // World template UUID (nil = 16 zero bytes, written as 2 x i64_le)
        writer.write_i64_le(0);
        writer.write_i64_le(0);
        writer.write_string(""); // world_template_version
        writer.write_u16_le(0); // resource_packs count

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACKS_INFO, writer.as_bytes())]
    }

    fn send_resource_pack_stack(&self) -> Vec<Vec<u8>> {
        let mut writer = mc_rs_proto::io::ProtoWriter::with_capacity(64);
        writer.write_bool(false); // must_accept
        writer.write_var_u32(0); // resource_pack_stack count
        writer.write_string("1.26.10"); // base_game_version
        writer.write_u32_le(0); // experiments count
        writer.write_bool(false); // experiments_previously_toggled
        writer.write_bool(false); // use_vanilla_editor_packs

        vec![self.encode_compressed_packet(packet_id::RESOURCE_PACK_STACK, writer.as_bytes())]
    }

    /// Helper used by login and resource pack response to re-ensure the hub menu item.
    pub(super) fn ensure_hub_menu_item(&mut self) {
        if self
            .inventory
            .slots
            .iter()
            .any(|slot| slot.item.id == hub_menu_item_id() && !slot.item.is_air())
        {
            return;
        }

        let menu_item = ItemStack::new(hub_menu_item_id(), 1, 0);
        if self.inventory.slots[HUB_MENU_SLOT].item.is_air() {
            let stack_id = self.inventory.next_stack_id();
            self.inventory.slots[HUB_MENU_SLOT] = ItemStackWrapper::new(menu_item, stack_id);
        } else {
            let _ = self.inventory.add_item(menu_item);
        }
    }
}
