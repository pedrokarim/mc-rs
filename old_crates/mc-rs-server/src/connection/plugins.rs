use super::*;

impl ConnectionHandler {
    /// Apply deferred plugin actions.
    pub(super) async fn apply_plugin_actions(&mut self, actions: Vec<PendingAction>) {
        for action in actions {
            match action {
                PendingAction::SendMessage {
                    player_name,
                    message,
                } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        self.send_packet(addr, packets::id::TEXT, &Text::raw(&message))
                            .await;
                    }
                }
                PendingAction::BroadcastMessage { message } => {
                    self.broadcast_packet(packets::id::TEXT, &Text::raw(&message))
                        .await;
                }
                PendingAction::KickPlayer {
                    player_name,
                    reason,
                } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        self.send_packet(
                            addr,
                            packets::id::DISCONNECT,
                            &Disconnect::with_message(&reason),
                        )
                        .await;
                    }
                }
                PendingAction::SetPlayerHealth {
                    player_name,
                    health,
                } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        let pkt = if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.health = health.clamp(0.0, 20.0);
                            Some(UpdateAttributes::health(
                                conn.entity_runtime_id,
                                conn.health,
                                0,
                            ))
                        } else {
                            None
                        };
                        if let Some(pkt) = pkt {
                            self.send_packet(addr, packets::id::UPDATE_ATTRIBUTES, &pkt)
                                .await;
                        }
                    }
                }
                PendingAction::SetPlayerFood { player_name, food } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.food = food.clamp(0, 20);
                        }
                    }
                }
                PendingAction::TeleportPlayer {
                    player_name,
                    x,
                    y,
                    z,
                } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        let pkt = if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.position = Vec3::new(x, y, z);
                            Some(MovePlayer {
                                runtime_entity_id: conn.entity_runtime_id,
                                position: Vec3::new(x, y, z),
                                pitch: conn.pitch,
                                yaw: conn.yaw,
                                head_yaw: conn.head_yaw,
                                mode: MoveMode::Teleport,
                                on_ground: false,
                                ridden_entity_runtime_id: 0,
                                teleport_cause: Some(0),
                                teleport_entity_type: Some(0),
                                tick: conn.client_tick,
                            })
                        } else {
                            None
                        };
                        if let Some(pkt) = pkt {
                            self.send_packet(addr, packets::id::MOVE_PLAYER, &pkt).await;
                        }
                    }
                }
                PendingAction::SetTime { time } => {
                    self.world_time = time;
                    let pkt = SetTime {
                        time: self.world_time as i32,
                    };
                    self.broadcast_packet(packets::id::SET_TIME, &pkt).await;
                }
                PendingAction::SpawnMob { mob_type, x, y, z } => {
                    self.game_world.spawn_mob(&mob_type, x, y, z);
                }
                PendingAction::RemoveMob { runtime_id } => {
                    self.game_world.remove_mob(runtime_id);
                }
                PendingAction::RegisterCommand {
                    name, plugin_name, ..
                } => {
                    self.plugin_manager
                        .plugin_commands
                        .insert(name, plugin_name);
                }
                PendingAction::ShowForm {
                    player_name,
                    form_id,
                    form_data,
                    form_type,
                } => {
                    if let Some(addr) = self.find_player_addr(&player_name) {
                        if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.pending_forms.insert(form_id, form_type);
                        }
                        self.send_packet(
                            addr,
                            packets::id::MODAL_FORM_REQUEST,
                            &packets::ModalFormRequest { form_id, form_data },
                        )
                        .await;
                    }
                }
                PendingAction::ScheduleTask { .. } | PendingAction::CancelTask { .. } => {
                    // These are handled internally by PluginManager
                }
                PendingAction::Log { level, message } => {
                    use mc_rs_plugin_api::LogLevel;
                    match level {
                        LogLevel::Info => info!("[plugin] {message}"),
                        LogLevel::Warn => warn!("[plugin] {message}"),
                        LogLevel::Error => warn!("[plugin] ERROR: {message}"),
                        LogLevel::Debug => debug!("[plugin] {message}"),
                    }
                }
            }
        }
    }

    /// Drain ECS game events and send the corresponding packets.
    pub(super) async fn process_game_events(&mut self) {
        let events = self.game_world.drain_events();
        for event in events {
            match event {
                GameEvent::MobSpawned {
                    runtime_id,
                    unique_id,
                    mob_type,
                    position,
                    health,
                    max_health,
                    bb_width,
                    bb_height,
                    is_baby,
                } => {
                    // Plugin event: MobSpawn (cancellable)
                    {
                        let mob_event = PluginEvent::MobSpawn {
                            mob_type: mob_type.clone(),
                            runtime_id,
                            position,
                        };
                        let snapshot = self.build_snapshot();
                        let (result, actions) = self.plugin_manager.dispatch(&mob_event, &snapshot);
                        self.apply_plugin_actions(actions).await;
                        if result == EventResult::Cancelled {
                            self.game_world.remove_mob(runtime_id);
                            continue;
                        }
                    }
                    let metadata = if is_baby {
                        baby_mob_metadata(bb_width, bb_height)
                    } else {
                        default_mob_metadata(bb_width, bb_height)
                    };
                    let pkt = AddActor {
                        entity_unique_id: unique_id,
                        entity_runtime_id: runtime_id,
                        entity_type: mob_type,
                        position: Vec3::new(position.0, position.1, position.2),
                        velocity: Vec3::ZERO,
                        pitch: 0.0,
                        yaw: 0.0,
                        head_yaw: 0.0,
                        body_yaw: 0.0,
                        attributes: vec![ActorAttribute {
                            name: "minecraft:health".to_string(),
                            min: 0.0,
                            max: max_health,
                            current: health,
                            default: max_health,
                        }],
                        metadata,
                    };
                    self.broadcast_packet(packets::id::ADD_ACTOR, &pkt).await;
                }
                GameEvent::MobMoved {
                    runtime_id,
                    position,
                    pitch,
                    yaw,
                    head_yaw,
                    on_ground,
                } => {
                    let pkt = MoveActorAbsolute::normal(
                        runtime_id,
                        Vec3::new(position.0, position.1, position.2),
                        pitch,
                        yaw,
                        head_yaw,
                        on_ground,
                    );
                    self.broadcast_packet(packets::id::MOVE_ACTOR_ABSOLUTE, &pkt)
                        .await;
                }
                GameEvent::MobHurt {
                    runtime_id,
                    new_health,
                    tick,
                } => {
                    self.broadcast_packet(
                        packets::id::ENTITY_EVENT,
                        &EntityEvent::hurt(runtime_id),
                    )
                    .await;
                    self.broadcast_packet(
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(runtime_id, new_health, tick),
                    )
                    .await;
                }
                GameEvent::MobDied {
                    runtime_id,
                    unique_id,
                    ref mob_type,
                    killed_by,
                } => {
                    // Plugin event: MobDeath (non-cancellable)
                    {
                        let mob_event = PluginEvent::MobDeath {
                            mob_type: mob_type.clone(),
                            runtime_id,
                            killer_runtime_id: killed_by,
                        };
                        let snapshot = self.build_snapshot();
                        let (_, actions) = self.plugin_manager.dispatch(&mob_event, &snapshot);
                        self.apply_plugin_actions(actions).await;
                    }
                    self.broadcast_packet(
                        packets::id::ENTITY_EVENT,
                        &EntityEvent::death(runtime_id),
                    )
                    .await;
                    self.broadcast_packet(
                        packets::id::REMOVE_ENTITY,
                        &RemoveEntity {
                            entity_unique_id: unique_id,
                        },
                    )
                    .await;
                    // Award XP to the killer
                    if let Some(killer_rid) = killed_by {
                        if let Some(killer_addr) = self.find_addr_by_runtime_id(killer_rid) {
                            let looting_bonus = self
                                .connections
                                .get(&killer_addr)
                                .map(|c| {
                                    game_combat::looting_level(&c.inventory.held_item().nbt_data)
                                        as i32
                                })
                                .unwrap_or(0);
                            let base_xp = xp::mob_xp(mob_type);
                            self.award_xp(killer_addr, base_xp + looting_bonus).await;
                        }
                    }
                }
                GameEvent::EntityRemoved { unique_id } => {
                    self.broadcast_packet(
                        packets::id::REMOVE_ENTITY,
                        &RemoveEntity {
                            entity_unique_id: unique_id,
                        },
                    )
                    .await;
                }
                GameEvent::MobLoveParticles { runtime_id } => {
                    self.broadcast_packet(
                        packets::id::ENTITY_EVENT,
                        &EntityEvent::love_particles(runtime_id),
                    )
                    .await;
                }
                GameEvent::MobAttackPlayer {
                    mob_runtime_id,
                    target_runtime_id,
                    damage: raw_damage,
                    knockback,
                } => {
                    // Find the target player by runtime_id
                    let target_addr = self
                        .connections
                        .iter()
                        .find(|(_, c)| c.entity_runtime_id == target_runtime_id)
                        .map(|(a, _)| *a);
                    if let Some(addr) = target_addr {
                        // Invulnerability check
                        let tick = match self.connections.get(&addr) {
                            Some(c) => c.client_tick,
                            None => continue,
                        };
                        if let Some(last) =
                            self.connections.get(&addr).and_then(|c| c.last_damage_tick)
                        {
                            if tick.saturating_sub(last) < 10 {
                                continue;
                            }
                        }

                        // Apply armor + protection + resistance reduction
                        let (armor_defense, armor_nbt_slots) = {
                            let conn = match self.connections.get(&addr) {
                                Some(c) => c,
                                None => continue,
                            };
                            let defense = game_combat::total_armor_defense(
                                &self.item_registry,
                                &conn.inventory.armor,
                            );
                            let nbt: Vec<Vec<u8>> = conn
                                .inventory
                                .armor
                                .iter()
                                .map(|i| i.nbt_data.clone())
                                .collect();
                            (defense, nbt)
                        };
                        let armor_refs: Vec<&[u8]> =
                            armor_nbt_slots.iter().map(|v| v.as_slice()).collect();
                        let resistance = self.get_resistance_factor(addr);

                        // Mob attacks have no weapon enchantments or criticals
                        let damage = game_combat::calculate_damage(&game_combat::DamageInput {
                            base_damage: raw_damage,
                            weapon_nbt: &[],
                            armor_defense,
                            armor_nbt_slots: &armor_refs,
                            is_critical: false,
                            strength_bonus: 0.0,
                            weakness_penalty: 0.0,
                            resistance_factor: resistance,
                        });

                        // Apply damage
                        let conn = match self.connections.get_mut(&addr) {
                            Some(c) => c,
                            None => continue,
                        };
                        conn.health = (conn.health - damage).max(0.0);
                        conn.last_damage_tick = Some(tick);
                        let new_health = conn.health;
                        let runtime_id = conn.entity_runtime_id;
                        let is_dead = new_health <= 0.0;

                        // Send hurt animation
                        self.broadcast_packet(
                            packets::id::ENTITY_EVENT,
                            &EntityEvent::hurt(runtime_id),
                        )
                        .await;

                        // Send updated health
                        self.broadcast_packet(
                            packets::id::UPDATE_ATTRIBUTES,
                            &UpdateAttributes::health(runtime_id, new_health, tick),
                        )
                        .await;

                        // Knockback
                        self.broadcast_packet(
                            packets::id::SET_ENTITY_MOTION,
                            &SetEntityMotion {
                                entity_runtime_id: runtime_id,
                                motion: Vec3::new(knockback.0, knockback.1, knockback.2),
                            },
                        )
                        .await;

                        // Thorns: reflect damage back to the attacking mob
                        let thorns = game_combat::thorns_level(&armor_refs);
                        if thorns > 0 {
                            let thorns_dmg = (thorns as f32).min(4.0);
                            let mob_tick = self.game_world.current_tick();
                            self.game_world
                                .damage_mob(mob_runtime_id, thorns_dmg, mob_tick, None);
                        }

                        if is_dead {
                            // Death flow — XP loss + mark dead
                            let conn = self.connections.get_mut(&addr).unwrap();
                            let (nl, nt) = xp::after_death(conn.xp_level, conn.xp_total);
                            conn.xp_level = nl;
                            conn.xp_total = nt;
                            conn.is_dead = true;
                            conn.health = 0.0;

                            self.broadcast_packet(
                                packets::id::ENTITY_EVENT,
                                &EntityEvent::death(runtime_id),
                            )
                            .await;

                            self.send_packet(
                                addr,
                                packets::id::RESPAWN,
                                &Respawn {
                                    position: self.spawn_position,
                                    state: 0,
                                    runtime_entity_id: runtime_id,
                                },
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }
}
