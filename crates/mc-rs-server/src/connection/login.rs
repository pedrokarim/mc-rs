use super::*;

impl ConnectionHandler {
    pub(super) fn handle_session_connected(&mut self, addr: SocketAddr, guid: i64) {
        info!("Session connected: {addr} (GUID: {guid})");
        let entity_id = self.allocate_entity_id();
        self.connections.insert(
            addr,
            PlayerConnection {
                state: LoginState::AwaitingNetworkSettings,
                batch_config: BatchConfig::default(),
                login_data: None,
                client_data: None,
                encryption: None,
                pending_encryption: None,
                entity_unique_id: entity_id,
                entity_runtime_id: entity_id as u64,
                position: self.spawn_position,
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
                on_ground: true,
                client_tick: 0,
                sent_chunks: HashSet::new(),
                chunk_radius: 0,
                dimension: self.dimension_id,
                portal_cooldown_until: 0,
                gamemode: gamemode_from_str(&self.server_config.server.gamemode),
                breaking_block: None,
                airborne_ticks: 0,
                inventory: PlayerInventory::new(),
                health: 20.0,
                last_damage_tick: None,
                is_dead: false,
                is_sprinting: false,
                effects: Vec::new(),
                last_position_delta_y: 0.0,
                fire_ticks: 0,
                food: 20,
                saturation: 5.0,
                exhaustion: 0.0,
                fall_distance: 0.0,
                air_ticks: 300,
                is_swimming: false,
                xp_level: 0,
                xp_total: 0,
                pending_forms: HashMap::new(),
                open_container: None,
                next_window_id: 1,
                enchant_seed: rand::thread_rng().gen(),
                pending_enchant_options: Vec::new(),
                tags: HashSet::new(),
                protocol_version: packets::PROTOCOL_VERSION,
                violations: ViolationTracker::default(),
                last_break_tick: 0,
                last_place_tick: 0,
                last_attack_tick: 0,
                last_command_tick: 0,
                actions_this_second: 0,
                action_second_start: 0,
            },
        );

        // Spawn ECS mirror entity for this player
        self.game_world.spawn_player(
            entity_id,
            entity_id as u64,
            (
                self.spawn_position.x,
                self.spawn_position.y,
                self.spawn_position.z,
            ),
            addr,
        );
        self.runtime_id_to_addr.insert(entity_id as u64, addr);
    }

    pub(super) async fn handle_session_disconnected(&mut self, addr: SocketAddr) {
        // Save player data before removing connection
        if let Some(conn) = self.connections.get(&addr) {
            if conn.state == LoginState::InGame {
                if let Some(ref login) = conn.login_data {
                    let player_data = PlayerData::from_connection(conn);
                    if let Err(e) = player_data.save(&self.world_dir, &login.identity) {
                        warn!("Failed to save player data for {}: {e}", login.display_name);
                    }
                }
            }
        }

        // Collect data before removing from connections
        let (was_in_game, entity_unique_id, uuid, display_name) = match self.connections.get(&addr)
        {
            Some(conn) => {
                let in_game = conn.state == LoginState::InGame;
                let uid = conn.entity_unique_id;
                let (uuid, name) = match &conn.login_data {
                    Some(d) => (
                        Uuid::parse(&d.identity).unwrap_or(Uuid::ZERO),
                        d.display_name.clone(),
                    ),
                    None => (Uuid::ZERO, String::new()),
                };
                (in_game, uid, uuid, name)
            }
            None => return,
        };

        // Plugin event: PlayerQuit
        if was_in_game {
            if let Some(conn) = self.connections.get(&addr) {
                let player = Self::make_plugin_player(conn);
                let event = PluginEvent::PlayerQuit { player };
                let snapshot = self.build_snapshot();
                let (_, actions) = self.plugin_manager.dispatch(&event, &snapshot);
                self.apply_plugin_actions(actions).await;
            }
        }

        // Clean up projectiles belonging to this player
        self.cleanup_player_projectiles(addr).await;

        // Remove the connection
        self.runtime_id_to_addr.remove(&(entity_unique_id as u64));
        self.connections.remove(&addr);

        // Despawn the ECS mirror entity for this player
        self.game_world.despawn_player(entity_unique_id);

        if was_in_game {
            // Broadcast RemoveEntity to all remaining players
            self.broadcast_packet(
                packets::id::REMOVE_ENTITY,
                &RemoveEntity { entity_unique_id },
            )
            .await;

            // Broadcast PlayerList(Remove) to all remaining players
            self.broadcast_packet(
                packets::id::PLAYER_LIST,
                &PlayerListRemove { uuids: vec![uuid] },
            )
            .await;

            // Broadcast leave message
            let leave_msg = Text::system(format!("{display_name} left the game"));
            self.broadcast_packet(packets::id::TEXT, &leave_msg).await;

            info!("Player {display_name} disconnected ({addr})");
        } else {
            info!("Session disconnected: {addr}");
        }
    }

