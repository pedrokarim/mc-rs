use super::*;

/// Axis along which a Nether portal frame is oriented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalAxis {
    X,
    Z,
}

/// Describes a detected Nether portal frame.
#[derive(Debug, Clone)]
pub struct PortalFrame {
    /// Bottom-left interior block (min x/z, min y).
    pub min: (i32, i32, i32),
    /// Top-right interior block (max x/z, max y).
    pub max: (i32, i32, i32),
    /// Axis the portal frame runs along.
    pub axis: PortalAxis,
}

impl ConnectionHandler {
    // -----------------------------------------------------------------------
    // Phase 5.3: Portal detection and dimension travel
    // -----------------------------------------------------------------------

    /// Portal cooldown in ticks (4 seconds).
    const PORTAL_COOLDOWN_TICKS: u64 = 80;

    /// Nether coordinate scaling factor.
    const NETHER_SCALE: f32 = 8.0;

    /// Search radius (in blocks) when looking for an existing portal in the target dimension.
    const PORTAL_SEARCH_RADIUS: i32 = 128;

    /// Fixed End spawn position (feet Y).
    const END_SPAWN_Y: i32 = 49;

    // -----------------------------------------------------------------------
    // Portal frame detection
    // -----------------------------------------------------------------------

    /// Detect a Nether portal frame around position (x, y, z).
    /// The position should be inside the frame (air or fire block).
    /// Returns the frame description if a valid frame is found.
    pub(super) fn detect_nether_portal_frame(
        &self,
        dim: i32,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<PortalFrame> {
        let obsidian = self.tick_blocks.obsidian;

        // Try both axes
        for axis in [PortalAxis::X, PortalAxis::Z] {
            if let Some(frame) = self.try_detect_frame(dim, x, y, z, obsidian, axis) {
                return Some(frame);
            }
        }
        None
    }

    /// Try detecting a portal frame along a specific axis.
    fn try_detect_frame(
        &self,
        dim: i32,
        x: i32,
        y: i32,
        z: i32,
        obsidian: u32,
        axis: PortalAxis,
    ) -> Option<PortalFrame> {
        let air = self.flat_world_blocks.air;
        let fire = self.tick_blocks.fire;

        // Helper: get the coordinate along the portal axis
        let get_along = |bx: i32, bz: i32| -> i32 {
            match axis {
                PortalAxis::X => bx,
                PortalAxis::Z => bz,
            }
        };

        let make_pos = |along: i32, by: i32| -> (i32, i32, i32) {
            match axis {
                PortalAxis::X => (along, by, z),
                PortalAxis::Z => (x, by, along),
            }
        };

        let along = get_along(x, z);

        // Find bottom: go down until we hit obsidian
        let mut bottom_y = y;
        for dy in 1..=21 {
            let check_y = y - dy;
            let (cx, cy, cz) = make_pos(along, check_y);
            match self.get_block_in(dim, cx, cy, cz) {
                Some(rid) if rid == obsidian => {
                    bottom_y = check_y + 1;
                    break;
                }
                Some(rid)
                    if rid == air || rid == fire || self.tick_blocks.is_nether_portal(rid) =>
                {
                    continue;
                }
                _ => return None,
            }
        }
        if bottom_y == y && y > 0 {
            // Check block below starting position
            let (cx, cy, cz) = make_pos(along, y - 1);
            match self.get_block_in(dim, cx, cy, cz) {
                Some(rid) if rid == obsidian => {
                    bottom_y = y;
                }
                _ => return None,
            }
        }

        // Find left edge: go negative along axis until obsidian
        let mut min_along = along;
        for d in 1..=21 {
            let test = along - d;
            let (cx, cy, cz) = make_pos(test, bottom_y);
            match self.get_block_in(dim, cx, cy, cz) {
                Some(rid) if rid == obsidian => {
                    min_along = test + 1;
                    break;
                }
                Some(rid)
                    if rid == air || rid == fire || self.tick_blocks.is_nether_portal(rid) =>
                {
                    continue;
                }
                _ => return None,
            }
        }

        // Find right edge: go positive along axis until obsidian
        let mut max_along = along;
        for d in 1..=21 {
            let test = along + d;
            let (cx, cy, cz) = make_pos(test, bottom_y);
            match self.get_block_in(dim, cx, cy, cz) {
                Some(rid) if rid == obsidian => {
                    max_along = test - 1;
                    break;
                }
                Some(rid)
                    if rid == air || rid == fire || self.tick_blocks.is_nether_portal(rid) =>
                {
                    continue;
                }
                _ => return None,
            }
        }

        let width = max_along - min_along + 1;
        if !(2..=21).contains(&width) {
            return None;
        }

        // Find top: go up until obsidian
        let mut top_y = bottom_y;
        for dy in 1..=21 {
            let check_y = bottom_y + dy;
            let (cx, cy, cz) = make_pos(min_along, check_y);
            match self.get_block_in(dim, cx, cy, cz) {
                Some(rid) if rid == obsidian => {
                    top_y = check_y - 1;
                    break;
                }
                Some(rid)
                    if rid == air || rid == fire || self.tick_blocks.is_nether_portal(rid) =>
                {
                    continue;
                }
                _ => return None,
            }
        }

        let height = top_y - bottom_y + 1;
        if !(3..=21).contains(&height) {
            return None;
        }

        // Validate the full frame:
        // 1. Bottom row of obsidian
        for a in min_along..=max_along {
            let (cx, cy, cz) = make_pos(a, bottom_y - 1);
            if self.get_block_in(dim, cx, cy, cz) != Some(obsidian) {
                return None;
            }
        }

        // 2. Top row of obsidian
        for a in min_along..=max_along {
            let (cx, cy, cz) = make_pos(a, top_y + 1);
            if self.get_block_in(dim, cx, cy, cz) != Some(obsidian) {
                return None;
            }
        }

        // 3. Left column of obsidian
        for by in bottom_y..=top_y {
            let (cx, cy, cz) = make_pos(min_along - 1, by);
            if self.get_block_in(dim, cx, cy, cz) != Some(obsidian) {
                return None;
            }
        }

        // 4. Right column of obsidian
        for by in bottom_y..=top_y {
            let (cx, cy, cz) = make_pos(max_along + 1, by);
            if self.get_block_in(dim, cx, cy, cz) != Some(obsidian) {
                return None;
            }
        }

        // 5. Interior must be air or fire or portal
        for a in min_along..=max_along {
            for by in bottom_y..=top_y {
                let (cx, cy, cz) = make_pos(a, by);
                match self.get_block_in(dim, cx, cy, cz) {
                    Some(rid)
                        if rid == air || rid == fire || self.tick_blocks.is_nether_portal(rid) => {}
                    _ => return None,
                }
            }
        }

        let (min_pos, max_pos) = match axis {
            PortalAxis::X => ((min_along, bottom_y, z), (max_along, top_y, z)),
            PortalAxis::Z => ((x, bottom_y, min_along), (x, top_y, max_along)),
        };

        Some(PortalFrame {
            min: min_pos,
            max: max_pos,
            axis,
        })
    }

    /// Fill the interior of a portal frame with nether portal blocks and broadcast updates.
    pub(super) async fn fill_portal_interior(&mut self, frame: &PortalFrame) {
        let portal_rid = match frame.axis {
            PortalAxis::X => self.tick_blocks.nether_portal_x,
            PortalAxis::Z => self.tick_blocks.nether_portal_z,
        };

        // Determine the dimension from context (we always fill in the current player's dimension)
        // The caller should ensure the correct dimension is used
        match frame.axis {
            PortalAxis::X => {
                let z = frame.min.2;
                for bx in frame.min.0..=frame.max.0 {
                    for by in frame.min.1..=frame.max.1 {
                        self.set_block_and_broadcast(bx, by, z, portal_rid).await;
                    }
                }
            }
            PortalAxis::Z => {
                let x = frame.min.0;
                for bz in frame.min.2..=frame.max.2 {
                    for by in frame.min.1..=frame.max.1 {
                        self.set_block_and_broadcast(x, by, bz, portal_rid).await;
                    }
                }
            }
        }
    }

    /// Fill the interior of a portal frame in a specific dimension.
    #[allow(dead_code)]
    pub(super) async fn fill_portal_interior_dim(&mut self, frame: &PortalFrame, dim: i32) {
        let portal_rid = match frame.axis {
            PortalAxis::X => self.tick_blocks.nether_portal_x,
            PortalAxis::Z => self.tick_blocks.nether_portal_z,
        };

        match frame.axis {
            PortalAxis::X => {
                let z = frame.min.2;
                for bx in frame.min.0..=frame.max.0 {
                    for by in frame.min.1..=frame.max.1 {
                        if self.set_block_in(dim, bx, by, z, portal_rid) {
                            let pos = BlockPos::new(bx, by, z);
                            self.broadcast_packet_in_dimension(
                                dim,
                                packets::id::UPDATE_BLOCK,
                                &UpdateBlock::new(pos, portal_rid),
                            )
                            .await;
                        }
                    }
                }
            }
            PortalAxis::Z => {
                let x = frame.min.0;
                for bz in frame.min.2..=frame.max.2 {
                    for by in frame.min.1..=frame.max.1 {
                        if self.set_block_in(dim, x, by, bz, portal_rid) {
                            let pos = BlockPos::new(x, by, bz);
                            self.broadcast_packet_in_dimension(
                                dim,
                                packets::id::UPDATE_BLOCK,
                                &UpdateBlock::new(pos, portal_rid),
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Portal detection in movement
    // -----------------------------------------------------------------------

    /// Check if a player is standing in a portal block and start dimension change if needed.
    pub(super) async fn check_portal_at_player(&mut self, addr: SocketAddr) {
        let (dim, pos, cooldown, tick) = match self.connections.get(&addr) {
            Some(c) => (
                c.dimension,
                c.position,
                c.portal_cooldown_until,
                c.client_tick,
            ),
            None => return,
        };

        // Check cooldown
        if tick < cooldown {
            return;
        }

        // Check blocks at feet and head
        const PLAYER_EYE_HEIGHT: f32 = 1.62;
        let feet_y = (pos.y - PLAYER_EYE_HEIGHT).floor() as i32;
        let head_y = pos.y.floor() as i32;
        let bx = pos.x.floor() as i32;
        let bz = pos.z.floor() as i32;

        for check_y in [feet_y, head_y] {
            if let Some(rid) = self.get_block_in(dim, bx, check_y, bz) {
                if self.tick_blocks.is_nether_portal(rid) {
                    self.start_nether_portal_travel(addr).await;
                    return;
                }
                if self.tick_blocks.is_end_portal(rid) {
                    self.start_end_portal_travel(addr).await;
                    return;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Dimension change logic
    // -----------------------------------------------------------------------

    /// Start traveling through a Nether portal.
    async fn start_nether_portal_travel(&mut self, addr: SocketAddr) {
        let (src_dim, src_pos, entity_unique_id) = match self.connections.get(&addr) {
            Some(c) => (c.dimension, c.position, c.entity_unique_id),
            None => return,
        };

        let (target_dim, target_x, target_z) = match src_dim {
            0 => {
                // Overworld → Nether: divide coords by 8
                (
                    1,
                    src_pos.x / Self::NETHER_SCALE,
                    src_pos.z / Self::NETHER_SCALE,
                )
            }
            1 => {
                // Nether → Overworld: multiply coords by 8
                (
                    0,
                    src_pos.x * Self::NETHER_SCALE,
                    src_pos.z * Self::NETHER_SCALE,
                )
            }
            _ => return, // End portals don't use nether portal blocks
        };

        // Find a suitable Y in the target dimension
        let target_y =
            self.find_safe_y(target_dim, target_x.floor() as i32, target_z.floor() as i32);
        let target_pos = Vec3::new(
            target_x.floor() + 0.5,
            target_y as f32 + 1.62,
            target_z.floor() + 0.5,
        );

        self.execute_dimension_change(addr, src_dim, target_dim, target_pos, entity_unique_id)
            .await;

        // Try to find or create a portal at the destination
        let dest_x = target_pos.x.floor() as i32;
        let dest_z = target_pos.z.floor() as i32;
        let dest_y = (target_pos.y - 1.62).floor() as i32;
        if self
            .find_nearest_portal(target_dim, dest_x, dest_y, dest_z)
            .is_none()
        {
            self.create_nether_portal(target_dim, dest_x, dest_y, dest_z)
                .await;
        }
    }

    /// Start traveling through an End portal.
    async fn start_end_portal_travel(&mut self, addr: SocketAddr) {
        let (src_dim, entity_unique_id) = match self.connections.get(&addr) {
            Some(c) => (c.dimension, c.entity_unique_id),
            None => return,
        };

        let (target_dim, target_pos) = match src_dim {
            0 => {
                // Overworld → End: fixed spawn at (0.5, 49+1.62, 0.5)
                let end_y = Self::END_SPAWN_Y as f32 + 1.62;
                (2, Vec3::new(0.5, end_y, 0.5))
            }
            2 => {
                // End → Overworld: return to world spawn
                (0, self.spawn_position)
            }
            _ => return,
        };

        // Create End platform if going to End
        if target_dim == 2 {
            self.create_end_platform().await;
        }

        self.execute_dimension_change(addr, src_dim, target_dim, target_pos, entity_unique_id)
            .await;
    }

    /// Execute the full dimension change flow.
    async fn execute_dimension_change(
        &mut self,
        addr: SocketAddr,
        src_dim: i32,
        target_dim: i32,
        target_pos: Vec3,
        entity_unique_id: i64,
    ) {
        let current_tick = self.game_world.current_tick();

        // Set portal cooldown
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.portal_cooldown_until = current_tick + Self::PORTAL_COOLDOWN_TICKS;
        }

        // 1. Send RemoveEntity to players in the source dimension
        let remove = RemoveEntity { entity_unique_id };
        self.broadcast_packet_in_dimension_except(
            src_dim,
            addr,
            packets::id::REMOVE_ENTITY,
            &remove,
        )
        .await;

        // 2. Send ChangeDimension to the traveling player
        let change_dim = packets::ChangeDimension {
            dimension: target_dim,
            position: target_pos,
            respawn: false,
        };
        self.send_packet(addr, packets::id::CHANGE_DIMENSION, &change_dim)
            .await;

        // 3. Update player state
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.dimension = target_dim;
            conn.position = target_pos;
            conn.sent_chunks.clear();
            conn.fall_distance = 0.0;
            conn.airborne_ticks = 0;
        }

        // 4. Send chunks in the new dimension
        self.send_new_chunks(addr).await;

        // 5. Send PlayStatus(PlayerSpawn) to confirm dimension change
        self.send_packet(
            addr,
            packets::id::PLAY_STATUS,
            &PlayStatus {
                status: PlayStatusType::PlayerSpawn,
            },
        )
        .await;

        // 6. Broadcast AddPlayer to players in the target dimension
        self.broadcast_add_player_to_dimension(addr, target_dim)
            .await;

        // 7. Sync ECS mirror entity position
        self.game_world.update_player_position(
            entity_unique_id,
            target_pos.x,
            target_pos.y,
            target_pos.z,
        );

        let name = self
            .connections
            .get(&addr)
            .and_then(|c| c.login_data.as_ref())
            .map(|d| d.display_name.clone())
            .unwrap_or_default();
        debug!("Player {name} traveled from dimension {src_dim} to {target_dim}");
    }

    /// Broadcast AddPlayer for a player entering a new dimension.
    async fn broadcast_add_player_to_dimension(&mut self, addr: SocketAddr, dim: i32) {
        let (add_player, list_add) = match self.connections.get(&addr) {
            Some(conn) => {
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

                let ap = AddPlayer {
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
                    device_id: client_data.device_id.clone(),
                    device_os: client_data.device_os,
                };

                let la = PlayerListAddPacket {
                    entries: vec![PlayerListAdd {
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
                    }],
                };

                (ap, la)
            }
            None => return,
        };

        // Send to all players in the target dimension (except the traveling player)
        let targets: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(&a, c)| a != addr && c.state == LoginState::InGame && c.dimension == dim)
            .map(|(&a, _)| a)
            .collect();

        for target_addr in targets {
            self.send_packet(target_addr, packets::id::PLAYER_LIST, &list_add)
                .await;
            self.send_packet(target_addr, packets::id::ADD_PLAYER, &add_player)
                .await;
        }
    }

    // -----------------------------------------------------------------------
    // Portal search & creation
    // -----------------------------------------------------------------------

    /// Find the nearest portal block in a dimension within search radius.
    fn find_nearest_portal(&self, dim: i32, x: i32, y: i32, z: i32) -> Option<(i32, i32, i32)> {
        let chunk_radius = Self::PORTAL_SEARCH_RADIUS >> 4;
        let cx = x >> 4;
        let cz = z >> 4;

        let dim_map = self.world_chunks.get(&dim)?;

        let mut best: Option<(i32, i32, i32)> = None;
        let mut best_dist_sq = i64::MAX;

        for dcx in -chunk_radius..=chunk_radius {
            for dcz in -chunk_radius..=chunk_radius {
                let ccx = cx + dcx;
                let ccz = cz + dcz;
                if let Some(column) = dim_map.get(&(ccx, ccz)) {
                    // Scan the chunk for portal blocks
                    for si in 0..OVERWORLD_SUB_CHUNK_COUNT {
                        let sub = &column.sub_chunks[si];
                        for lx in 0..16 {
                            for ly in 0..16 {
                                for lz in 0..16 {
                                    let rid = sub.get_block(lx, ly, lz);
                                    if self.tick_blocks.is_nether_portal(rid) {
                                        let wx = ccx * 16 + lx as i32;
                                        let wy = OVERWORLD_MIN_Y + si as i32 * 16 + ly as i32;
                                        let wz = ccz * 16 + lz as i32;
                                        let dx = (wx - x) as i64;
                                        let dy = (wy - y) as i64;
                                        let dz = (wz - z) as i64;
                                        let dist_sq = dx * dx + dy * dy + dz * dz;
                                        if dist_sq < best_dist_sq {
                                            best_dist_sq = dist_sq;
                                            best = Some((wx, wy, wz));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        best
    }

    /// Create a minimal 4×5 Nether portal frame at the given position.
    async fn create_nether_portal(&mut self, dim: i32, x: i32, y: i32, z: i32) {
        let obsidian = self.tick_blocks.obsidian;
        let portal_rid = self.tick_blocks.nether_portal_x;

        // Clamp Y to valid range
        let base_y = y.clamp(OVERWORLD_MIN_Y + 1, Self::MAX_Y - 5);

        // Find solid ground nearby
        let base_y = self.find_ground_y(dim, x, base_y, z);

        // Build a 4-wide × 5-tall obsidian frame along X axis
        // Bottom row
        for dx in 0..4 {
            self.set_block_in_and_broadcast_dim(dim, x + dx, base_y, z, obsidian)
                .await;
        }
        // Top row
        for dx in 0..4 {
            self.set_block_in_and_broadcast_dim(dim, x + dx, base_y + 4, z, obsidian)
                .await;
        }
        // Left column
        for dy in 1..=3 {
            self.set_block_in_and_broadcast_dim(dim, x, base_y + dy, z, obsidian)
                .await;
        }
        // Right column
        for dy in 1..=3 {
            self.set_block_in_and_broadcast_dim(dim, x + 3, base_y + dy, z, obsidian)
                .await;
        }
        // Interior: portal blocks
        for dx in 1..=2 {
            for dy in 1..=3 {
                self.set_block_in_and_broadcast_dim(dim, x + dx, base_y + dy, z, portal_rid)
                    .await;
            }
        }
    }

    /// Create the obsidian platform at End spawn (0, 48, 0).
    async fn create_end_platform(&mut self) {
        let obsidian = self.tick_blocks.obsidian;
        let air = self.flat_world_blocks.air;

        // Ensure chunks around End spawn are loaded
        self.ensure_chunks_loaded(2, 0, 0).await;

        // 5×5 obsidian platform at Y=48
        for dx in -2..=2 {
            for dz in -2..=2 {
                self.set_block_in_and_broadcast_dim(2, dx, 48, dz, obsidian)
                    .await;
                // Clear 3 blocks above
                for dy in 1..=3 {
                    self.set_block_in_and_broadcast_dim(2, dx, 48 + dy, dz, air)
                        .await;
                }
            }
        }
    }

    /// Set a block in a specific dimension and broadcast the update to players in that dimension.
    async fn set_block_in_and_broadcast_dim(
        &mut self,
        dim: i32,
        x: i32,
        y: i32,
        z: i32,
        runtime_id: u32,
    ) {
        if self.set_block_in(dim, x, y, z, runtime_id) {
            let pos = BlockPos::new(x, y, z);
            self.broadcast_packet_in_dimension(
                dim,
                packets::id::UPDATE_BLOCK,
                &UpdateBlock::new(pos, runtime_id),
            )
            .await;
        }
    }

    /// Ensure chunks around a position are loaded/generated in a dimension.
    async fn ensure_chunks_loaded(&mut self, dim: i32, x: i32, z: i32) {
        let cx = x >> 4;
        let cz = z >> 4;

        for dcx in -1..=1 {
            for dcz in -1..=1 {
                let target_cx = cx + dcx;
                let target_cz = cz + dcz;
                if self
                    .dim_chunks(dim)
                    .is_some_and(|m| m.contains_key(&(target_cx, target_cz)))
                {
                    continue;
                }

                // Try loading from LevelDB
                if let Some(loaded) = self.chunk_storage.load_chunk_dim(target_cx, target_cz, dim) {
                    self.dim_chunks_mut(dim)
                        .insert((target_cx, target_cz), loaded);
                    continue;
                }

                // Generate
                let gen_ow = self.overworld_generator.clone();
                let gen_neth = self.nether_generator.clone();
                let gen_end = self.end_generator.clone();
                let fb = self.flat_world_blocks;
                let d = dim;
                let tcx = target_cx;
                let tcz = target_cz;

                if let Ok(mut col) = tokio::task::spawn_blocking(move || match d {
                    1 => gen_neth
                        .as_ref()
                        .map(|g| g.generate_chunk(tcx, tcz))
                        .unwrap_or_else(|| generate_flat_chunk(tcx, tcz, &fb)),
                    2 => gen_end
                        .as_ref()
                        .map(|g| g.generate_chunk(tcx, tcz))
                        .unwrap_or_else(|| generate_flat_chunk(tcx, tcz, &fb)),
                    _ => gen_ow
                        .as_ref()
                        .map(|g| g.generate_chunk(tcx, tcz))
                        .unwrap_or_else(|| generate_flat_chunk(tcx, tcz, &fb)),
                })
                .await
                {
                    col.dirty = true;
                    self.dim_chunks_mut(dim).insert((target_cx, target_cz), col);
                }
            }
        }
    }

    /// Find a safe Y position for spawning in a dimension.
    fn find_safe_y(&self, dim: i32, x: i32, z: i32) -> i32 {
        // Scan from top down to find first air block above solid ground
        let max_y = match dim {
            1 => 120, // Nether ceiling at 128
            2 => 100,
            _ => 256,
        };
        let min_y = match dim {
            1 => 5,
            _ => OVERWORLD_MIN_Y,
        };

        for y in (min_y..max_y).rev() {
            let block = self.get_block_in(dim, x, y, z);
            let above = self.get_block_in(dim, x, y + 1, z);
            let above2 = self.get_block_in(dim, x, y + 2, z);

            let is_solid = block
                .map(|rid| self.block_registry.is_solid(rid))
                .unwrap_or(false);
            let above_clear = above
                .map(|rid| !self.block_registry.is_solid(rid))
                .unwrap_or(true);
            let above2_clear = above2
                .map(|rid| !self.block_registry.is_solid(rid))
                .unwrap_or(true);

            if is_solid && above_clear && above2_clear {
                return y + 1;
            }
        }

        // Fallback: use a default safe Y
        match dim {
            1 => 64,
            2 => Self::END_SPAWN_Y,
            _ => 64,
        }
    }

    /// Find ground Y near a position (for portal creation).
    fn find_ground_y(&self, dim: i32, x: i32, y: i32, z: i32) -> i32 {
        // Look for solid ground nearby
        for dy in 0..10 {
            let check_y = y - dy;
            if check_y < OVERWORLD_MIN_Y {
                break;
            }
            if let Some(rid) = self.get_block_in(dim, x, check_y, z) {
                if self.block_registry.is_solid(rid) {
                    return check_y + 1;
                }
            }
        }
        // If no ground found, check upwards
        for dy in 1..10 {
            let check_y = y + dy;
            if let Some(rid) = self.get_block_in(dim, x, check_y, z) {
                if self.block_registry.is_solid(rid) {
                    return check_y + 1;
                }
            }
        }
        // Fallback
        y
    }

    /// Check if a player is holding flint_and_steel.
    pub(super) fn is_holding_flint_and_steel(&self, addr: SocketAddr) -> bool {
        let conn = match self.connections.get(&addr) {
            Some(c) => c,
            None => return false,
        };
        let held = conn.inventory.held_item();
        if held.runtime_id == 0 {
            return false;
        }
        self.item_registry
            .get_by_id(held.runtime_id as i16)
            .map(|info| info.name == "minecraft:flint_and_steel")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nether_coordinate_scaling() {
        // Overworld (800, y, 1600) → Nether (100, y, 200)
        let ow_x: f32 = 800.0;
        let ow_z: f32 = 1600.0;
        let nether_x = ow_x / ConnectionHandler::NETHER_SCALE;
        let nether_z = ow_z / ConnectionHandler::NETHER_SCALE;
        assert_eq!(nether_x, 100.0);
        assert_eq!(nether_z, 200.0);

        // Nether (50, y, -25) → Overworld (400, y, -200)
        let n_x: f32 = 50.0;
        let n_z: f32 = -25.0;
        let ow_x = n_x * ConnectionHandler::NETHER_SCALE;
        let ow_z = n_z * ConnectionHandler::NETHER_SCALE;
        assert_eq!(ow_x, 400.0);
        assert_eq!(ow_z, -200.0);
    }

    #[test]
    fn portal_cooldown_constant() {
        // 80 ticks = 4 seconds at 20 TPS
        assert_eq!(ConnectionHandler::PORTAL_COOLDOWN_TICKS, 80);
    }

    #[test]
    fn portal_axis_variants() {
        assert_ne!(PortalAxis::X, PortalAxis::Z);
        let x = PortalAxis::X;
        let z = PortalAxis::Z;
        assert_eq!(x, PortalAxis::X);
        assert_eq!(z, PortalAxis::Z);
    }

    #[test]
    fn portal_frame_struct() {
        let frame = PortalFrame {
            min: (0, 1, 5),
            max: (1, 3, 5),
            axis: PortalAxis::X,
        };
        assert_eq!(frame.min, (0, 1, 5));
        assert_eq!(frame.max, (1, 3, 5));
        assert_eq!(frame.axis, PortalAxis::X);
    }
}
