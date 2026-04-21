use tracing::info;

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::item_entities::{item_stack_debug_summary, PendingItemEntitySpawn};
use crate::world::block_registry::BLOCKS;

use super::Connection;

impl Connection {
    pub(super) fn handle_player_auth_input(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = PlayerAuthInput::decode(reader) else {
            return Vec::new();
        };

        // Validate position (anti-cheat basics)
        if !pkt.position[0].is_finite()
            || !pkt.position[1].is_finite()
            || !pkt.position[2].is_finite()
        {
            return Vec::new(); // ignore invalid position
        }

        // Void kill check
        if pkt.position[1] < -128.0 {
            self.position = self.spawn_position;
            let reset = MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: 0.0,
                yaw: 0.0,
                head_yaw: 0.0,
                mode: 1, // reset
                on_ground: true,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Anti-fly: if player is moving UP by more than 1.5 blocks per tick,
        // clamp their Y to prevent fly hacking
        let dy = pkt.position[1] - self.position[1];
        if dy > 1.5 {
            // Player is rising too fast -- clamp Y to max jump height
            let clamped_y = self.position[1] + 1.5;
            self.position = [pkt.position[0], clamped_y, pkt.position[2]];
            // Send correction
            let reset = MovePlayer {
                runtime_entity_id: self.entity_runtime_id,
                position: self.position,
                pitch: pkt.pitch,
                yaw: pkt.yaw,
                head_yaw: pkt.head_yaw,
                mode: 1, // reset
                on_ground: false,
                riding_runtime_id: 0,
                tick: self.tick,
            };
            return vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &reset.encode())];
        }

        // Fall damage detection : on suit le pic Y pendant la chute ; à
        // l'atterrissage (Y stable après descente) on applique les dégâts.
        //
        // Survival only — le mode créatif/spectateur est exempt. Cette logique
        // touche directement `attributes.HEALTH` donc sera propagée au client
        // via UpdateAttributes dans le prochain `tick_game_state`.
        let dy = pkt.position[1] - self.position[1];
        if self.gamemode == 0 {
            if dy > 0.01 {
                // Monte (saut/eau) → reset fall tracking.
                self.fall_peak_y = None;
            } else if dy < -0.01 {
                // Descend → mémorise le pic (le max Y depuis le début de la chute).
                let peak = self.fall_peak_y.unwrap_or(self.position[1]);
                self.fall_peak_y = Some(peak.max(self.position[1]));
            } else if let Some(peak) = self.fall_peak_y.take() {
                // Y stable après chute → atterrissage.
                let fall_distance = peak - pkt.position[1];
                if fall_distance > crate::entity_fall::FALL_THRESHOLD {
                    let damage = crate::entity_fall::compute_damage(fall_distance, 0);
                    if damage > 0.0 {
                        let current = self
                            .attributes
                            .must_get(crate::attribute::ids::HEALTH)
                            .current_value;
                        let new_hp = (current - damage).max(0.0);
                        self.attributes
                            .must_get_mut(crate::attribute::ids::HEALTH)
                            .set_value(new_hp, true);
                        info!(
                            "[{}] Fall damage: distance={:.2}, damage={:.1}, hp={:.1}",
                            self.addr, fall_distance, damage, new_hp
                        );
                    }
                }
            }
        }

        // Exhaustion par mouvement (survival only) — PMMP `Human::onMovement`
        // utilise 0.005/block walk, 0.1/block sprint, 0.015/block swim.
        // On discrimine via `is_sprinting` pour respecter les valeurs vanilla.
        if self.gamemode == 0 {
            let dx = pkt.position[0] - self.position[0];
            let dz = pkt.position[2] - self.position[2];
            let horizontal = (dx * dx + dz * dz).sqrt();
            if horizontal > 0.01 {
                let rate = if self.is_sprinting {
                    0.1
                } else if self.is_swimming {
                    0.015
                } else {
                    0.005
                };
                self.hunger.exhaust(&mut self.attributes, horizontal * rate);
            }
        }

        // Update player position
        self.position = pkt.position;
        self.pitch = pkt.pitch;
        self.yaw = pkt.yaw;
        self.head_yaw = pkt.head_yaw;
        self.tick += 1;