    pub(super) async fn handle_packet(&mut self, addr: SocketAddr, payload: Bytes) {
        // Decrypt if encryption is active
        let decrypted = {
            let conn = match self.connections.get_mut(&addr) {
                Some(c) => c,
                None => {
                    warn!("Packet from unknown session {addr}");
                    return;
                }
            };
            if let Some(enc) = conn.encryption.as_mut() {
                match enc.decrypt(&payload) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Decrypt error from {addr}: {e}");
                        return;
                    }
                }
            } else {
                payload
            }
        };

        // Clone batch_config to avoid borrow issues
        let batch_config = match self.connections.get(&addr) {
            Some(conn) => conn.batch_config.clone(),
            None => return,
        };

        let sub_packets = match decode_batch(decrypted, &batch_config) {
            Ok(p) => p,
            Err(e) => {
                warn!("Batch decode error from {addr}: {e}");
                return;
            }
        };

        for sub_packet in sub_packets {
            let mut cursor = Cursor::new(&sub_packet[..]);
            let packet_id = match VarUInt32::proto_decode(&mut cursor) {
                Ok(id) => id.0,
                Err(e) => {
                    warn!("Bad packet ID from {addr}: {e}");
                    continue;
                }
            };

            match packet_id {
                packets::id::REQUEST_NETWORK_SETTINGS => {
                    self.handle_request_network_settings(addr, &mut cursor)
                        .await;
                }
                packets::id::LOGIN => {
                    self.handle_login(addr, &mut cursor).await;
                }
                packets::id::CLIENT_TO_SERVER_HANDSHAKE => {
                    self.handle_client_to_server_handshake(addr, &mut cursor)
                        .await;
                }
                packets::id::RESOURCE_PACK_CLIENT_RESPONSE => {
                    self.handle_resource_pack_client_response(addr, &mut cursor)
                        .await;
                }
                packets::id::RESOURCE_PACK_CHUNK_REQUEST => {
                    self.handle_resource_pack_chunk_request(addr, &mut cursor)
                        .await;
                }
                packets::id::REQUEST_CHUNK_RADIUS => {
                    self.handle_request_chunk_radius(addr, &mut cursor).await;
                }
                packets::id::SET_LOCAL_PLAYER_AS_INITIALIZED => {
                    self.handle_set_local_player_as_initialized(addr, &mut cursor)
                        .await;
                }
                packets::id::TEXT => {
                    self.handle_text(addr, &mut cursor).await;
                }
                packets::id::COMMAND_REQUEST => {
                    self.handle_command_request(addr, &mut cursor).await;
                }
                packets::id::MOB_EQUIPMENT => {
                    self.handle_mob_equipment(addr, &mut cursor).await;
                }
                packets::id::INVENTORY_TRANSACTION => {
                    self.handle_inventory_transaction(addr, &mut cursor).await;
                }
                packets::id::PLAYER_ACTION => {
                    self.handle_player_action(addr, &mut cursor).await;
                }
                packets::id::ITEM_STACK_REQUEST => {
                    self.handle_item_stack_request(addr, &mut cursor).await;
                }
                packets::id::PLAYER_AUTH_INPUT => {
                    self.handle_player_auth_input(addr, &mut cursor).await;
                }
                packets::id::ANIMATE => {
                    self.handle_animate(addr, &mut cursor).await;
                }
                packets::id::RESPAWN => {
                    self.handle_respawn(addr, &mut cursor).await;
                }
                packets::id::MODAL_FORM_RESPONSE => {
                    self.handle_modal_form_response(addr, &mut cursor).await;
                }
                packets::id::CONTAINER_CLOSE => {
                    self.handle_container_close(addr, &mut cursor).await;
                }
                packets::id::BLOCK_ACTOR_DATA => {
                    self.handle_block_actor_data(addr, &mut cursor).await;
                }
                packets::id::PLAYER_SKIN => {
                    self.handle_player_skin(addr, &mut cursor).await;
                }
                other => {
                    debug!(
                        "Game packet 0x{other:02X} from {addr}: {} bytes",
                        sub_packet.len()
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Network negotiation
    // -----------------------------------------------------------------------

    async fn handle_request_network_settings(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingNetworkSettings => {}
            _ => {
                warn!("Unexpected RequestNetworkSettings from {addr}");
                return;
            }
        }

        let request = match packets::RequestNetworkSettings::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad RequestNetworkSettings from {addr}: {e}");
                return;
            }
        };

        info!(
            "RequestNetworkSettings from {addr}: protocol {}",
            request.protocol_version
        );

        if !packets::is_supported_version(request.protocol_version) {
            let status = if request.protocol_version < packets::MIN_PROTOCOL_VERSION {
                PlayStatusType::FailedClient
            } else {
                PlayStatusType::FailedServer
            };
            info!(
                "Protocol mismatch from {addr}: got {}, supported {}-{} -> {:?}",
                request.protocol_version,
                packets::MIN_PROTOCOL_VERSION,
                packets::PROTOCOL_VERSION,
                status
            );
            self.send_packet(addr, packets::id::PLAY_STATUS, &PlayStatus { status })
                .await;
            return;
        }

        let settings = NetworkSettings::default();
        self.send_packet(addr, packets::id::NETWORK_SETTINGS, &settings)
            .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.batch_config.compression_enabled = true;
            conn.batch_config.compression =
                CompressionAlgorithm::from_u16(settings.compression_algorithm)
                    .unwrap_or(CompressionAlgorithm::Zlib);
            conn.batch_config.compression_threshold = settings.compression_threshold as usize;
            conn.protocol_version = request.protocol_version;
            conn.state = LoginState::AwaitingLogin;
        }

        info!(
            "Sent NetworkSettings to {addr}, compression enabled (online_mode={})",
            self.online_mode
        );
    }

    // -----------------------------------------------------------------------
    // Phase 4: Authentication
    // -----------------------------------------------------------------------

    async fn handle_login(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingLogin => {}
            _ => {
                warn!("Unexpected LoginPacket from {addr}");
                return;
            }
        }

        let login = match packets::LoginPacket::proto_decode(buf) {
            Ok(l) => l,
            Err(e) => {
                warn!("Bad LoginPacket from {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Invalid login data"),
                )
                .await;
                return;
            }
        };

        let login_data = match jwt::extract_login_data(&login.chain_data) {
            Ok(data) => data,
            Err(e) => {
                warn!("JWT extraction failed for {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Authentication failed"),
                )
                .await;
                return;
            }
        };

        // Parse client data (skin, device info) from the client_data JWT
        let client_data = jwt::extract_client_data(&login.client_data_jwt).unwrap_or_else(|e| {
            warn!("Failed to parse client_data from {addr}: {e}, using defaults");
            jwt::ClientData::default()
        });

        info!(
            "Login from {addr}: {} (XUID: {}, UUID: {})",
            login_data.display_name, login_data.xuid, login_data.identity
        );

        // Check IP ban
        let ip_str = addr.ip().to_string();
        if let Some(ban) = self.permissions.banned_ips.get(&ip_str) {
            info!("Rejected banned IP {ip_str}: {}", ban.reason);
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {}", ban.reason)),
            )
            .await;
            return;
        }

        // Check player ban
        if let Some(ban) = self
            .permissions
            .banned_players
            .get(&login_data.display_name)
        {
            info!(
                "Rejected banned player {}: {}",
                login_data.display_name, ban.reason
            );
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message(format!("You are banned: {}", ban.reason)),
            )
            .await;
            return;
        }

        // Check whitelist
        if self.permissions.whitelist_enabled
            && !self
                .permissions
                .whitelist
                .contains(&login_data.display_name)
        {
            info!(
                "Rejected non-whitelisted player: {}",
                login_data.display_name
            );
            self.send_packet(
                addr,
                packets::id::DISCONNECT,
                &Disconnect::with_message("You are not whitelisted on this server."),
            )
            .await;
            return;
        }

        if self.online_mode {
            // Store client_data before encryption handshake
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.client_data = Some(client_data);
            }
            self.start_encryption_handshake(addr, login_data).await;
        } else {
            if let Some(conn) = self.connections.get_mut(&addr) {
                conn.login_data = Some(login_data);
                conn.client_data = Some(client_data);
                conn.state = LoginState::LoggedIn;
            }

            self.send_packet(
                addr,
                packets::id::PLAY_STATUS,
                &PlayStatus {
                    status: PlayStatusType::LoginSuccess,
                },
            )
            .await;

            info!("Sent PlayStatus(LoginSuccess) to {addr} (offline mode)");

            // Start resource pack exchange
            self.send_resource_packs_info(addr).await;
        }
    }

    async fn start_encryption_handshake(&mut self, addr: SocketAddr, login_data: jwt::LoginData) {
        let client_public = match parse_client_public_key(&login_data.identity_public_key) {
            Ok(pk) => pk,
            Err(e) => {
                warn!("Invalid client public key from {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Invalid public key"),
                )
                .await;
                return;
            }
        };

        let server_keypair = ServerKeyPair::generate();
        let shared_secret = server_keypair.shared_secret(&client_public);
        let salt: [u8; 16] = rand::random();
        let (aes_key, iv) = derive_key(&salt, &shared_secret);

        let jwt_string = match create_handshake_jwt(&server_keypair, &salt) {
            Ok(jwt) => jwt,
            Err(e) => {
                warn!("Failed to create handshake JWT for {addr}: {e}");
                self.send_packet(
                    addr,
                    packets::id::DISCONNECT,
                    &Disconnect::with_message("Encryption handshake failed"),
                )
                .await;
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.login_data = Some(login_data);
            conn.pending_encryption = Some(PendingEncryption { aes_key, iv });
            conn.state = LoginState::AwaitingHandshake;
        }

        self.send_packet(
            addr,
            packets::id::SERVER_TO_CLIENT_HANDSHAKE,
            &ServerToClientHandshake { jwt: jwt_string },
        )
        .await;

        info!("Sent ServerToClientHandshake to {addr}, awaiting client confirmation");
    }

    async fn handle_client_to_server_handshake(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::AwaitingHandshake => {}
            _ => {
                warn!("Unexpected ClientToServerHandshake from {addr}");
                return;
            }
        }

        if let Err(e) = ClientToServerHandshake::proto_decode(buf) {
            warn!("Bad ClientToServerHandshake from {addr}: {e}");
            return;
        }

        let activated = if let Some(conn) = self.connections.get_mut(&addr) {
            if let Some(pending) = conn.pending_encryption.take() {
                conn.encryption = Some(PacketEncryption::new(&pending.aes_key, &pending.iv));
                conn.state = LoginState::LoggedIn;
                true
            } else {
                warn!("No pending encryption for {addr}");
                false
            }
        } else {
            false
        };

        if !activated {
            return;
        }

        info!("Encryption activated for {addr}");

        self.send_packet(
            addr,
            packets::id::PLAY_STATUS,
            &PlayStatus {
                status: PlayStatusType::LoginSuccess,
            },
        )
        .await;

        info!("Sent PlayStatus(LoginSuccess) to {addr} (encrypted)");

        // Start resource pack exchange
        self.send_resource_packs_info(addr).await;
    }

    // -----------------------------------------------------------------------
    // Phase 5: Resource Packs
    // -----------------------------------------------------------------------

    async fn send_resource_packs_info(&mut self, addr: SocketAddr) {
        use mc_rs_proto::packets::resource_packs_info::BehaviorPackEntry;

        let bp_entries: Vec<BehaviorPackEntry> = self
            .behavior_packs
            .iter()
            .map(|pack| BehaviorPackEntry {
                uuid: pack.manifest.header.uuid.clone(),
                version: pack.manifest.version_string(),
                size: pack.pack_size,
                content_key: String::new(),
                sub_pack_name: String::new(),
                content_identity: String::new(),
                has_scripts: false,
            })
            .collect();

        let pack_info = ResourcePacksInfo {
            forcing_server_packs: self.server_config.packs.force_packs
                && !self.behavior_packs.is_empty(),
            behavior_packs: bp_entries,
            ..ResourcePacksInfo::default()
        };

        let pack_count = pack_info.behavior_packs.len();
        self.send_packet(addr, packets::id::RESOURCE_PACKS_INFO, &pack_info)
            .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::AwaitingResourcePackResponse;
        }

        if pack_count > 0 {
            info!("Sent ResourcePacksInfo to {addr} ({pack_count} behavior pack(s))");
        } else {
            info!("Sent ResourcePacksInfo to {addr} (no packs)");
        }
    }

    async fn handle_resource_pack_client_response(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let current_state = match self.connections.get(&addr) {
            Some(c) => c.state,
            None => return,
        };

        let response = match ResourcePackClientResponse::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad ResourcePackClientResponse from {addr}: {e}");
                return;
            }
        };

        debug!(
            "ResourcePackClientResponse from {addr}: {:?} (state={:?})",
            response.status, current_state
        );

        match (current_state, response.status) {
            (LoginState::AwaitingResourcePackResponse, ResourcePackResponseStatus::SendPacks) => {
                // Client wants us to send pack data — collect DataInfo packets first
                let data_infos: Vec<(packets::ResourcePackDataInfo, String)> = response
                    .resource_pack_ids
                    .iter()
                    .filter_map(|pack_id_str| {
                        let uuid = pack_id_str.split('_').next().unwrap_or(pack_id_str);
                        let pack = self
                            .behavior_packs
                            .iter()
                            .find(|p| p.manifest.header.uuid == uuid)?;
                        let bytes = pack.pack_bytes.as_ref()?;
                        let chunk_size: u32 = 1_048_576;
                        let chunk_count = (bytes.len() as u64).div_ceil(chunk_size as u64) as u32;
                        Some((
                            packets::ResourcePackDataInfo {
                                pack_id: format!(
                                    "{}_{}",
                                    pack.manifest.header.uuid,
                                    pack.manifest.version_string()
                                ),
                                max_chunk_size: chunk_size,
                                chunk_count,
                                pack_size: bytes.len() as u64,
                                pack_hash: String::new(),
                                is_premium: false,
                                pack_type: 2,
                            },
                            pack.manifest.header.name.clone(),
                        ))
                    })
                    .collect();

                for (data_info, name) in &data_infos {
                    self.send_packet(addr, packets::id::RESOURCE_PACK_DATA_INFO, data_info)
                        .await;
                    info!(
                        "Sent ResourcePackDataInfo for {name} to {addr} ({} chunks)",
                        data_info.chunk_count
                    );
                }
            }
            (
                LoginState::AwaitingResourcePackResponse,
                ResourcePackResponseStatus::HaveAllPacks,
            )
            | (LoginState::AwaitingResourcePackResponse, ResourcePackResponseStatus::Completed) => {
                // Client has all packs (or none needed) — send stack
                use mc_rs_proto::packets::resource_pack_stack::StackPackEntry;

                let bp_stack: Vec<StackPackEntry> = self
                    .behavior_packs
                    .iter()
                    .map(|pack| StackPackEntry {
                        uuid: pack.manifest.header.uuid.clone(),
                        version: pack.manifest.version_string(),
                        sub_pack_name: String::new(),
                    })
                    .collect();

                let stack = ResourcePackStack {
                    must_accept: self.server_config.packs.force_packs
                        && !self.behavior_packs.is_empty(),
                    behavior_packs: bp_stack,
                    ..ResourcePackStack::default()
                };

                self.send_packet(addr, packets::id::RESOURCE_PACK_STACK, &stack)
                    .await;

                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.state = LoginState::AwaitingResourcePackComplete;
                }

                info!("Sent ResourcePackStack to {addr}");
            }
            (LoginState::AwaitingResourcePackComplete, ResourcePackResponseStatus::Completed) => {
                // Resource packs done — start game initialization
                info!("Resource pack exchange complete for {addr}");
                self.send_start_game(addr).await;
            }
            _ => {
                warn!(
                    "Unexpected ResourcePackClientResponse {:?} in state {:?} from {addr}",
                    response.status, current_state
                );
            }
        }
    }

    /// Handle a ResourcePackChunkRequest (0x54) — send the requested chunk data.
    pub(super) async fn handle_resource_pack_chunk_request(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let request = match packets::ResourcePackChunkRequest::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad ResourcePackChunkRequest from {addr}: {e}");
                return;
            }
        };

        // Extract chunk data from the pack (collect before send to avoid borrow conflict)
        let uuid = request
            .pack_id
            .split('_')
            .next()
            .unwrap_or(&request.pack_id);
        let chunk_size: usize = 1_048_576;

        let chunk_data = self
            .behavior_packs
            .iter()
            .find(|p| p.manifest.header.uuid == uuid)
            .and_then(|pack| {
                let bytes = pack.pack_bytes.as_ref()?;
                let offset = request.chunk_index as usize * chunk_size;
                if offset >= bytes.len() {
                    return None;
                }
                let end = (offset + chunk_size).min(bytes.len());
                Some((
                    packets::ResourcePackChunkData {
                        pack_id: request.pack_id.clone(),
                        chunk_index: request.chunk_index,
                        progress: offset as u64,
                        data: bytes[offset..end].to_vec(),
                    },
                    bytes.len().div_ceil(chunk_size),
                ))
            });

        match chunk_data {
            Some((data, total_chunks)) => {
                self.send_packet(addr, packets::id::RESOURCE_PACK_CHUNK_DATA, &data)
                    .await;
                debug!(
                    "Sent chunk {}/{total_chunks} for pack {} to {addr}",
                    request.chunk_index, request.pack_id,
                );
            }
            None => {
                warn!(
                    "Failed to find chunk {} for pack {} from {addr}",
                    request.chunk_index, request.pack_id
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 6: World initialization
    // -----------------------------------------------------------------------

    async fn send_start_game(&mut self, addr: SocketAddr) {
        let (entity_unique_id, entity_runtime_id, player_uuid, client_proto) =
            match self.connections.get(&addr) {
                Some(c) => {
                    let uuid = c
                        .login_data
                        .as_ref()
                        .map(|d| d.identity.clone())
                        .unwrap_or_default();
                    (
                        c.entity_unique_id,
                        c.entity_runtime_id,
                        uuid,
                        c.protocol_version,
                    )
                }
                None => return,
            };

        // Try to load saved player data
        let saved = if !player_uuid.is_empty() {
            PlayerData::load(&self.world_dir, &player_uuid)
        } else {
            None
        };

        // Apply saved data to connection (position, health, inventory, effects, etc.)
        let player_position = if let Some(ref data) = saved {
            if let Some(conn) = self.connections.get_mut(&addr) {
                data.apply_to_connection(conn);
            }
            Vec3::new(data.position[0], data.position[1], data.position[2])
        } else {
            self.spawn_position
        };

        let player_rotation = if let Some(ref data) = saved {
            Vec2::new(data.pitch, data.yaw)
        } else {
            Vec2::ZERO
        };

        let config = &self.server_config;
        let gamemode = if let Some(ref data) = saved {
            data.gamemode
        } else {
            gamemode_from_str(&config.server.gamemode)
        };
        let difficulty = difficulty_from_str(&config.server.difficulty);
        let generator = generator_from_str(&config.world.generator);
        let enchant_seed = self
            .connections
            .get(&addr)
            .map(|c| c.enchant_seed)
            .unwrap_or(0);

        let start_game = StartGame {
            entity_unique_id,
            entity_runtime_id,
            player_gamemode: gamemode,
            player_position,
            rotation: player_rotation,
            seed: config.world.seed as u64,
            dimension: self.dimension_id,
            generator,
            world_gamemode: gamemode,
            difficulty,
            spawn_position: self.spawn_block,
            level_id: "level".into(),
            world_name: config.world.name.clone(),
            game_version: packets::game_version_for_protocol(client_proto).into(),
            rain_level: self.rain_level,
            lightning_level: self.lightning_level,
            current_tick: self.world_time,
            day_cycle_stop_time: if self.do_daylight_cycle {
                -1
            } else {
                self.world_time as i32
            },
            game_rules: vec![
                GameRule {
                    name: "dodaylightcycle".into(),
                    editable: false,
                    value: GameRuleValue::Bool(self.do_daylight_cycle),
                },
                GameRule {
                    name: "domobspawning".into(),
                    editable: false,
                    value: GameRuleValue::Bool(true),
                },
                GameRule {
                    name: "doweathercycle".into(),
                    editable: false,
                    value: GameRuleValue::Bool(self.do_weather_cycle),
                },
                GameRule {
                    name: "commandblocksenabled".into(),
                    editable: false,
                    value: GameRuleValue::Bool(false),
                },
            ],
            item_table: self
                .item_registry
                .item_table_entries()
                .into_iter()
                .map(|e| mc_rs_proto::packets::start_game::ItemTableEntry {
                    string_id: e.string_id,
                    numeric_id: e.numeric_id,
                    is_component_based: e.is_component_based,
                })
                .collect(),
            enchantment_seed: enchant_seed,
            ..StartGame::default()
        };

        self.send_packet(addr, packets::id::START_GAME, &start_game)
            .await;
        info!("Sent StartGame to {addr}");

        // Send creative content (items available in creative menu)
        let creative_items = mc_rs_proto::packets::creative_content::default_creative_items();
        let creative_content =
            mc_rs_proto::packets::creative_content::build_creative_content(&creative_items);
        self.send_packet(addr, packets::id::CREATIVE_CONTENT, &creative_content)
            .await;

        // Send crafting recipes
        let crafting_data = self.build_crafting_data();
        self.send_packet(addr, packets::id::CRAFTING_DATA, &crafting_data)
            .await;

        self.send_packet(
            addr,
            packets::id::BIOME_DEFINITION_LIST,
            &BiomeDefinitionList::canonical(),
        )
        .await;

        self.send_packet(
            addr,
            packets::id::AVAILABLE_ENTITY_IDENTIFIERS,
            &AvailableEntityIdentifiers::canonical(),
        )
        .await;

        self.send_packet(addr, packets::id::AVAILABLE_COMMANDS, &AvailableCommands)
            .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::Spawning;
        }

        info!("Sent world initialization packets to {addr}");
    }

    // -----------------------------------------------------------------------
    // Forms UI
    // -----------------------------------------------------------------------

    async fn handle_modal_form_response(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let response = match packets::ModalFormResponse::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad ModalFormResponse from {addr}: {e}");
                return;
            }
        };

        // Look up the form type from pending_forms
        let form_type = match self.connections.get_mut(&addr) {
            Some(conn) => conn.pending_forms.remove(&response.form_id),
            None => return,
        };

        let form_type = match form_type {
            Some(ft) => ft,
            None => {
                debug!(
                    "ModalFormResponse for unknown form_id {} from {addr}",
                    response.form_id
                );
                return;
            }
        };

        // Parse the response based on form type
        let form_response =
            mc_rs_plugin_api::parse_form_response(&form_type, response.response_data.as_deref());

        // Build plugin player and dispatch event
        let player = match self.connections.get(&addr) {
            Some(conn) => Self::make_plugin_player(conn),
            None => return,
        };

        let event = PluginEvent::FormResponse {
            player,
            form_id: response.form_id,
            response: form_response,
        };

        let snapshot = self.build_snapshot();
        let (_result, actions) = self.plugin_manager.dispatch(&event, &snapshot);
        self.apply_plugin_actions(actions).await;
    }

    // -----------------------------------------------------------------------
    // PlayerSkin (0x5D)
    // -----------------------------------------------------------------------

    async fn handle_player_skin(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let pkt = match packets::PlayerSkin::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                debug!("Bad PlayerSkin from {addr}: {e}");
                return;
            }
        };

        // Update the stored skin data for this player.
        if let Some(conn) = self.connections.get_mut(&addr) {
            if let Some(ref mut cd) = conn.client_data {
                cd.skin_id = pkt.skin_data.skin_id.clone();
                cd.skin_image = pkt.skin_data.skin_image.clone();
                cd.cape_id = pkt.skin_data.cape_id.clone();
                cd.cape_image = pkt.skin_data.cape_image.clone();
                cd.skin_resource_patch = pkt.skin_data.skin_resource_patch.clone();
                cd.skin_geometry_data = pkt.skin_data.skin_geometry_data.clone();
                cd.skin_color = pkt.skin_data.skin_color.clone();
                cd.arm_size = pkt.skin_data.arm_size.clone();
                cd.persona_skin = pkt.skin_data.persona_skin;
                cd.play_fab_id = pkt.skin_data.play_fab_id.clone();
            }
        }

        // Broadcast the PlayerSkin packet to all other players.
        let broadcast = packets::PlayerSkin {
            uuid: pkt.uuid,
            skin_data: pkt.skin_data,
            new_skin_name: pkt.new_skin_name,
            old_skin_name: pkt.old_skin_name,
        };
        self.broadcast_packet_except(addr, packets::id::PLAYER_SKIN, &broadcast)
            .await;
    }
}
