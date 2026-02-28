//! Projectile management: spawn, tick, collision, despawn.

use std::net::SocketAddr;

use mc_rs_game::combat::{self as game_combat, enchantment_id, parse_enchantments};
use mc_rs_game::projectile::{
    self, arrow_config, check_entity_collision, launch_velocity, step_projectile, trident_config,
};
use mc_rs_proto::packets::{
    self, AddActor, EntityEvent, EntityMetadataEntry, InventoryContent, MetadataValue,
    MoveActorAbsolute, RemoveEntity, SetEntityMotion, UpdateAttributes,
};
use mc_rs_proto::types::Vec3;

use super::ConnectionHandler;

/// The type of projectile.
#[derive(Debug, Clone)]
pub enum ProjectileKind {
    Arrow,
    Trident {
        /// Loyalty enchantment level (0 = none).
        loyalty: i16,
    },
}

/// A live projectile in the world.
pub struct ActiveProjectile {
    pub unique_id: i64,
    pub runtime_id: u64,
    pub position: (f32, f32, f32),
    pub velocity: (f32, f32, f32),
    pub pitch: f32,
    pub yaw: f32,
    pub shooter_runtime_id: u64,
    pub shooter_addr: SocketAddr,
    pub kind: ProjectileKind,
    pub ticks_alive: u32,
    pub stuck_ticks: u32,
    pub is_stuck: bool,
    pub critical: bool,
    pub damage: f32,
    pub punch_level: i16,
    pub flame: bool,
    #[allow(dead_code)]
    pub infinity: bool,
    /// Whether this trident is returning to the player via Loyalty.
    pub returning: bool,
}

