use super::*;

impl ConnectionHandler {
    // ------------------------------------------------------------------
    // Status effect management
    // ------------------------------------------------------------------

    /// Apply a status effect to a player, sending the MobEffect packet.
    pub(super) async fn apply_effect(
        &mut self,
        addr: SocketAddr,
        effect_id: i32,
        amplifier: i32,
        duration_ticks: i32,
    ) {
        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                // Remove existing effect of same type
                conn.effects.retain(|e| e.effect_id != effect_id);
                conn.effects.push(ActiveEffect {
                    effect_id,
                    amplifier,
                    remaining_ticks: duration_ticks,
                });
                conn.entity_runtime_id
            }
            None => return,
        };

        self.send_packet(
            addr,
            packets::id::MOB_EFFECT,
            &MobEffect::add(runtime_id, effect_id, amplifier, duration_ticks, true),
        )
        .await;
    }

    /// Remove a status effect from a player, sending the MobEffect(remove) packet.
    #[allow(dead_code)]
    pub(super) async fn remove_effect(&mut self, addr: SocketAddr, effect_id: i32) {
        let runtime_id = match self.connections.get_mut(&addr) {
            Some(conn) => {
                conn.effects.retain(|e| e.effect_id != effect_id);
                conn.entity_runtime_id
            }
            None => return,
        };

        self.send_packet(
            addr,
            packets::id::MOB_EFFECT,
            &MobEffect::remove(runtime_id, effect_id),
        )
        .await;
    }

    /// Remove all status effects from a player.
    pub(super) async fn clear_effects(&mut self, addr: SocketAddr) {
        let (runtime_id, effect_ids) = match self.connections.get_mut(&addr) {
            Some(conn) => {
                let ids: Vec<i32> = conn.effects.iter().map(|e| e.effect_id).collect();
                conn.effects.clear();
                (conn.entity_runtime_id, ids)
            }
            None => return,
        };

        for eid in effect_ids {
            self.send_packet(
                addr,
                packets::id::MOB_EFFECT,
                &MobEffect::remove(runtime_id, eid),
            )
            .await;
        }
    }

    /// Tick all active effects for all players. Called once per game tick (50ms).
    pub(super) async fn tick_effects(&mut self) {
        // Collect addresses of all in-game players
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(_, c)| c.state == LoginState::InGame && !c.is_dead)
            .map(|(a, _)| *a)
            .collect();

        for addr in addrs {
            let conn = match self.connections.get_mut(&addr) {
                Some(c) => c,
                None => continue,
            };

            // Tick fire damage (skip if Fire Resistance is active)
            let has_fire_res = conn.effects.iter().any(|e| {
                e.effect_id == mc_rs_proto::packets::mob_effect::effect_id::FIRE_RESISTANCE
            });
            if conn.fire_ticks > 0 {
                conn.fire_ticks -= 1;
                if !has_fire_res && conn.fire_ticks % 20 == 0 && conn.fire_ticks >= 0 {
                    // Deal 1 fire damage every second, reduced by Fire Protection
                    let nbt: Vec<&[u8]> = conn
                        .inventory
                        .armor
                        .iter()
                        .map(|i| i.nbt_data.as_slice())
                        .collect();
                    let fp_reduction = game_combat::fire_protection_reduction(&nbt);
                    let fire_dmg = 1.0 * (1.0 - fp_reduction);
                    conn.health = (conn.health - fire_dmg).max(0.0);
                    let rid = conn.entity_runtime_id;
                    let hp = conn.health;
                    let tick = conn.client_tick;
                    self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                        .await;
                    self.send_packet(
                        addr,
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(rid, hp, tick),
                    )
                    .await;
                    if hp <= 0.0 {
                        let name = self
                            .connections
                            .get(&addr)
                            .and_then(|c| c.login_data.as_ref())
                            .map(|d| d.display_name.clone())
                            .unwrap_or_default();
                        self.handle_player_death_with_message(
                            addr,
                            &format!("{name} burned to death"),
                        )
                        .await;
                        continue;
                    }
                }
            }

            // Tick effect durations
            let mut expired = Vec::new();
            if let Some(conn) = self.connections.get_mut(&addr) {
                for effect in &mut conn.effects {
                    effect.remaining_ticks -= 1;
                    if effect.remaining_ticks <= 0 {
                        expired.push(effect.effect_id);
                    }
                }
                conn.effects.retain(|e| e.remaining_ticks > 0);
            }

            // Send remove packets for expired effects
            if let Some(conn) = self.connections.get(&addr) {
                let rid = conn.entity_runtime_id;
                for eid in expired {
                    self.send_packet(addr, packets::id::MOB_EFFECT, &MobEffect::remove(rid, eid))
                        .await;
                }
            }
        }
    }

    /// Get combat bonuses from active effects for an attacker.
    /// Returns (strength_bonus, weakness_penalty).
    pub(super) fn get_attacker_bonuses(&self, addr: SocketAddr) -> (f32, f32) {
        use mc_rs_proto::packets::mob_effect::effect_id as eid;

        let conn = match self.connections.get(&addr) {
            Some(c) => c,
            None => return (0.0, 0.0),
        };

        let mut strength = 0.0_f32;
        let mut weakness = 0.0_f32;

        for effect in &conn.effects {
            match effect.effect_id {
                eid::STRENGTH => strength += 3.0 * (effect.amplifier as f32 + 1.0),
                eid::WEAKNESS => weakness += 4.0,
                _ => {}
            }
        }

        (strength, weakness)
    }

    /// Get damage resistance factor from victim's effects and armor.
    /// Returns the resistance factor (0.0 to 1.0).
    pub(super) fn get_resistance_factor(&self, addr: SocketAddr) -> f32 {
        use mc_rs_proto::packets::mob_effect::effect_id as eid;

        let conn = match self.connections.get(&addr) {
            Some(c) => c,
            None => return 0.0,
        };

        let mut factor = 0.0_f32;
        for effect in &conn.effects {
            if effect.effect_id == eid::RESISTANCE {
                factor += 0.2 * (effect.amplifier as f32 + 1.0);
            }
        }

        factor.min(1.0)
    }

    /// Tick survival mechanics: hunger drain, natural regen, starvation,
    /// drowning, lava damage, and suffocation. Called once per game tick (50ms).
    pub(super) async fn tick_survival(&mut self) {
        let addrs: Vec<SocketAddr> = self
            .connections
            .iter()
            .filter(|(_, c)| c.state == LoginState::InGame && !c.is_dead && c.gamemode == 0)
            .map(|(a, _)| *a)
            .collect();

        for addr in addrs {
            let (pos, tick, rid) = match self.connections.get(&addr) {
                Some(c) => (c.position, c.client_tick, c.entity_runtime_id),
                None => continue,
            };

            // --- Exhaustion drain ---
            let hunger_changed = {
                let conn = match self.connections.get_mut(&addr) {
                    Some(c) => c,
                    None => continue,
                };
                if conn.exhaustion >= 4.0 {
                    conn.exhaustion -= 4.0;
                    if conn.saturation > 0.0 {
                        conn.saturation = (conn.saturation - 1.0).max(0.0);
                    } else {
                        conn.food = (conn.food - 1).max(0);
                    }
                    true
                } else {
                    false
                }
            };
            if hunger_changed {
                let conn = match self.connections.get(&addr) {
                    Some(c) => c,
                    None => continue,
                };
                self.send_packet(
                    addr,
                    packets::id::UPDATE_ATTRIBUTES,
                    &UpdateAttributes::hunger(
                        rid,
                        conn.food as f32,
                        conn.saturation,
                        conn.exhaustion,
                        tick,
                    ),
                )
                .await;
            }

            // --- Natural regeneration (every 80 ticks = 4 seconds) ---
            let (food, health) = match self.connections.get(&addr) {
                Some(c) => (c.food, c.health),
                None => continue,
            };
            if food >= 18 && health < 20.0 && tick % 80 == 0 {
                let conn = match self.connections.get_mut(&addr) {
                    Some(c) => c,
                    None => continue,
                };
                conn.health = (conn.health + 1.0).min(20.0);
                conn.exhaustion += 6.0;
                let hp = conn.health;
                self.send_packet(
                    addr,
                    packets::id::UPDATE_ATTRIBUTES,
                    &UpdateAttributes::health(rid, hp, tick),
                )
                .await;
            }

            // --- Starvation (every 80 ticks, food == 0) ---
            let (food, health) = match self.connections.get(&addr) {
                Some(c) => (c.food, c.health),
                None => continue,
            };
            if food == 0 && tick % 80 == 0 && health > 1.0 {
                let conn = match self.connections.get_mut(&addr) {
                    Some(c) => c,
                    None => continue,
                };
                conn.health = (conn.health - 1.0).max(1.0);
                let hp = conn.health;
                self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                    .await;
                self.send_packet(
                    addr,
                    packets::id::UPDATE_ATTRIBUTES,
                    &UpdateAttributes::health(rid, hp, tick),
                )
                .await;
            }

            // --- Drowning ---
            const EYE_HEIGHT: f32 = 1.62;
            let head_y = pos.y; // position.y is already eye position in Bedrock
            let head_block_x = head_y.floor() as i32; // reuse for block coord
            let _ = head_block_x; // suppress warning, we compute below
            let hx = pos.x.floor() as i32;
            let hy = head_y.floor() as i32;
            let hz = pos.z.floor() as i32;

            let head_block_name = self
                .get_block(hx, hy, hz)
                .and_then(|hash| self.block_registry.get(hash))
                .map(|info| info.name);

            let in_water = matches!(
                head_block_name,
                Some("minecraft:water") | Some("minecraft:flowing_water")
            );

            let has_water_breathing = self
                .connections
                .get(&addr)
                .map(|c| {
                    c.effects.iter().any(|e| {
                        e.effect_id == mc_rs_proto::packets::mob_effect::effect_id::WATER_BREATHING
                    })
                })
                .unwrap_or(false);

            if in_water {
                if has_water_breathing {
                    if let Some(conn) = self.connections.get_mut(&addr) {
                        conn.air_ticks = 300;
                    }
                } else {
                    // Respiration: slow air consumption (skip decrement some ticks)
                    let resp_level = self
                        .connections
                        .get(&addr)
                        .map(|c| game_combat::respiration_level(&c.inventory.armor[0].nbt_data))
                        .unwrap_or(0);
                    let should_drain = resp_level == 0 || tick % (resp_level as u64 + 1) == 0;
                    if should_drain {
                        if let Some(conn) = self.connections.get_mut(&addr) {
                            conn.air_ticks -= 1;
                        }
                    }
                    let air = self
                        .connections
                        .get(&addr)
                        .map(|c| c.air_ticks)
                        .unwrap_or(0);
                    if air <= 0 && tick % 20 == 0 {
                        // Drowning: 2 damage per second
                        let conn = match self.connections.get_mut(&addr) {
                            Some(c) => c,
                            None => continue,
                        };
                        conn.health = (conn.health - 2.0).max(0.0);
                        let hp = conn.health;
                        self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                            .await;
                        self.send_packet(
                            addr,
                            packets::id::UPDATE_ATTRIBUTES,
                            &UpdateAttributes::health(rid, hp, tick),
                        )
                        .await;
                        if hp <= 0.0 {
                            let name = self
                                .connections
                                .get(&addr)
                                .and_then(|c| c.login_data.as_ref())
                                .map(|d| d.display_name.clone())
                                .unwrap_or_default();
                            self.handle_player_death_with_message(addr, &format!("{name} drowned"))
                                .await;
                            continue;
                        }
                    }
                }
            } else {
                // Recover air quickly out of water
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.air_ticks = (conn.air_ticks + 5).min(300);
                }
            }

            // --- Lava damage ---
            let feet_y = pos.y - EYE_HEIGHT;
            let fx = pos.x.floor() as i32;
            let fy = feet_y.floor() as i32;
            let fz = pos.z.floor() as i32;

            let feet_block_name = self
                .get_block(fx, fy, fz)
                .and_then(|hash| self.block_registry.get(hash))
                .map(|info| info.name);

            let in_lava = matches!(
                feet_block_name,
                Some("minecraft:lava") | Some("minecraft:flowing_lava")
            );

            if in_lava {
                // Set on fire
                if let Some(conn) = self.connections.get_mut(&addr) {
                    conn.fire_ticks = conn.fire_ticks.max(300);
                }

                let has_fire_res = self
                    .connections
                    .get(&addr)
                    .map(|c| {
                        c.effects.iter().any(|e| {
                            e.effect_id
                                == mc_rs_proto::packets::mob_effect::effect_id::FIRE_RESISTANCE
                        })
                    })
                    .unwrap_or(false);

                if !has_fire_res && tick % 10 == 0 {
                    // 4 damage every 0.5 seconds, reduced by Fire Protection
                    let fp_reduction = self
                        .connections
                        .get(&addr)
                        .map(|c| {
                            let nbt: Vec<&[u8]> = c
                                .inventory
                                .armor
                                .iter()
                                .map(|i| i.nbt_data.as_slice())
                                .collect();
                            game_combat::fire_protection_reduction(&nbt)
                        })
                        .unwrap_or(0.0);
                    let lava_dmg = 4.0 * (1.0 - fp_reduction);
                    let conn = match self.connections.get_mut(&addr) {
                        Some(c) => c,
                        None => continue,
                    };
                    conn.health = (conn.health - lava_dmg).max(0.0);
                    let hp = conn.health;
                    self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                        .await;
                    self.send_packet(
                        addr,
                        packets::id::UPDATE_ATTRIBUTES,
                        &UpdateAttributes::health(rid, hp, tick),
                    )
                    .await;
                    if hp <= 0.0 {
                        let name = self
                            .connections
                            .get(&addr)
                            .and_then(|c| c.login_data.as_ref())
                            .map(|d| d.display_name.clone())
                            .unwrap_or_default();
                        self.handle_player_death_with_message(
                            addr,
                            &format!("{name} tried to swim in lava"),
                        )
                        .await;
                        continue;
                    }
                }
            }

            // --- Suffocation (head inside solid block) ---
            // Check block at head position
            let head_solid = self
                .get_block(hx, hy, hz)
                .map(|hash| self.block_registry.is_solid(hash))
                .unwrap_or(false);

            if head_solid && tick % 10 == 0 {
                let conn = match self.connections.get_mut(&addr) {
                    Some(c) => c,
                    None => continue,
                };
                conn.health = (conn.health - 1.0).max(0.0);
                let hp = conn.health;
                self.broadcast_packet(packets::id::ENTITY_EVENT, &EntityEvent::hurt(rid))
                    .await;
                self.send_packet(
                    addr,
                    packets::id::UPDATE_ATTRIBUTES,
                    &UpdateAttributes::health(rid, hp, tick),
                )
                .await;
                if hp <= 0.0 {
                    let name = self
                        .connections
                        .get(&addr)
                        .and_then(|c| c.login_data.as_ref())
                        .map(|d| d.display_name.clone())
                        .unwrap_or_default();
                    self.handle_player_death_with_message(
                        addr,
                        &format!("{name} suffocated in a wall"),
                    )
                    .await;
                    continue;
                }
            }
        }
    }
}
