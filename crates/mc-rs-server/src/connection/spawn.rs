use super::*;

impl ConnectionHandler {
    pub(super) async fn handle_request_chunk_radius(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::Spawning || c.state == LoginState::InGame => c.state,
            _ => {
                debug!("RequestChunkRadius from {addr} in unexpected state");
                return;
            }
        };

        let request = match RequestChunkRadius::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad RequestChunkRadius from {addr}: {e}");
                return;
            }
        };

        // Clamp to a reasonable server max (8 chunks)
        let accepted_radius = request.chunk_radius.clamp(1, 8);

        self.send_packet(
            addr,
            packets::id::CHUNK_RADIUS_UPDATED,
            &ChunkRadiusUpdated {
                chunk_radius: accepted_radius,
            },
        )
        .await;

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.chunk_radius = accepted_radius;
        }

        if state == LoginState::Spawning {
            // Spawn flow: send initial chunks around spawn
            self.send_packet(
                addr,
                packets::id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                &NetworkChunkPublisherUpdate {
                    position: self.spawn_block,
                    radius: (accepted_radius * 16) as u32,
                },
            )
            .await;

            self.send_spawn_chunks(addr, accepted_radius).await;

            // Tell the client it can spawn the player
            self.send_packet(
                addr,
                packets::id::PLAY_STATUS,
                &PlayStatus {
                    status: PlayStatusType::PlayerSpawn,
                },
            )
            .await;

            // Send initial inventory contents
            self.send_inventory(addr).await;

            info!(
                "Sent ChunkRadiusUpdated({accepted_radius}) + {} chunks + PlayStatus(PlayerSpawn) + inventory to {addr}",
                (accepted_radius * 2 + 1) * (accepted_radius * 2 + 1)
            );
        } else {
            // In-game render distance change: send new chunks around current position
            self.send_new_chunks(addr).await;
            info!("Updated chunk radius to {accepted_radius} for {addr}");
        }
    }

    async fn send_spawn_chunks(&mut self, addr: SocketAddr, radius: i32) {
        // Store chunk_radius on the connection
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.chunk_radius = radius;
        }

        let mut count = 0u32;
        for cx in -radius..=radius {
            for cz in -radius..=radius {
                // Load from disk or generate chunk if not already cached
                if !self.world_chunks.contains_key(&(cx, cz)) {
                    let column = if let Some(loaded) = self.chunk_storage.load_chunk(cx, cz) {
                        loaded
                    } else {
                        let mut gen = self.generate_chunk(cx, cz);
                        gen.dirty = true;
                        gen
                    };
                    self.world_chunks.insert((cx, cz), column);

                    // Load block entities for this chunk from LevelDB
                    let be_key = block_entity_key(cx, cz);
                    if let Some(be_data) = self.chunk_storage.get_raw(&be_key) {
                        let entries = block_entity::parse_block_entities(&be_data);
                        for ((bx, by, bz), data) in entries {
                            self.block_entities.insert((bx, by, bz), data);
                        }
                    }
                }

                let column = self.world_chunks.get(&(cx, cz)).unwrap();
                let (sub_chunk_count, payload) = serialize_chunk_column(column);

                let level_chunk = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: 0,
                    sub_chunk_count,
                    cache_enabled: false,
                    payload: Bytes::from(payload),
                };

                self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                    .await;
                count += 1;

                // Send block entity data for this chunk
                let be_keys: Vec<(i32, i32, i32)> = self
                    .block_entities
                    .keys()
                    .filter(|&&(bx, _, bz)| bx >> 4 == cx && bz >> 4 == cz)
                    .cloned()
                    .collect();
                for (bx, by, bz) in be_keys {
                    if let Some(be) = self.block_entities.get(&(bx, by, bz)) {
                        let nbt = be.to_network_nbt(bx, by, bz);
                        self.send_packet(
                            addr,
                            packets::id::BLOCK_ACTOR_DATA,
                            &BlockActorData {
                                position: BlockPos::new(bx, by, bz),
                                nbt_data: nbt,
                            },
                        )
                        .await;
                    }
                }

                // Track sent chunk
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.sent_chunks.insert((cx, cz));
                }
            }
        }
        debug!("Sent {count} LevelChunk packets to {addr}");
    }

    /// Convert a world coordinate (f32) to chunk coordinate.
    pub(super) fn chunk_coord(v: f32) -> i32 {
        v.floor() as i32 >> 4
    }

    /// Send new chunks around the player's current position that haven't been sent yet.
    pub(super) async fn send_new_chunks(&mut self, addr: SocketAddr) {
        let (center_x, center_z, radius) = match self.connections.get(&addr) {
            Some(c) => {
                let cx = Self::chunk_coord(c.position.x);
                let cz = Self::chunk_coord(c.position.z);
                (cx, cz, c.chunk_radius)
            }
            None => return,
        };

        // Send NetworkChunkPublisherUpdate with the player's block position
        let player_block_pos = match self.connections.get(&addr) {
            Some(c) => BlockPos::new(
                c.position.x.floor() as i32,
                c.position.y.floor() as i32,
                c.position.z.floor() as i32,
            ),
            None => return,
        };
        self.send_packet(
            addr,
            packets::id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &NetworkChunkPublisherUpdate {
                position: player_block_pos,
                radius: (radius * 16) as u32,
            },
        )
        .await;

        // Find new chunks to send
        let mut to_send = Vec::new();
        for cx in (center_x - radius)..=(center_x + radius) {
            for cz in (center_z - radius)..=(center_z + radius) {
                let key = (cx, cz);
                let already_sent = self
                    .connections
                    .get(&addr)
                    .map(|c| c.sent_chunks.contains(&key))
                    .unwrap_or(true);
                if !already_sent {
                    to_send.push(key);
                }
            }
        }

        if to_send.is_empty() {
            return;
        }

        // Generate and send new chunks
        for &(cx, cz) in &to_send {
            if !self.world_chunks.contains_key(&(cx, cz)) {
                let column = if let Some(loaded) = self.chunk_storage.load_chunk(cx, cz) {
                    loaded
                } else {
                    let mut gen = self.generate_chunk(cx, cz);
                    gen.dirty = true;
                    gen
                };
                self.world_chunks.insert((cx, cz), column);

                // Load block entities for this chunk from LevelDB
                let be_key = block_entity_key(cx, cz);
                if let Some(be_data) = self.chunk_storage.get_raw(&be_key) {
                    let entries = block_entity::parse_block_entities(&be_data);
                    for ((bx, by, bz), data) in entries {
                        self.block_entities.insert((bx, by, bz), data);
                    }
                }
            }

            let column = self.world_chunks.get(&(cx, cz)).unwrap();
            let (sub_chunk_count, payload) = serialize_chunk_column(column);

            let level_chunk = LevelChunk {
                chunk_x: cx,
                chunk_z: cz,
                dimension_id: 0,
                sub_chunk_count,
                cache_enabled: false,
                payload: Bytes::from(payload),
            };

            self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                .await;

            // Send block entity data for this chunk
            let be_keys: Vec<(i32, i32, i32)> = self
                .block_entities
                .keys()
                .filter(|&&(bx, _, bz)| bx >> 4 == cx && bz >> 4 == cz)
                .cloned()
                .collect();
            for (bx, by, bz) in be_keys {
                if let Some(be) = self.block_entities.get(&(bx, by, bz)) {
                    let nbt = be.to_network_nbt(bx, by, bz);
                    self.send_packet(
                        addr,
                        packets::id::BLOCK_ACTOR_DATA,
                        &BlockActorData {
                            position: BlockPos::new(bx, by, bz),
                            nbt_data: nbt,
                        },
                    )
                    .await;
                }
            }
        }

        // Mark as sent
        if let Some(conn) = self.connections.get_mut(&addr) {
            for key in &to_send {
                conn.sent_chunks.insert(*key);
            }
        }

        debug!("Sent {} new LevelChunk packets to {addr}", to_send.len());
    }

    pub(super) async fn handle_set_local_player_as_initialized(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::Spawning => {}
            _ => {
                debug!("SetLocalPlayerAsInitialized from {addr} in unexpected state");
                return;
            }
        }

        let packet = match SetLocalPlayerAsInitialized::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!("Bad SetLocalPlayerAsInitialized from {addr}: {e}");
                return;
            }
        };

        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = LoginState::InGame;
        }

        // --- Multi-player: send PlayerList + AddPlayer ---
        // 1. Send PlayerList(Add) with all existing InGame players to the new player
        self.send_existing_players_to(addr).await;
        // 2. Broadcast PlayerList(Add) for the new player to everyone (including self for tab list)
        self.broadcast_new_player_list(addr).await;
        // 3. Send AddPlayer for each existing InGame player to the new player
        self.send_existing_add_players_to(addr).await;
        // 4. Broadcast AddPlayer for the new player to all existing InGame players
        self.broadcast_add_player(addr).await;
        // 5. Send AddActor for all existing mobs to the new player
        self.send_existing_mobs_to(addr).await;

        // 6. Send initial health + hunger + XP attributes so the client HUD shows correctly
        let (rid, hp, food, sat, exh, xl, xt) = match self.connections.get(&addr) {
            Some(c) => (
                c.entity_runtime_id,
                c.health,
                c.food as f32,
                c.saturation,
                c.exhaustion,
                c.xp_level,
                c.xp_total,
            ),
            None => return,
        };
        let xp_progress = xp::xp_progress(xl, xt);
        self.send_packet(
            addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::all(rid, hp, food, sat, exh, xl, xp_progress, 0),
        )
        .await;

        let name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_default();

        // 7. Send current time
        self.send_packet(
            addr,
            packets::id::SET_TIME,
            &SetTime {
                time: self.world_time as i32,
            },
        )
        .await;

        // 8. Sync current weather state
        if self.is_raining {
            self.send_packet(addr, packets::id::LEVEL_EVENT, &LevelEvent::start_rain())
                .await;
        }
        if self.is_thundering {
            self.send_packet(addr, packets::id::LEVEL_EVENT, &LevelEvent::start_thunder())
                .await;
        }

        // 9. Broadcast join message
        let join_msg = Text::system(format!("{name} joined the game"));
        self.broadcast_packet(packets::id::TEXT, &join_msg).await;

        // 10. Plugin event: PlayerJoin
        if let Some(conn) = self.connections.get(&addr) {
            let player = Self::make_plugin_player(conn);
            let event = PluginEvent::PlayerJoin { player };
            let snapshot = self.build_snapshot();
            let (_, actions) = self.plugin_manager.dispatch(&event, &snapshot);
            self.apply_plugin_actions(actions).await;
        }

        info!(
            "Player {name} is now in-game ({addr}, runtime_id={})",
            packet.entity_runtime_id
        );
    }

    /// Remove chunks from `sent_chunks` that are outside the player's view radius.
    /// The client handles visual unloading via `NetworkChunkPublisherUpdate.radius`,
    /// this just prevents the tracking `HashSet` from growing indefinitely.
    pub(super) fn cleanup_sent_chunks(&mut self, addr: SocketAddr) {
        let (center_x, center_z, radius) = match self.connections.get(&addr) {
            Some(c) => (
                Self::chunk_coord(c.position.x),
                Self::chunk_coord(c.position.z),
                c.chunk_radius,
            ),
            None => return,
        };

        // Keep a margin of 2 chunks beyond the render radius
        let unload_radius = radius + 2;
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.sent_chunks.retain(|&(cx, cz)| {
                (cx - center_x).abs() <= unload_radius && (cz - center_z).abs() <= unload_radius
            });
        }
    }

    /// Send PlayerList(Add) with all existing InGame players to a newly joined player.
    async fn send_existing_players_to(&mut self, new_addr: SocketAddr) {
        let entries: Vec<PlayerListAdd> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != new_addr && c.state == LoginState::InGame)
            .filter_map(|(_, conn)| {
                let login = conn.login_data.as_ref()?;
                let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
                let client_data = conn.client_data.clone().unwrap_or_default();
                Some(PlayerListAdd {
                    uuid,
                    entity_unique_id: conn.entity_unique_id,
                    username: login.display_name.clone(),
                    xuid: login.xuid.clone(),
                    platform_chat_id: String::new(),
                    device_os: client_data.device_os,
                    skin_data: client_data,
                    is_teacher: false,
                    is_host: false,
                    is_sub_client: false,
                })
            })
            .collect();

        if !entries.is_empty() {
            self.send_packet(
                new_addr,
                packets::id::PLAYER_LIST,
                &PlayerListAddPacket { entries },
            )
            .await;
        }
    }

    /// Broadcast PlayerList(Add) for the new player to all InGame players (including self).
    async fn broadcast_new_player_list(&mut self, new_addr: SocketAddr) {
        let entry = {
            let conn = match self.connections.get(&new_addr) {
                Some(c) => c,
                None => return,
            };
            let login = match &conn.login_data {
                Some(d) => d,
                None => return,
            };
            let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
            let client_data = conn.client_data.clone().unwrap_or_default();
            PlayerListAdd {
                uuid,
                entity_unique_id: conn.entity_unique_id,
                username: login.display_name.clone(),
                xuid: login.xuid.clone(),
                platform_chat_id: String::new(),
                device_os: client_data.device_os,
                skin_data: client_data,
                is_teacher: false,
                is_host: false,
                is_sub_client: false,
            }
        };
        let packet = PlayerListAddPacket {
            entries: vec![entry],
        };
        self.broadcast_packet(packets::id::PLAYER_LIST, &packet)
            .await;
    }

    /// Send AddPlayer for each existing InGame player to a newly joined player.
    async fn send_existing_add_players_to(&mut self, new_addr: SocketAddr) {
        let ops = &self.permissions.ops;
        let players: Vec<AddPlayer> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != new_addr && c.state == LoginState::InGame)
            .filter_map(|(_, conn)| {
                let login = conn.login_data.as_ref()?;
                let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
                let client_data = conn.client_data.clone().unwrap_or_default();
                let held_item = conn
                    .inventory
                    .get_slot(0, conn.inventory.held_slot)
                    .cloned()
                    .unwrap_or_else(mc_rs_proto::item_stack::ItemStack::empty);
                let is_op = ops.contains(&login.display_name);
                Some(AddPlayer {
                    uuid,
                    username: login.display_name.clone(),
                    entity_runtime_id: conn.entity_runtime_id,
                    platform_chat_id: String::new(),
                    position: conn.position,
                    velocity: Vec3::ZERO,
                    pitch: conn.pitch,
                    yaw: conn.yaw,
                    head_yaw: conn.head_yaw,
                    held_item,
                    gamemode: conn.gamemode,
                    metadata: default_player_metadata(&login.display_name),
                    entity_unique_id: conn.entity_unique_id,
                    permission_level: if is_op { 2 } else { 1 },
                    command_permission_level: if is_op { 1 } else { 0 },
                    device_id: client_data.device_id,
                    device_os: client_data.device_os,
                })
            })
            .collect();

        for player in &players {
            self.send_packet(new_addr, packets::id::ADD_PLAYER, player)
                .await;
        }
    }

    /// Send AddActor for all existing mobs to a newly joined player.
    async fn send_existing_mobs_to(&mut self, addr: SocketAddr) {
        let mobs = self.game_world.all_mobs();
        for mob in mobs {
            let metadata = if mob.is_baby {
                baby_mob_metadata(mob.bb_width, mob.bb_height)
            } else {
                default_mob_metadata(mob.bb_width, mob.bb_height)
            };
            let pkt = AddActor {
                entity_unique_id: mob.unique_id,
                entity_runtime_id: mob.runtime_id,
                entity_type: mob.mob_type,
                position: Vec3::new(mob.position.0, mob.position.1, mob.position.2),
                velocity: Vec3::ZERO,
                pitch: mob.pitch,
                yaw: mob.yaw,
                head_yaw: mob.head_yaw,
                body_yaw: mob.yaw,
                attributes: vec![ActorAttribute {
                    name: "minecraft:health".to_string(),
                    min: 0.0,
                    max: mob.max_health,
                    current: mob.health,
                    default: mob.max_health,
                }],
                metadata,
            };
            self.send_packet(addr, packets::id::ADD_ACTOR, &pkt).await;
        }
    }

    /// Broadcast AddPlayer for the new player to all existing InGame players.
    async fn broadcast_add_player(&mut self, new_addr: SocketAddr) {
        let packet = {
            let conn = match self.connections.get(&new_addr) {
                Some(c) => c,
                None => return,
            };
            let login = match &conn.login_data {
                Some(d) => d,
                None => return,
            };
            let uuid = Uuid::parse(&login.identity).unwrap_or(Uuid::ZERO);
            let client_data = conn.client_data.clone().unwrap_or_default();
            let held_item = conn
                .inventory
                .get_slot(0, conn.inventory.held_slot)
                .cloned()
                .unwrap_or_else(mc_rs_proto::item_stack::ItemStack::empty);
            let is_op = self.permissions.ops.contains(&login.display_name);
            AddPlayer {
                uuid,
                username: login.display_name.clone(),
                entity_runtime_id: conn.entity_runtime_id,
                platform_chat_id: String::new(),
                position: conn.position,
                velocity: Vec3::ZERO,
                pitch: conn.pitch,
                yaw: conn.yaw,
                head_yaw: conn.head_yaw,
                held_item,
                gamemode: conn.gamemode,
                metadata: default_player_metadata(&login.display_name),
                entity_unique_id: conn.entity_unique_id,
                permission_level: if is_op { 2 } else { 1 },
                command_permission_level: if is_op { 1 } else { 0 },
                device_id: client_data.device_id,
                device_os: client_data.device_os,
            }
        };
        self.broadcast_packet_except(new_addr, packets::id::ADD_PLAYER, &packet)
            .await;
    }
}