        // Broadcast MovePlayer to all other players
        let move_pkt = MovePlayer {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            mode: 0, // normal
            on_ground: true,
            riding_runtime_id: 0,
            tick: self.tick,
        };
        self.broadcasts
            .push(self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode()));

        // Check if player moved to a new chunk -- queue chunks for tick-based sending
        let mut responses = Vec::new();
        let chunk_x = (self.position[0] as i32) >> 4;
        let chunk_z = (self.position[2] as i32) >> 4;

        if chunk_x != self.last_chunk_x || chunk_z != self.last_chunk_z {
            self.last_chunk_x = chunk_x;
            self.last_chunk_z = chunk_z;

            // CRITICAL: Tell client the new center of the chunk render area.
            let ncpu = NetworkChunkPublisherUpdate {
                position: [
                    self.position[0] as i32,
                    self.position[1] as i32,
                    self.position[2] as i32,
                ],
                radius: (self.view_distance * 16) as u32,
            };
            responses.push(self.encode_compressed_packet(
                packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                &ncpu.encode(),
            ));

            info!(
                "[{}] NCPU sent: pos=({},{},{}), radius={} blocks, chunk=({},{})",
                self.addr,
                self.position[0] as i32,
                self.position[1] as i32,
                self.position[2] as i32,
                self.view_distance * 16,
                chunk_x,
                chunk_z,
            );

            // PMMP: nextChunkOrderRun = 0 on chunk change
            self.chunk_order_countdown = 0;
        } else {
            // PMMP: nextChunkOrderRun = min(current, 20) on normal movement
            if self.chunk_order_countdown > 20 {
                self.chunk_order_countdown = 20;
            }
        }

        // Handle block actions (breaking/placing)
        for action in &pkt.block_actions {
            let bx = action.position[0];
            let by = action.position[1];
            let bz = action.position[2];
            info!(
                "[{}] block_action type={} pos=({},{},{}) face={}",
                self.addr, action.action_type, bx, by, bz, action.face
            );
            // PMMP uses integer block position (cast to float), NOT +0.5 center
            let block_pos = [bx as f32, by as f32, bz as f32];
            let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

            match action.action_type {
                // START_BREAK (0) or CONTINUE_DESTROY_BLOCK (27)
                0 | 27 => {
                    // PMMP InGamePacketHandler.php:678-685 : si on reçoit
                    // CONTINUE_DESTROY_BLOCK pour le MÊME bloc qu'on est déjà
                    // en train de casser, on IGNORE. Le client envoie
                    // spuriousement 27 pour le bloc courant, si on répond on
                    // reset son animation → crack repart à zéro → il ne casse
                    // jamais. (Source du bug "je mine indéfiniment sans rien
                    // obtenir" rapporté par l'utilisateur.)
                    if self.last_block_attacked == Some(action.position) {
                        continue;
                    }
                    self.last_block_attacked = Some(action.position);

                    // Break speed basé sur la hardness du bloc (par nom).
                    // PMMP : break_time_secs ≈ hardness × 1.5 (avec bon outil)
                    // ou × 5 (main nue / mauvais outil). event_data envoyé au
                    // client est `65535 × (1 / (break_time_secs × 20 TPS))`.
                    let block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                        cache.get_block(bx, by, bz)
                    } else {
                        0
                    };
                    let block_name = BLOCKS.name_for(block_id).unwrap_or("");
                    let h = crate::block_hardness::hardness(block_name);
                    info!(
                        "[{}] start_break pos=({},{},{}) rid={} name={:?} hardness={}",
                        self.addr, bx, by, bz, block_id, block_name, h
                    );
                    let break_speed: f32 = if h < 0.0 {
                        0.0 // unbreakable (bedrock, ...)
                    } else if h == 0.0 {
                        1.0 // instant-break (grass, air, plants)
                    } else {
                        // Main nue par défaut → facteur 5.
                        let secs = h * 5.0;
                        1.0 / (secs * 20.0)
                    };

                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_START_BREAK,
                        position: block_pos,
                        event_data: (65535.0 * break_speed) as i32,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // ABORT_BREAK (1) or STOP_BREAK (2)
                1 | 2 => {
                    self.last_block_attacked = None;
                    let event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
                        event_data: 0,
                    };
                    let event_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &event.encode());
                    responses.push(event_bytes.clone());
                    self.broadcasts.push(event_bytes);
                }

                // PREDICT_DESTROY_BLOCK (26)
                26 => {
                    // Le bloc vient d'être cassé, on peut en re-cibler un autre.
                    self.last_block_attacked = None;
                    let air_id = BLOCKS.air;

                    // Send BLOCK_STOP_BREAK to clear crack animation
                    let stop_event = LevelEvent {
                        event_id: LevelEvent::BLOCK_STOP_BREAK,
                        position: block_pos,
                        event_data: 0,
                    };
                    let stop_bytes =
                        self.encode_compressed_packet(packet_id::LEVEL_EVENT, &stop_event.encode());
                    responses.push(stop_bytes.clone());
                    self.broadcasts.push(stop_bytes);

                    // Get the old block ID and set to air
                    let old_block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                        let old = cache.get_block(bx, by, bz);
                        cache.set_block(bx, by, bz, air_id);
                        cache.save_chunk_now(bx.div_euclid(16), bz.div_euclid(16));
                        old
                    } else {
                        air_id
                    };

                    // Si c'était un furnace → queue unregister pour que main.rs
                    // retire l'entrée du FurnaceManager et drop les items in/fuel/output.
                    if crate::furnace::FurnaceKind::from_block_id(old_block_id).is_some() {
                        self.pending_furnace_events
                            .push(super::PendingFurnaceEvent::Unregister {
                                pos: (bx, by, bz),
                            });
                        info!(
                            "[{}] Queued furnace unregister at ({bx},{by},{bz})",
                            self.addr
                        );
                    }

                    // Send UpdateBlock
                    let update = UpdateBlock {
                        position: action.position,
                        runtime_id: air_id,
                        flags: 3, // FLAG_NEIGHBORS | FLAG_NETWORK
                        layer: 0,
                    };
                    let update_bytes =
                        self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
                    responses.push(update_bytes.clone());
                    self.broadcasts.push(update_bytes);

                    // Send block destroy particles + sound
                    if old_block_id != air_id {
                        let level_event = LevelEvent {
                            event_id: LevelEvent::PARTICLE_DESTROY,
                            position: block_center,
                            event_data: old_block_id as i32,
                        };
                        let event_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_EVENT,
                            &level_event.encode(),
                        );
                        responses.push(event_bytes.clone());
                        self.broadcasts.push(event_bytes);

                        let sound = LevelSoundEvent::block_sound(
                            LevelSoundEvent::BREAK,
                            block_center,
                            old_block_id as i32,
                        );
                        let sound_bytes = self.encode_compressed_packet(
                            packet_id::LEVEL_SOUND_EVENT,
                            &sound.encode(),
                        );
                        responses.push(sound_bytes.clone());
                        self.broadcasts.push(sound_bytes);
                    }

                    // Spawn a dropped item entity — règle vanilla :
                    // - Seuls les blocs dont l'outil requis est Pickaxe sont
                    //   REELLEMENT tool-gated (stone/cobblestone/ores/obsidian).
                    //   Les blocs Axe/Shovel/Shears sont juste "plus rapides"
                    //   avec le bon outil mais droppent à main nue aussi.
                    // - Tier check ne s'applique QUE sur les blocs pickaxe
                    //   (iron_ore → stone+, diamond_ore → iron+, obsidian → diamond+).
                    // En créatif on skip le drop entièrement.
                    if old_block_id != air_id && self.gamemode != 1 {
                        let block_name = crate::world::block_registry::BLOCKS
                            .name_for(old_block_id)
                            .unwrap_or("");
                        let held_item_id =
                            self.inventory.slots[self.inventory.held_slot as usize].item.id;
                        let held_tool = crate::durability::durable_info(held_item_id);
                        let needs_tool =
                            crate::block_hardness::required_tool_type(block_name);
                        let min_tier =
                            crate::block_hardness::min_tool_tier_for_drop(block_name);

                        // Seuls les blocs pickaxe-required sont gate-és côté outil.
                        let needs_pickaxe = matches!(
                            needs_tool,
                            Some(crate::durability::ToolType::Pickaxe)
                        );
                        let tool_ok = if needs_pickaxe {
                            matches!(
                                held_tool,
                                Some(crate::durability::DurableInfo {
                                    tool_type: crate::durability::ToolType::Pickaxe,
                                    ..
                                })
                            )
                        } else {
                            true // logs/dirt/sand/etc. droppent à main nue
                        };
                        let tier_ok = match (min_tier, held_tool) {
                            (None, _) => true,
                            (Some(req_tier), Some(info)) => {
                                info.tier.mining_tier() >= req_tier.mining_tier()
                            }
                            (Some(_), None) => false,
                        };

                        if tool_ok && tier_ok {
                            if let Some(drop_item) = crate::inventory::block_drop(old_block_id) {
                                let item_id = drop_item.id;
                                let drop_summary = item_stack_debug_summary(&drop_item);
                                self.pending_item_spawns
                                    .push(PendingItemEntitySpawn::with_scatter(
                                        drop_item,
                                        [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5],
                                    ));
                                info!(
                                    "[{}] Queued dropped item entity: item_id={} at ({}, {}, {}) :: {}",
                                    self.addr, item_id, bx, by, bz, drop_summary
                                );
                            } else {
                                info!(
                                    "[{}] Block {} broken but has no drop",
                                    self.addr, block_name
                                );
                            }
                        } else {
                            info!(
                                "[{}] Block {} broken but tool mismatch (tool_ok={}, tier_ok={}, held={:?}, need={:?}/{:?})",
                                self.addr,
                                block_name,
                                tool_ok,
                                tier_ok,
                                held_tool,
                                needs_tool,
                                min_tier
                            );
                        }
                    }

                    info!(
                        "[{}] Block broken at ({}, {}, {}) old_id={}",
                        self.addr, bx, by, bz, old_block_id
                    );

                    // Pop des blocs attachés qui ont perdu leur support.
                    // On check les 6 voisins : si leur règle d'attachement n'est
                    // plus satisfaite, on les casse récursivement (drops inclus).
                    let neighbors: [(i32, i32, i32); 6] = [
                        (bx, by + 1, bz),
                        (bx, by - 1, bz),
                        (bx + 1, by, bz),
                        (bx - 1, by, bz),
                        (bx, by, bz + 1),
                        (bx, by, bz - 1),
                    ];
                    let mut pop_queue: Vec<(i32, i32, i32)> = Vec::new();
                    for (nx, ny, nz) in neighbors {
                        if let Ok(mut cache) = self.chunk_cache.lock() {
                            let nid = cache.get_block(nx, ny, nz);
                            if nid == air_id {
                                continue;
                            }
                            let nname = BLOCKS.name_for(nid).unwrap_or("");
                            let Some(rule) = crate::block_attachment::attachment_rule(nname) else {
                                continue;
                            };
                            let ok = crate::block_attachment::check_support(
                                &mut cache, nx, ny, nz, rule,
                            );
                            if !ok {
                                pop_queue.push((nx, ny, nz));
                            }
                        }
                    }
                    for (nx, ny, nz) in pop_queue {
                        self.pop_attached_block(nx, ny, nz, &mut responses);
                    }

                    // Exhaustion mining (survival only) — PMMP `Human::onBlockBreak` : 0.005
                    // d'exhaustion par bloc cassé (en plus du damage à l'outil).
                    if self.gamemode == 0 && old_block_id != air_id {
                        self.hunger.exhaust(&mut self.attributes, 0.005);
                    }

                    // PMMP `BlockBreakEvent` (post-break, pour monitoring plugins).
                    if let Ok(mut ev_mgr) = self.events.lock() {
                        let mut ev = crate::event::block::BlockBreakEvent {
                            player_addr: self.addr,
                            position: [bx, by, bz],
                            old_block_runtime_id: old_block_id,
                            new_block_runtime_id: air_id,
                            drops: Vec::new(),
                            xp_drop: 0,
                            cancelled: false,
                        };
                        ev_mgr.call(&mut ev);
                    }

                    // PMMP `Block::onAttackedByTool` / `Durable::applyDamage` :
                    // si le held item est un outil durable, on décrémente sa
                    // durabilité de 1 (PMMP standard pour chaque bloc cassé).
                    // Si l'outil casse, on le remplace par air.
                    if self.gamemode != 1 {
                        let held_slot = self.inventory.held_slot as usize;
                        let held = &mut self.inventory.slots[held_slot];
                        if let Some(info) = crate::durability::durable_info(held.item.id) {
                            let broken = crate::durability::apply_damage(&mut held.item, 1);
                            if broken {
                                info!(
                                    "[{}] Tool broken (id={}, tier={:?})",
                                    self.addr, held.item.id, info.tier
                                );
                                // Replace par air + track via manager.
                                let new_item = mc_rs_proto::packets::player::ItemStack::AIR;
                                self.inventory_manager.set_slot(
                                    &mut self.inventory,
                                    crate::inventory_manager::InvKey::Main,
                                    held_slot,
                                    new_item,
                                );
                            } else {
                                // Slot changé (meta a bougé) : push pending_sync.
                                let current_item = held.item.clone();
                                self.inventory_manager.set_slot(
                                    &mut self.inventory,
                                    crate::inventory_manager::InvKey::Main,
                                    held_slot,
                                    current_item,
                                );
                            }
                        }
                    }
                }

                // RESPAWN (7) — client a cliqué Respawn après l'écran de mort.
                7 => {
                    let respawn_pkts = self.handle_respawn_request();
                    for pkt in respawn_pkts {
                        responses.push(pkt);
                    }
                }

                _ => {}
            }
        }

        // Handle block placement (item interaction with ACTION_CLICK_BLOCK)
        // State edge-triggered : sprint/sneak/swim. Les bits 25/26/27/28/29/30
        // de input_flags sont edge-triggered par le client (START_* / STOP_*).
        // On détecte le changement → update Connection state → broadcast
        // SetActorData aux autres viewers.
        let start_sprint = (pkt.input_flags >> 25) & 1 == 1;
        let stop_sprint = (pkt.input_flags >> 26) & 1 == 1;
        let start_sneak = (pkt.input_flags >> 27) & 1 == 1;
        let stop_sneak = (pkt.input_flags >> 28) & 1 == 1;
        let start_swim = (pkt.input_flags >> 29) & 1 == 1;
        let stop_swim = (pkt.input_flags >> 30) & 1 == 1;

        let mut state_changed = false;
        if start_sprint && !self.is_sprinting {
            self.is_sprinting = true;
            state_changed = true;
        }
        if stop_sprint && self.is_sprinting {
            self.is_sprinting = false;
            state_changed = true;
        }
        if start_sneak && !self.is_sneaking {
            self.is_sneaking = true;
            state_changed = true;
        }
        if stop_sneak && self.is_sneaking {
            self.is_sneaking = false;
            state_changed = true;
        }
        if start_swim && !self.is_swimming {
            self.is_swimming = true;
            state_changed = true;
        }
        if stop_swim && self.is_swimming {
            self.is_swimming = false;
            state_changed = true;
        }

        if state_changed {
            // Build SetActorData avec flags actuels (sprint, sneak, swim + base).
            use mc_rs_proto::packets::player::entity_flags;
            let mut flags = entity_flags::CAN_SHOW_NAMETAG
                | entity_flags::BREATHING
                | entity_flags::HAS_GRAVITY
                | entity_flags::HAS_COLLISION;
            if self.is_sprinting {
                flags |= entity_flags::SPRINTING;
            }
            if self.is_sneaking {
                flags |= entity_flags::SNEAKING;
            }
            let player_name = self.display_name.clone().unwrap_or_default();
            let actor_data = SetActorData {
                runtime_entity_id: self.entity_runtime_id,
                metadata: vec![(
                    0,
                    7, // MetadataValue::Long type id
                    mc_rs_proto::packets::player::MetadataValue::Long(flags),
                )],
                tick: self.tick,
            };
            let bytes = self
                .encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode());
            self.broadcasts.push(bytes);
        }

        if let Some(ref interaction) = pkt.item_interaction {
            info!(
                "[{}] item_interaction: action_type={} face={} hotbar_slot={}",
                self.addr, interaction.action_type, interaction.face, interaction.hotbar_slot
            );
            if interaction.action_type == 0 {
                // ACTION_CLICK_BLOCK : d'abord check si on clique sur un bed →
                // mise à jour du spawn (PMMP BedBlock::onInteract). Sinon on
                // tente une pose de bloc normale.
                let bx = interaction.block_position[0];
                let by = interaction.block_position[1];
                let bz = interaction.block_position[2];
                let clicked_id = if let Ok(mut cache) = self.chunk_cache.lock() {
                    cache.get_block(bx, by, bz)
                } else {
                    0
                };
                if BLOCKS.is_bed(clicked_id) {
                    self.spawn_position =
                        [bx as f32 + 0.5, by as f32 + 1.0, bz as f32 + 0.5];
                    info!(
                        "[{}] Bed interact → spawn override ({}, {}, {})",
                        self.addr, bx, by, bz
                    );
                    let msg = mc_rs_proto::packets::player::Text::system(
                        "§eRespawn point set",
                    );
                    responses.push(
                        self.encode_compressed_packet(packet_id::TEXT, &msg),
                    );
                } else {
                    self.handle_block_place(interaction, &mut responses);
                }
            }
        }

        // Handle inventory stack requests (slot movements)
        if let Some(ref request) = pkt.item_stack_request {
            info!(
                "[{}] item_stack_request id={} actions={}",
                self.addr,
                request.request_id,
                request.actions.len()
            );
            self.handle_item_stack_request(request, &mut responses);
        }

        responses
    }

    /// PlayerActionPacket standalone (0x24) handler. Port PMMP
    /// `InGamePacketHandler::handlePlayerAction` → `handlePlayerActionFromData`.
    /// Structure : actorRuntimeId(varU64) + action(varI32) + blockPos(3×VarInt)
    /// + resultPos(3×VarInt) + face(varI32). Action = PlayerAction constants.
    pub(super) fn handle_player_action(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let _actor_runtime_id = reader.read_var_u64().unwrap_or(0);
        let action = reader.read_var_i32().unwrap_or(-1);
        // blockPosition (3 VarInts)
        let _bx = reader.read_var_i32().unwrap_or(0);
        let _by = reader.read_var_u32().unwrap_or(0) as i32;
        let _bz = reader.read_var_i32().unwrap_or(0);
        // resultPosition (3 VarInts)
        let _rx = reader.read_var_i32().unwrap_or(0);
        let _ry = reader.read_var_u32().unwrap_or(0) as i32;
        let _rz = reader.read_var_i32().unwrap_or(0);
        let _face = reader.read_var_i32().unwrap_or(0);

        info!("[{}] PlayerAction action={}", self.addr, action);

        match action {
            // RESPAWN (7) — client clique "respawn" après l'écran de mort.
            // Duplique le handler qu'on a déjà dans block_actions (même action
            // peut arriver par les deux voies selon version client).
            7 => self.handle_respawn_request(),
            // 5/6 SLEEPING, 8 JUMP, 9/10 START/STOP_SPRINT, 11/12 START/STOP_SNEAK :
            // PMMP met à jour les metadata entity flags. Pas encore implémenté
            // côté mc-rs (nécessite SetActorData push aux autres viewers).
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_block_place(
        &mut self,
        interaction: &ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let held_slot_idx = self.inventory.held_slot as usize;
        let held = &self.inventory.slots[held_slot_idx];
        info!(
            "[{}] block_place attempt: held_slot={} item_id={} count={} face={} gamemode={}",
            self.addr, held_slot_idx, held.item.id, held.item.count, interaction.face, self.gamemode
        );
        if held.item.is_air() {
            info!("[{}] block_place: held is AIR, abort", self.addr);
            return;
        }

        // Check if held item is a placeable block
        let block_runtime_id = match crate::inventory::item_to_block(held.item.id) {
            Some(id) => id,
            None => {
                info!(
                    "[{}] block_place: item_id={} not in item_to_block map → NO decrement",
                    self.addr, held.item.id
                );
                return; // Not a block item
            }
        };

        // Calculate target position.
        //
        // PMMP / Allay pattern (cf Allay InventoryTransactionPacketProcessor:80-81
        // + PMMP World::useItemOn:2306-2308) :
        //   si le bloc CLIQUÉ est replaceable (tall_grass, fern, snow_layer, etc.)
        //     → place AU MÊME ENDROIT (remplace le bloc cliqué)
        //   sinon
        //     → offset par face (comportement normal)
        //
        // Sans cette logique, cliquer sur une touffe d'herbe plaçait la dirt
        // au-dessus (ou côté) de l'herbe au lieu de la remplacer → 2 blocs
        // visibles (le bloc posé + l'herbe toujours en place).
        let (dx, dy, dz) = match interaction.face {
            0 => (0, -1, 0), // down
            1 => (0, 1, 0),  // up
            2 => (0, 0, -1), // north
            3 => (0, 0, 1),  // south
            4 => (-1, 0, 0), // west
            5 => (1, 0, 0),  // east
            _ => return,
        };

        let cx = interaction.block_position[0];
        let cy = interaction.block_position[1];
        let cz = interaction.block_position[2];

        let (tx, ty, tz) = if let Ok(mut cache) = self.chunk_cache.lock() {
            let clicked_id = cache.get_block(cx, cy, cz);
            let clicked_name = BLOCKS.name_for(clicked_id).unwrap_or("");
            if crate::block_attachment::is_replaceable(clicked_name) {
                (cx, cy, cz)
            } else {
                (cx + dx, cy + dy, cz + dz)
            }
        } else {
            (cx + dx, cy + dy, cz + dz)
        };

        // Le TARGET doit être replaceable (air ou autre) pour accepter le
        // placement. Sinon on abort (empêche les doubles blocs).
        if let Ok(mut cache) = self.chunk_cache.lock() {
            let existing_id = cache.get_block(tx, ty, tz);
            let existing_name = BLOCKS.name_for(existing_id).unwrap_or("");
            if !crate::block_attachment::is_replaceable(existing_name) {
                info!(
                    "[{}] block_place: target ({},{},{}) non-replaceable ({:?}), abort",
                    self.addr, tx, ty, tz, existing_name
                );
                return;
            }
            cache.set_block(tx, ty, tz, block_runtime_id);
            cache.save_chunk_now(tx.div_euclid(16), tz.div_euclid(16));
        }

        // Send UpdateBlock
        let update = UpdateBlock {
            position: [tx, ty, tz],
            runtime_id: block_runtime_id,
            flags: 3,
            layer: 0,
        };
        let update_bytes = self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
        responses.push(update_bytes.clone());
        self.broadcasts.push(update_bytes);

        // Send place sound
        let block_center = [tx as f32 + 0.5, ty as f32 + 0.5, tz as f32 + 0.5];
        let sound = LevelSoundEvent::block_sound(
            LevelSoundEvent::PLACE,
            block_center,
            block_runtime_id as i32,
        );
        let sound_bytes =
            self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &sound.encode());
        responses.push(sound_bytes.clone());
        self.broadcasts.push(sound_bytes);

        // Decrement item count in inventory via manager (track + queue sync).
        // Survival only — en créatif/spectateur les items sont infinis.
        if self.gamemode == 0 {
            let slot = self.inventory.held_slot as usize;
            let (prev_count, new_count, was_air) = {
                let cur = &self.inventory.slots[slot].item;
                let was_air = cur.is_air();
                let prev = cur.count;
                let next = if prev > 1 { prev - 1 } else { 0 };
                (prev, next, was_air)
            };
            let new_item = if new_count == 0 {
                mc_rs_proto::packets::player::ItemStack::AIR
            } else {
                let mut n = self.inventory.slots[slot].item.clone();
                n.count = new_count;
                n
            };
            self.inventory_manager.set_slot(
                &mut self.inventory,
                crate::inventory_manager::InvKey::Main,
                slot,
                new_item,
            );
            info!(
                "[{}] block_place: decrement slot={} count={}→{} (was_air={})",
                self.addr, slot, prev_count, new_count, was_air
            );
        } else {
            info!(
                "[{}] block_place: gamemode={} (creative/other) — no decrement",
                self.addr, self.gamemode
            );
        }
        // Le sync sera émis à la fin du tick via flush_pending_updates (boucle
        // principale). Pas d'envoi inline ici.

        info!(
            "[{}] Block placed at ({}, {}, {}) block_id={}",
            self.addr, tx, ty, tz, block_runtime_id
        );

        // Si c'est un furnace → queue un event register pour que main.rs
        // l'ajoute au FurnaceManager (tick global 20 TPS).
        if let Some(kind) = crate::furnace::FurnaceKind::from_block_id(block_runtime_id) {
            self.pending_furnace_events
                .push(super::PendingFurnaceEvent::Register {
                    pos: (tx, ty, tz),
                    kind,
                });
            info!("[{}] Queued furnace register at ({tx},{ty},{tz})", self.addr);
        }

        // PMMP `BlockPlaceEvent` (post-place).
        if let Ok(mut ev_mgr) = self.events.lock() {
            let mut ev = crate::event::block::BlockPlaceEvent {
                player_addr: self.addr,
                position: [tx, ty, tz],
                block_runtime_id,
                replaced_block_runtime_id: BLOCKS.air,
                cancelled: false,
            };
            ev_mgr.call(&mut ev);
        }
    }

    /// Pop un bloc attaché qui a perdu son support (plante, torche, snow,
    /// etc.). Applique le drop standard + UpdateBlock + particules/son, puis
    /// relance le check de support sur les voisins (cascade naturelle).
    pub(super) fn pop_attached_block(
        &mut self,
        bx: i32,
        by: i32,
        bz: i32,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let air_id = BLOCKS.air;
        let old_block_id = if let Ok(mut cache) = self.chunk_cache.lock() {
            let old = cache.get_block(bx, by, bz);
            if old == air_id {
                return;
            }
            cache.set_block(bx, by, bz, air_id);
            cache.save_chunk_now(bx.div_euclid(16), bz.div_euclid(16));
            old
        } else {
            return;
        };

        let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

        let update = UpdateBlock {
            position: [bx, by, bz],
            runtime_id: air_id,
            flags: 3,
            layer: 0,
        };
        let bytes = self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update.encode());
        responses.push(bytes.clone());
        self.broadcasts.push(bytes);

        let particle = LevelEvent {
            event_id: LevelEvent::PARTICLE_DESTROY,
            position: block_center,
            event_data: old_block_id as i32,
        };
        let pbytes =
            self.encode_compressed_packet(packet_id::LEVEL_EVENT, &particle.encode());
        responses.push(pbytes.clone());
        self.broadcasts.push(pbytes);

        // Drop standard (pas de tool-gating : les plantes tombent toujours).
        if let Some(drop_item) = crate::inventory::block_drop(old_block_id) {
            self.pending_item_spawns
                .push(PendingItemEntitySpawn::with_scatter(
                    drop_item,
                    [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5],
                ));
        }

        info!(
            "[{}] Popped attached block at ({bx},{by},{bz}) old_id={}",
            self.addr, old_block_id
        );

        // Cascade : vérifier si d'autres blocs s'appuyaient sur celui-ci.
        let neighbors: [(i32, i32, i32); 6] = [
            (bx, by + 1, bz),
            (bx, by - 1, bz),
            (bx + 1, by, bz),
            (bx - 1, by, bz),
            (bx, by, bz + 1),
            (bx, by, bz - 1),
        ];
        let mut cascade: Vec<(i32, i32, i32)> = Vec::new();
        for (nx, ny, nz) in neighbors {
            if let Ok(mut cache) = self.chunk_cache.lock() {
                let nid = cache.get_block(nx, ny, nz);
                if nid == air_id {
                    continue;
                }
                let nname = BLOCKS.name_for(nid).unwrap_or("");
                let Some(rule) = crate::block_attachment::attachment_rule(nname) else {
                    continue;
                };
                let ok =
                    crate::block_attachment::check_support(&mut cache, nx, ny, nz, rule);
                if !ok {
                    cascade.push((nx, ny, nz));
                }
            }
        }
        for (nx, ny, nz) in cascade {
            self.pop_attached_block(nx, ny, nz, responses);
        }
    }
}
