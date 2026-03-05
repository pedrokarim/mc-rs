use super::*;

impl ConnectionHandler {
    pub(super) async fn handle_request_chunk_radius(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let body_head = {
            let raw = buf.get_ref();
            let start = (buf.position() as usize).min(raw.len());
            hex_preview(&raw[start..], 48)
        };
        let state = match self.connections.get(&addr) {
            Some(c) if c.state == LoginState::Spawning || c.state == LoginState::InGame => c.state,
            _ => {
                debug!("RequestChunkRadius from {addr} in unexpected state, body_head={body_head}");
                return;
            }
        };

        let request = match RequestChunkRadius::proto_decode(buf) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Bad RequestChunkRadius from {addr}: {e} (state={state:?}, body_head={body_head})"
                );
                return;
            }
        };

        // Keep initial spawn burst small to avoid client-side loading stalls.
        // Once the player is already in-game, allow the normal runtime cap.
        let accepted_radius = if state == LoginState::Spawning {
            request.chunk_radius.clamp(1, 2)
        } else {
            request.chunk_radius.clamp(1, 8)
        };
        info!(
            "RequestChunkRadius from {addr}: requested={}, max={}, accepted={}, state={state:?}",
            request.chunk_radius, request.max_chunk_radius, accepted_radius
        );

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
            info!(
                "### BUILD_MARKER spawn-r10-2026-03-02: entering Spawning sequence for {addr} ###"
            );

            self.send_spawn_chunks(addr, accepted_radius).await;
            self.send_packet(
                addr,
                packets::id::PLAY_STATUS,
                &PlayStatus {
                    status: PlayStatusType::PlayerSpawn,
                },
            )
            .await;

            info!(
                "Sent spawn stream (PocketMine-like): ChunkRadiusUpdated({accepted_radius}) + NetworkChunkPublisherUpdate + initial chunks + PlayStatus(PlayerSpawn) to {addr}"
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
            conn.sent_chunks.clear();
        }

        let (dim, center_x, center_z) = self
            .connections
            .get(&addr)
            .map(|c| {
                (
                    c.dimension,
                    Self::chunk_coord(c.position.x),
                    Self::chunk_coord(c.position.z),
                )
            })
            .unwrap_or((0, 0, 0));

        // Tell the client which chunk area is publishable.
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
                position: player_block_pos.into(),
                radius: (radius * 16) as u32,
                saved_chunks: Vec::new(),
            },
        )
        .await;

        // Phase 1: Identify missing chunks, load from LevelDB, collect those needing generation
        let mut to_generate: Vec<(i32, i32)> = Vec::new();
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let cx = center_x + dx;
                let cz = center_z + dz;
                if self
                    .dim_chunks(dim)
                    .is_some_and(|m| m.contains_key(&(cx, cz)))
                {
                    continue;
                }
                if let Some(loaded) = self.chunk_storage.load_chunk_dim(cx, cz, dim) {
                    self.dim_chunks_mut(dim).insert((cx, cz), loaded);
                    let be_key = block_entity_key(cx, cz);
                    if let Some(be_data) = self.chunk_storage.get_raw(&be_key) {
                        let entries = block_entity::parse_block_entities(&be_data);
                        for ((bx, by, bz), data) in entries {
                            self.insert_block_entity_dim((bx, by, bz), dim, data);
                        }
                    }
                } else {
                    to_generate.push((cx, cz));
                }
            }
        }

        // Phase 2: Generate missing chunks in parallel via spawn_blocking
        if !to_generate.is_empty() {
            let gen_overworld = self.overworld_generator.clone();
            let gen_nether = self.nether_generator.clone();
            let gen_end = self.end_generator.clone();
            let flat_blocks = self.flat_world_blocks;

            let handles: Vec<_> = to_generate
                .iter()
                .map(|&(cx, cz)| {
                    let ow = gen_overworld.clone();
                    let neth = gen_nether.clone();
                    let end = gen_end.clone();
                    let fb = flat_blocks;
                    let d = dim;
                    tokio::task::spawn_blocking(move || {
                        let mut col = match d {
                            1 => {
                                if let Some(ref g) = neth {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                            2 => {
                                if let Some(ref g) = end {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                            _ => {
                                if let Some(ref g) = ow {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                        };
                        col.dirty = true;
                        (cx, cz, col)
                    })
                })
                .collect();

            for handle in handles {
                if let Ok((cx, cz, column)) = handle.await {
                    self.dim_chunks_mut(dim).insert((cx, cz), column);
                }
            }
        }

        // Phase 3: Serialize and send all chunks
        let sanitize_flat_chunks = dim == 0 && self.overworld_generator.is_none();
        let flat_blocks = self.flat_world_blocks;
        let mut suspicious_regen_count = 0u32;
        let mut sub_chunk_count_hist: std::collections::BTreeMap<u32, u32> =
            std::collections::BTreeMap::new();
        let mut count = 0u32;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let cx = center_x + dx;
                let cz = center_z + dz;
                let (sub_chunk_count, payload) = {
                    let column = self.dim_chunks_mut(dim).get_mut(&(cx, cz)).unwrap();
                    let mut encoded = serialize_chunk_column_cached(column);

                    // Flat-world safeguard: old persisted chunks from earlier serializers can look
                    // like 24 all-air sections (small payload) and confuse client loading.
                    if sanitize_flat_chunks && encoded.0 >= 20 && encoded.1.len() <= 256 {
                        warn!(
                            "Suspicious flat chunk payload at ({cx},{cz}) in dim={dim}: sub_chunks={}, payload={} bytes; regenerating chunk before send",
                            encoded.0,
                            encoded.1.len()
                        );
                        *column = generate_flat_chunk(cx, cz, &flat_blocks);
                        column.dirty = true;
                        column.cached_payload = None;
                        encoded = serialize_chunk_column_cached(column);
                        suspicious_regen_count += 1;
                    }

                    encoded
                };
                *sub_chunk_count_hist.entry(sub_chunk_count).or_insert(0) += 1;

                let level_chunk = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: dim,
                    sub_chunk_count,
                    cache_enabled: false,
                    payload: Bytes::from(payload),
                };

                if count == 0 {
                    info!(
                        "First LevelChunk ({cx},{cz}) around center ({center_x},{center_z}): sub_chunks={sub_chunk_count}, payload={} bytes",
                        level_chunk.payload.len()
                    );
                }
                self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                    .await;
                count += 1;

                // Send block entity data for this chunk (O(1) via chunk index)
                let be_keys = self.block_entities_in_chunk_dim(dim, cx, cz);
                for (bx, by, bz) in be_keys {
                    if let Some(be) = self.block_entities.get(&(bx, by, bz, dim)) {
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
        let hist = sub_chunk_count_hist
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(", ");
        info!(
            "Spawn chunk summary for {addr}: total={count}, sub_chunk_count_hist=[{hist}], regenerated_suspicious={suspicious_regen_count}"
        );
        debug!("Sent {count} LevelChunk packets to {addr}");
    }

    /// Convert a world coordinate (f32) to chunk coordinate.
    pub(super) fn chunk_coord(v: f32) -> i32 {
        v.floor() as i32 >> 4
    }

    /// Send new chunks around the player's current position that haven't been sent yet.
    pub(super) async fn send_new_chunks(&mut self, addr: SocketAddr) {
        let (center_x, center_z, radius, dim) = match self.connections.get(&addr) {
            Some(c) => {
                let cx = Self::chunk_coord(c.position.x);
                let cz = Self::chunk_coord(c.position.z);
                (cx, cz, c.chunk_radius, c.dimension)
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
                position: player_block_pos.into(),
                radius: (radius * 16) as u32,
                saved_chunks: Vec::new(),
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

        // Phase 1: Load from LevelDB, collect those needing generation
        let mut to_generate: Vec<(i32, i32)> = Vec::new();
        for &(cx, cz) in &to_send {
            if self
                .dim_chunks(dim)
                .is_some_and(|m| m.contains_key(&(cx, cz)))
            {
                continue;
            }
            if let Some(loaded) = self.chunk_storage.load_chunk_dim(cx, cz, dim) {
                self.dim_chunks_mut(dim).insert((cx, cz), loaded);
                let be_key = block_entity_key(cx, cz);
                if let Some(be_data) = self.chunk_storage.get_raw(&be_key) {
                    let entries = block_entity::parse_block_entities(&be_data);
                    for ((bx, by, bz), data) in entries {
                        self.insert_block_entity_dim((bx, by, bz), dim, data);
                    }
                }
            } else {
                to_generate.push((cx, cz));
            }
        }

        // Phase 2: Generate missing chunks in parallel (limited to 4 concurrent)
        if !to_generate.is_empty() {
            let gen_overworld = self.overworld_generator.clone();
            let gen_nether = self.nether_generator.clone();
            let gen_end = self.end_generator.clone();
            let flat_blocks = self.flat_world_blocks;

            // Limit to 4 parallel generations per tick
            let batch_size = to_generate.len().min(4);
            let batch = &to_generate[..batch_size];

            let handles: Vec<_> = batch
                .iter()
                .map(|&(cx, cz)| {
                    let ow = gen_overworld.clone();
                    let neth = gen_nether.clone();
                    let end = gen_end.clone();
                    let fb = flat_blocks;
                    let d = dim;
                    tokio::task::spawn_blocking(move || {
                        let mut col = match d {
                            1 => {
                                if let Some(ref g) = neth {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                            2 => {
                                if let Some(ref g) = end {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                            _ => {
                                if let Some(ref g) = ow {
                                    g.generate_chunk(cx, cz)
                                } else {
                                    generate_flat_chunk(cx, cz, &fb)
                                }
                            }
                        };
                        col.dirty = true;
                        (cx, cz, col)
                    })
                })
                .collect();

            for handle in handles {
                if let Ok((cx, cz, column)) = handle.await {
                    self.dim_chunks_mut(dim).insert((cx, cz), column);
                }
            }
        }

        // Phase 3: Send all ready chunks
        let sanitize_flat_chunks = dim == 0 && self.overworld_generator.is_none();
        let flat_blocks = self.flat_world_blocks;
        for &(cx, cz) in &to_send {
            if let Some(column) = self.dim_chunks_mut(dim).get_mut(&(cx, cz)) {
                let (sub_chunk_count, payload) = {
                    let mut encoded = serialize_chunk_column_cached(column);
                    if sanitize_flat_chunks && encoded.0 >= 20 && encoded.1.len() <= 256 {
                        warn!(
                            "Suspicious flat chunk payload at ({cx},{cz}) in dim={dim}: sub_chunks={}, payload={} bytes; regenerating chunk before send",
                            encoded.0,
                            encoded.1.len()
                        );
                        *column = generate_flat_chunk(cx, cz, &flat_blocks);
                        column.dirty = true;
                        column.cached_payload = None;
                        encoded = serialize_chunk_column_cached(column);
                    }
                    encoded
                };

                let level_chunk = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: dim,
                    sub_chunk_count,
                    cache_enabled: false,
                    payload: Bytes::from(payload),
                };

                self.send_packet(addr, packets::id::LEVEL_CHUNK, &level_chunk)
                    .await;

                // Send block entity data for this chunk (O(1) via chunk index)
                let be_keys = self.block_entities_in_chunk_dim(dim, cx, cz);
                for (bx, by, bz) in be_keys {
                    if let Some(be) = self.block_entities.get(&(bx, by, bz, dim)) {
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
        }

        // Mark as sent
        let loaded_keys: Vec<(i32, i32)> = to_send
            .iter()
            .filter(|key| self.dim_chunks(dim).is_some_and(|m| m.contains_key(key)))
            .copied()
            .collect();
        if let Some(conn) = self.connections.get_mut(&addr) {
            for key in loaded_keys {
                conn.sent_chunks.insert(key);
            }
        }

        debug!("Sent {} new LevelChunk packets to {addr}", to_send.len());
    }

    pub(super) async fn handle_set_local_player_as_initialized(
        &mut self,
        addr: SocketAddr,
        buf: &mut Cursor<&[u8]>,
    ) {
        let body_head = {
            let raw = buf.get_ref();
            let start = (buf.position() as usize).min(raw.len());
            hex_preview(&raw[start..], 48)
        };
        let state = self.connections.get(&addr).map(|c| c.state);
        match state {
            Some(LoginState::Spawning) => {}
            _ => {
                debug!(
                    "SetLocalPlayerAsInitialized from {addr} in unexpected state={state:?}, body_head={body_head}"
                );
                return;
            }
        }

        let packet = match SetLocalPlayerAsInitialized::proto_decode(buf) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Bad SetLocalPlayerAsInitialized from {addr}: {e} (state={state:?}, body_head={body_head})"
                );
                return;
            }
        };

        self.finalize_spawn_ready(
            addr,
            packet.entity_runtime_id,
            "SetLocalPlayerAsInitialized",
        )
        .await;
    }

    pub(super) async fn finalize_spawn_ready(
        &mut self,
        addr: SocketAddr,
        reported_runtime_id: u64,
        trigger: &str,
    ) {
        let state = self.connections.get(&addr).map(|c| c.state);
        if state != Some(LoginState::Spawning) {
            debug!("finalize_spawn_ready ignored for {addr}: trigger={trigger}, state={state:?}");
            return;
        }

        if let Some(conn) = self.connections.get(&addr) {
            if conn.entity_runtime_id != reported_runtime_id {
                warn!(
                    "Spawn ready runtime mismatch from {addr} via {trigger}: reported={}, expected={}",
                    reported_runtime_id, conn.entity_runtime_id
                );
            }
        }

        info!(
            "### BUILD_MARKER spawn-r12-2026-03-03: finalize_spawn_ready for {addr} via {trigger} (runtime_id={reported_runtime_id}) ###"
        );

        let initial_radius = if let Some(conn) = self.connections.get_mut(&addr) {
            let radius = conn.chunk_radius.clamp(1, 8);
            conn.state = LoginState::InGame;
            radius
        } else {
            8
        };

        // PocketMine parity: once spawn is acknowledged, force a local position sync
        // and explicit player spawnpoint to help clients exit loading UI.
        if let Some(conn) = self.connections.get(&addr) {
            let runtime_id = conn.entity_runtime_id;
            let position = conn.position;
            let pitch = conn.pitch;
            let yaw = conn.yaw;
            let head_yaw = conn.head_yaw;
            let on_ground = conn.on_ground;
            let dimension = conn.dimension;
            let player_spawn = BlockPos::new(
                position.x.floor() as i32,
                position.y.floor() as i32,
                position.z.floor() as i32,
            );
            let tick = self.game_world.current_tick();
            self.send_packet(
                addr,
                packets::id::SET_SPAWN_POSITION,
                &SetSpawnPosition {
                    spawn_type: 0, // TYPE_PLAYER_SPAWN
                    spawn_position: player_spawn,
                    dimension,
                    causing_block_position: player_spawn,
                },
            )
            .await;
            self.send_packet(
                addr,
                packets::id::MOVE_PLAYER,
                &MovePlayer::reset(
                    runtime_id, position, pitch, yaw, head_yaw, on_ground, tick,
                ),
            )
            .await;
            info!(
                "### BUILD_MARKER spawn-r13-2026-03-03: sent post-ready SetSpawnPosition(player)+MovePlayer(reset) to {addr} ###"
            );
        }

        // spawn-r10: pre-init spawn already sends the initial chunk stream;
        // post-init we only top up chunks around the current player position.
        self.send_new_chunks(addr).await;
        info!(
            "### BUILD_MARKER spawn-r10-2026-03-02: post-init chunk sync sent to {addr} (radius={initial_radius}) ###"
        );

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
        // 6. Sync active projectiles (arrows, tridents) to the new player
        self.sync_projectiles_to_player(addr).await;

        // 7. Send initial health + hunger + XP attributes so the client HUD shows correctly
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
            reported_runtime_id
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
                    color_argb: 0xFFFF_FFFF,
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
                color_argb: 0xFFFF_FFFF,
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