impl ConnectionHandler {
    /// Tick all active projectiles: physics, collision, despawn.
    pub(super) async fn tick_projectiles(&mut self) {
        if self.active_projectiles.is_empty() {
            return;
        }

        let arrow_cfg = arrow_config();
        let trident_cfg = trident_config();

        // Collect entity positions for collision detection:
        // players + mobs
        let mut entity_targets: Vec<(u64, f32, f32, f32, f32, f32)> = Vec::new();

        for conn in self.connections.values() {
            if conn.state == super::LoginState::InGame && !conn.is_dead {
                // Bedrock position.y = eye position; feet = y - 1.62
                let feet_y = conn.position.y - 1.62;
                entity_targets.push((
                    conn.entity_runtime_id,
                    conn.position.x,
                    feet_y,
                    conn.position.z,
                    0.6,
                    1.8,
                ));
            }
        }

        let mob_snapshots = self.game_world.all_mobs();
        for mob in &mob_snapshots {
            entity_targets.push((
                mob.runtime_id,
                mob.position.0,
                mob.position.1,
                mob.position.2,
                mob.bb_width,
                mob.bb_height,
            ));
        }

        let current_tick = self.game_world.current_tick();

        let mut to_remove: Vec<usize> = Vec::new();
        let mut damage_events: Vec<ProjectileHitEvent> = Vec::new();
        let mut move_packets: Vec<MoveActorAbsolute> = Vec::new();
        let mut remove_packets: Vec<RemoveEntity> = Vec::new();

        // Take ownership of projectiles so we can mutate them while accessing
        // self.get_block(), self.connections, etc.
        let mut projectiles = std::mem::take(&mut self.active_projectiles);

        for (i, proj) in projectiles.iter_mut().enumerate() {
            proj.ticks_alive += 1;

            if proj.is_stuck {
                proj.stuck_ticks += 1;

                // Trident loyalty: return to player after 40 ticks
                if let ProjectileKind::Trident { loyalty } = &proj.kind {
                    if *loyalty > 0 && proj.stuck_ticks > 40 && !proj.returning {
                        proj.returning = true;
                        proj.is_stuck = false;
                        proj.stuck_ticks = 0;
                    }
                }

                let max_age = match &proj.kind {
                    ProjectileKind::Arrow => arrow_cfg.max_stuck_age,
                    ProjectileKind::Trident { .. } => trident_cfg.max_stuck_age,
                };
                if proj.stuck_ticks >= max_age {
                    to_remove.push(i);
                    remove_packets.push(RemoveEntity {
                        entity_unique_id: proj.unique_id,
                    });
                }
                continue;
            }

            let config = match &proj.kind {
                ProjectileKind::Arrow => &arrow_cfg,
                ProjectileKind::Trident { .. } => &trident_cfg,
            };

            // Trident returning: fly towards shooter
            if proj.returning {
                if let Some(conn) = self.connections.get(&proj.shooter_addr) {
                    let tx = conn.position.x;
                    let ty = conn.position.y; // eye position
                    let tz = conn.position.z;
                    let dx = tx - proj.position.0;
                    let dy = ty - proj.position.1;
                    let dz = tz - proj.position.2;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                    if dist < 1.5 {
                        // Arrived — remove projectile, restore trident
                        to_remove.push(i);
                        remove_packets.push(RemoveEntity {
                            entity_unique_id: proj.unique_id,
                        });
                        damage_events.push(ProjectileHitEvent::TridentReturn {
                            shooter_addr: proj.shooter_addr,
                        });
                        continue;
                    }

                    // Fly towards player
                    let speed = 1.5;
                    let nx = dx / dist * speed;
                    let ny = dy / dist * speed;
                    let nz = dz / dist * speed;
                    proj.velocity = (nx, ny, nz);
                }
            }

            // Step physics
            let (new_pos, new_vel, new_pitch, new_yaw) =
                step_projectile(proj.position, proj.velocity, config);
            proj.position = new_pos;
            proj.velocity = new_vel;
            proj.pitch = new_pitch;
            proj.yaw = new_yaw;

            // Block collision
            let bx = new_pos.0.floor() as i32;
            let by = new_pos.1.floor() as i32;
            let bz = new_pos.2.floor() as i32;
            if let Some(block_hash) = self.get_block(bx, by, bz) {
                if self.block_registry.is_solid(block_hash) {
                    proj.is_stuck = true;
                    proj.velocity = (0.0, 0.0, 0.0);
                    // Send final position
                    move_packets.push(MoveActorAbsolute::normal(
                        proj.runtime_id,
                        Vec3::new(proj.position.0, proj.position.1, proj.position.2),
                        proj.pitch,
                        proj.yaw,
                        proj.yaw,
                        true,
                    ));
                    continue;
                }
            }

            // Entity collision (skip if returning trident)
            if !proj.returning {
                if let Some(hit_rid) = check_entity_collision(
                    proj.position,
                    &entity_targets,
                    proj.shooter_runtime_id,
                    config.bb_radius,
                ) {
                    // Determine if hit is a player or mob
                    let is_player = self
                        .connections
                        .values()
                        .any(|c| c.entity_runtime_id == hit_rid);

                    damage_events.push(ProjectileHitEvent::EntityHit {
                        target_rid: hit_rid,
                        is_player,
                        damage: proj.damage,
                        critical: proj.critical,
                        punch_level: proj.punch_level,
                        flame: proj.flame,
                        shooter_rid: proj.shooter_runtime_id,
                        shooter_addr: proj.shooter_addr,
                    });

                    to_remove.push(i);
                    remove_packets.push(RemoveEntity {
                        entity_unique_id: proj.unique_id,
                    });
                    continue;
                }
            }

            // Broadcast movement
            move_packets.push(MoveActorAbsolute::normal(
                proj.runtime_id,
                Vec3::new(proj.position.0, proj.position.1, proj.position.2),
                proj.pitch,
                proj.yaw,
                proj.yaw,
                false,
            ));

            // Max lifetime (100 seconds in flight without hitting anything)
            if proj.ticks_alive > 2000 {
                to_remove.push(i);
                remove_packets.push(RemoveEntity {
                    entity_unique_id: proj.unique_id,
                });
            }
        }

        // Remove projectiles (reverse order to keep indices valid)
        to_remove.sort_unstable();
        to_remove.dedup();
        for i in to_remove.into_iter().rev() {
            projectiles.swap_remove(i);
        }

        // Put projectiles back
        self.active_projectiles = projectiles;

        // Broadcast move packets
        for pkt in &move_packets {
            self.broadcast_packet(packets::id::MOVE_ACTOR_ABSOLUTE, pkt)
                .await;
        }

        // Broadcast remove packets
        for pkt in &remove_packets {
            self.broadcast_packet(packets::id::REMOVE_ENTITY, pkt).await;
        }

        // Process damage events
        for event in damage_events {
            match event {
                ProjectileHitEvent::EntityHit {
                    target_rid,
                    is_player,
                    damage,
                    critical,
                    punch_level,
                    flame,
                    shooter_rid,
                    shooter_addr,
                    ..
                } => {
                    let final_damage = if critical { damage * 1.5 } else { damage };

                    if is_player {
                        // Find target player addr
                        let target_addr = self
                            .connections
                            .iter()
                            .find(|(_, c)| c.entity_runtime_id == target_rid)
                            .map(|(&a, _)| a);

                        if let Some(t_addr) = target_addr {
                            self.apply_projectile_damage_to_player(
                                t_addr,
                                final_damage,
                                punch_level,
                                flame,
                                shooter_rid,
                                shooter_addr,
                                current_tick,
                            )
                            .await;
                        }
                    } else {
                        // Damage mob
                        self.game_world.damage_mob(
                            target_rid,
                            final_damage,
                            current_tick,
                            Some(shooter_rid),
                        );

                        // Apply knockback to mob
                        if punch_level > 0 {
                            let kb = 0.4 * punch_level as f32;
                            if let Some((mx, _, mz)) = self.game_world.mob_position(target_rid) {
                                if let Some(conn) = self.connections.get(&shooter_addr) {
                                    let dx = mx - conn.position.x;
                                    let dz = mz - conn.position.z;
                                    let dist = (dx * dx + dz * dz).sqrt().max(0.01);
                                    self.game_world.apply_knockback(
                                        target_rid,
                                        dx / dist * kb,
                                        0.3,
                                        dz / dist * kb,
                                    );
                                    self.broadcast_packet(
                                        packets::id::SET_ENTITY_MOTION,
                                        &SetEntityMotion {
                                            entity_runtime_id: target_rid,
                                            motion: Vec3::new(dx / dist * kb, 0.3, dz / dist * kb),
                                        },
                                    )
                                    .await;
                                }
                            }
                        }

                        // Flame → set fire ticks on mob (simplified: just extra damage)
                        if flame {
                            self.game_world
                                .damage_mob(target_rid, 1.0, current_tick + 10, None);
                        }
                    }
                }
                ProjectileHitEvent::TridentReturn { shooter_addr } => {
                    // Restore trident to player inventory
                    if let Some(conn) = self.connections.get_mut(&shooter_addr) {
                        // Find first empty slot or the held slot
                        let slot = conn.inventory.held_slot as usize;
                        if conn.inventory.main[slot].is_empty() {
                            // Look up trident item
                            if let Some(info) = self.item_registry.get_by_name("minecraft:trident")
                            {
                                conn.inventory.main[slot] = mc_rs_proto::item_stack::ItemStack::new(
                                    info.numeric_id as i32,
                                    1,
                                );
                            }
                        } else {
                            // Find first empty slot
                            if let Some(info) = self.item_registry.get_by_name("minecraft:trident")
                            {
                                for s in 0..36 {
                                    if conn.inventory.main[s].is_empty() {
                                        conn.inventory.main[s] =
                                            mc_rs_proto::item_stack::ItemStack::new(
                                                info.numeric_id as i32,
                                                1,
                                            );
                                        break;
                                    }
                                }
                            }
                        }

                        let items = conn.inventory.main.clone();
                        self.send_packet(
                            shooter_addr,
                            packets::id::INVENTORY_CONTENT,
                            &InventoryContent {
                                window_id: 0,
                                items,
                            },
                        )
                        .await;
                    }
                }
            }
        }
    }

    /// Apply projectile damage to a player.
    #[allow(clippy::too_many_arguments)]
    async fn apply_projectile_damage_to_player(
        &mut self,
        target_addr: SocketAddr,
        damage: f32,
        punch_level: i16,
        flame: bool,
        _shooter_rid: u64,
        shooter_addr: SocketAddr,
        tick: u64,
    ) {
        // Extract everything we need from the target before any mutable borrow
        let (rid, armor_defense, armor_nbt_slots, target_pos) = {
            let conn = match self.connections.get(&target_addr) {
                Some(c) => c,
                None => return,
            };
            if conn.is_dead || conn.gamemode == 1 || conn.gamemode == 3 {
                return;
            }
            if let Some(last) = conn.last_damage_tick {
                if tick.saturating_sub(last) < 10 {
                    return;
                }
            }
            let def = game_combat::total_armor_defense(&self.item_registry, &conn.inventory.armor);
            let nbt_slots: Vec<Vec<u8>> = conn
                .inventory
                .armor
                .iter()
                .map(|item| item.nbt_data.clone())
                .collect();
            (
                conn.entity_runtime_id,
                def,
                nbt_slots,
                (conn.position.x, conn.position.z),
            )
        };

        // Calculate damage
        let armor_nbt_refs: Vec<&[u8]> = armor_nbt_slots.iter().map(|v| v.as_slice()).collect();
        let input = game_combat::DamageInput {
            base_damage: damage,
            weapon_nbt: &[],
            armor_defense,
            armor_nbt_slots: &armor_nbt_refs,
            is_critical: false,
            strength_bonus: 0.0,
            weakness_penalty: 0.0,
            resistance_factor: 0.0,
        };
        let final_damage = game_combat::calculate_damage(&input);

        // Apply damage
        {
            let conn = match self.connections.get_mut(&target_addr) {
                Some(c) => c,
                None => return,
            };
            conn.health = (conn.health - final_damage).max(0.0);
            conn.last_damage_tick = Some(tick);
            if flame && conn.fire_ticks <= 0 {
                conn.fire_ticks = 100; // 5 seconds
            }
        }

        // Punch knockback (need shooter position)
        if punch_level > 0 {
            let shooter_pos = self
                .connections
                .get(&shooter_addr)
                .map(|c| (c.position.x, c.position.z));
            if let Some((sx, sz)) = shooter_pos {
                let dx = target_pos.0 - sx;
                let dz = target_pos.1 - sz;
                let dist = (dx * dx + dz * dz).sqrt().max(0.01);
                let kb = 0.4 * punch_level as f32;
                let motion = Vec3::new(dx / dist * kb, 0.3, dz / dist * kb);
                self.send_packet(
                    target_addr,
                    packets::id::SET_ENTITY_MOTION,
                    &SetEntityMotion {
                        entity_runtime_id: rid,
                        motion,
                    },
                )
                .await;
            }
        }

        let health = self
            .connections
            .get(&target_addr)
            .map(|c| c.health)
            .unwrap_or(0.0);

        // Send hurt effect
        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
            .await;
        self.send_packet(
            target_addr,
            packets::id::UPDATE_ATTRIBUTES,
            &UpdateAttributes::health(rid, health, tick),
        )
        .await;

        if health <= 0.0 {
            let victim_name = self
                .connections
                .get(&target_addr)
                .and_then(|c| c.login_data.as_ref())
                .map(|d| d.display_name.clone())
                .unwrap_or_default();
            let shooter_name = self
                .connections
                .get(&shooter_addr)
                .and_then(|c| c.login_data.as_ref())
                .map(|d| d.display_name.clone())
                .unwrap_or_else(|| "a projectile".to_string());
            let msg = format!("{victim_name} was shot by {shooter_name}");
            self.handle_player_death_with_message(target_addr, &msg)
                .await;
        }
    }

    /// Handle a bow release: spawn an arrow projectile.
    pub(super) async fn handle_bow_release(&mut self, addr: SocketAddr) {
        let start_tick = match self.bow_charge_start.remove(&addr) {
            Some(t) => t,
            None => return,
        };
        let current_tick = self.game_world.current_tick();
        let charge_ticks = current_tick.saturating_sub(start_tick).min(20) as u32;

        // Minimum charge: 3 ticks
        if charge_ticks < 3 {
            return;
        }

        let conn = match self.connections.get_mut(&addr) {
            Some(c) => c,
            None => return,
        };
        if conn.gamemode != 0 && conn.gamemode != 2 {
            // Only survival and adventure
            return;
        }

        // Read bow enchantments from held item
        let held = conn.inventory.held_item();
        let enchantments = parse_enchantments(&held.nbt_data);
        let power_level = enchantments
            .iter()
            .find(|e| e.id == enchantment_id::POWER)
            .map(|e| e.level)
            .unwrap_or(0);
        let punch_level = enchantments
            .iter()
            .find(|e| e.id == enchantment_id::PUNCH)
            .map(|e| e.level)
            .unwrap_or(0);
        let flame = enchantments.iter().any(|e| e.id == enchantment_id::FLAME);
        let infinity = enchantments
            .iter()
            .any(|e| e.id == enchantment_id::INFINITY);

        // Check for arrow in inventory (unless Infinity)
        if !infinity {
            let has_arrow = conn.inventory.main.iter().any(|s| {
                if s.is_empty() {
                    return false;
                }
                self.item_registry
                    .get_by_id(s.runtime_id as i16)
                    .map(|i| i.name == "minecraft:arrow")
                    .unwrap_or(false)
            });
            if !has_arrow {
                return;
            }
            // Consume one arrow
            for slot in conn.inventory.main.iter_mut() {
                if slot.is_empty() {
                    continue;
                }
                let is_arrow = self
                    .item_registry
                    .get_by_id(slot.runtime_id as i16)
                    .map(|i| i.name == "minecraft:arrow")
                    .unwrap_or(false);
                if is_arrow {
                    if slot.count > 1 {
                        slot.count -= 1;
                    } else {
                        *slot = mc_rs_proto::item_stack::ItemStack::empty();
                    }
                    break;
                }
            }
        }

        let charge_factor = (charge_ticks as f32) / 20.0;
        let speed = arrow_config().base_speed * charge_factor;
        let critical = charge_ticks >= 20;

        let pitch = conn.pitch;
        let yaw = conn.yaw;
        let pos = (conn.position.x, conn.position.y, conn.position.z);
        let shooter_rid = conn.entity_runtime_id;

        let (vx, vy, vz) = launch_velocity(pitch, yaw, speed);
        let damage = projectile::arrow_damage(charge_ticks, power_level);

        let entity_id = self.game_world.allocate_entity_id();
        let runtime_id = entity_id as u64;

        let proj = ActiveProjectile {
            unique_id: entity_id,
            runtime_id,
            position: pos,
            velocity: (vx, vy, vz),
            pitch,
            yaw,
            shooter_runtime_id: shooter_rid,
            shooter_addr: addr,
            kind: ProjectileKind::Arrow,
            ticks_alive: 0,
            stuck_ticks: 0,
            is_stuck: false,
            critical,
            damage,
            punch_level,
            flame,
            infinity,
            returning: false,
        };
        self.active_projectiles.push(proj);

        // Broadcast AddActor for the arrow
        let metadata = projectile_metadata(critical);
        let pkt = AddActor {
            entity_unique_id: entity_id,
            entity_runtime_id: runtime_id,
            entity_type: "minecraft:arrow".to_string(),
            position: Vec3::new(pos.0, pos.1, pos.2),
            velocity: Vec3::new(vx, vy, vz),
            pitch,
            yaw,
            head_yaw: yaw,
            body_yaw: yaw,
            attributes: vec![],
            metadata,
        };
        self.broadcast_packet(packets::id::ADD_ACTOR, &pkt).await;

        // Send updated inventory to shooter
        let items = self
            .connections
            .get(&addr)
            .map(|c| c.inventory.main.clone())
            .unwrap_or_default();
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 0,
                items,
            },
        )
        .await;
    }

    /// Handle a trident throw.
    pub(super) async fn throw_trident(&mut self, addr: SocketAddr) {
        let conn = match self.connections.get_mut(&addr) {
            Some(c) => c,
            None => return,
        };
        if conn.gamemode != 0 && conn.gamemode != 2 {
            return;
        }

        let held = conn.inventory.held_item();
        if held.is_empty() {
            return;
        }

        // Read enchantments
        let enchantments = parse_enchantments(&held.nbt_data);
        let loyalty = enchantments
            .iter()
            .find(|e| e.id == enchantment_id::LOYALTY)
            .map(|e| e.level)
            .unwrap_or(0);

        let pitch = conn.pitch;
        let yaw = conn.yaw;
        let pos = (conn.position.x, conn.position.y, conn.position.z);
        let shooter_rid = conn.entity_runtime_id;

        // Remove trident from hand
        let slot = conn.inventory.held_slot as usize;
        conn.inventory.main[slot] = mc_rs_proto::item_stack::ItemStack::empty();

        let speed = trident_config().base_speed;
        let (vx, vy, vz) = launch_velocity(pitch, yaw, speed);

        let entity_id = self.game_world.allocate_entity_id();
        let runtime_id = entity_id as u64;

        let proj = ActiveProjectile {
            unique_id: entity_id,
            runtime_id,
            position: pos,
            velocity: (vx, vy, vz),
            pitch,
            yaw,
            shooter_runtime_id: shooter_rid,
            shooter_addr: addr,
            kind: ProjectileKind::Trident { loyalty },
            ticks_alive: 0,
            stuck_ticks: 0,
            is_stuck: false,
            critical: false,
            damage: 8.0, // Trident base damage
            punch_level: 0,
            flame: false,
            infinity: false,
            returning: false,
        };
        self.active_projectiles.push(proj);

        let metadata = projectile_metadata(false);
        let pkt = AddActor {
            entity_unique_id: entity_id,
            entity_runtime_id: runtime_id,
            entity_type: "minecraft:trident".to_string(),
            position: Vec3::new(pos.0, pos.1, pos.2),
            velocity: Vec3::new(vx, vy, vz),
            pitch,
            yaw,
            head_yaw: yaw,
            body_yaw: yaw,
            attributes: vec![],
            metadata,
        };
        self.broadcast_packet(packets::id::ADD_ACTOR, &pkt).await;

        // Send updated inventory
        let items = self
            .connections
            .get(&addr)
            .map(|c| c.inventory.main.clone())
            .unwrap_or_default();
        self.send_packet(
            addr,
            packets::id::INVENTORY_CONTENT,
            &InventoryContent {
                window_id: 0,
                items,
            },
        )
        .await;
    }

    /// Sync active projectiles to a newly joined player.
    pub(super) async fn sync_projectiles_to_player(&mut self, addr: SocketAddr) {
        let packets: Vec<AddActor> = self
            .active_projectiles
            .iter()
            .map(|proj| {
                let entity_type = match &proj.kind {
                    ProjectileKind::Arrow => "minecraft:arrow".to_string(),
                    ProjectileKind::Trident { .. } => "minecraft:trident".to_string(),
                };
                let metadata = projectile_metadata(proj.critical);
                AddActor {
                    entity_unique_id: proj.unique_id,
                    entity_runtime_id: proj.runtime_id,
                    entity_type,
                    position: Vec3::new(proj.position.0, proj.position.1, proj.position.2),
                    velocity: Vec3::new(proj.velocity.0, proj.velocity.1, proj.velocity.2),
                    pitch: proj.pitch,
                    yaw: proj.yaw,
                    head_yaw: proj.yaw,
                    body_yaw: proj.yaw,
                    attributes: vec![],
                    metadata,
                }
            })
            .collect();
        for pkt in &packets {
            self.send_packet(addr, packets::id::ADD_ACTOR, pkt).await;
        }
    }

    /// Clean up projectiles belonging to a disconnecting player.
    pub(super) async fn cleanup_player_projectiles(&mut self, addr: SocketAddr) {
        let remove_ids: Vec<(usize, i64)> = self
            .active_projectiles
            .iter()
            .enumerate()
            .filter(|(_, proj)| proj.shooter_addr == addr)
            .map(|(i, proj)| (i, proj.unique_id))
            .collect();
        for &(_, uid) in &remove_ids {
            let pkt = RemoveEntity {
                entity_unique_id: uid,
            };
            self.broadcast_packet(packets::id::REMOVE_ENTITY, &pkt)
                .await;
        }
        for &(i, _) in remove_ids.iter().rev() {
            self.active_projectiles.swap_remove(i);
        }
        self.bow_charge_start.remove(&addr);
    }
}

/// Damage event from a projectile hit.
enum ProjectileHitEvent {
    EntityHit {
        target_rid: u64,
        is_player: bool,
        damage: f32,
        critical: bool,
        punch_level: i16,
        flame: bool,
        shooter_rid: u64,
        shooter_addr: SocketAddr,
    },
    TridentReturn {
        shooter_addr: SocketAddr,
    },
}

/// Build entity metadata for a projectile.
fn projectile_metadata(critical: bool) -> Vec<EntityMetadataEntry> {
    let flags: i64 = if critical { 1 } else { 0 };
    vec![
        EntityMetadataEntry {
            key: 0,
            data_type: 7, // Long
            value: MetadataValue::Long(flags),
        },
        EntityMetadataEntry {
            key: 23,
            data_type: 3,                     // Float
            value: MetadataValue::Float(1.0), // SCALE
        },
    ]
}
