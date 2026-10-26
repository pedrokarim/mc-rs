use tracing::{debug, info};

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::item_entities::PendingItemEntitySpawn;
use crate::world::block_registry::BLOCKS;

use super::Connection;

impl Connection {
    pub(super) fn handle_player_auth_input(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        // PlayerAuthInput arrive à chaque tick : la première erreur de décodage
        // du processus est journalisée en warn (diagnostic de format face à un
        // client réel), les suivantes en debug.
        static PAI_ERROR_REPORTED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        let report = |what: &str, err: &dyn std::fmt::Display| {
            if !PAI_ERROR_REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                tracing::warn!("[{}] PlayerAuthInput {}: {}", self.addr, what, err);
            } else {
                tracing::debug!("[{}] PlayerAuthInput {}: {}", self.addr, what, err);
            }
        };
        let pkt = match PlayerAuthInput::decode(reader) {
            Ok(pkt) => pkt,
            Err(e) => {
                report("decode failed", &e);
                return Vec::new();
            }
        };
        if let Some(err) = &pkt.trailing_error {
            report("optional blocks dropped", err);
        }

        // Validate position (anti-cheat basics)
        if !pkt.position[0].is_finite()
            || !pkt.position[1].is_finite()
            || !pkt.position[2].is_finite()
        {
            return Vec::new(); // ignore invalid position
        }

        // Void kill check
        if pkt.position[1] < -128.0 {
            // Tomber dans le vide du Nether/End ramène à l'overworld (au spawn).
            if self.current_dimension != 0 {
                let spawn = self.spawn_position;
                return self.change_dimension(0, spawn, true);
            }
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
        // Au sol : collision verticale sans montée (drapeau `VerticalCollision`, gophertunnel
        // `InputFlagVerticalCollision` = 50 ; heurter un plafond en montant ne compte pas).
        let vertical_collision = (pkt.input_flags >> 50) & 1 == 1;
        self.on_ground = vertical_collision && pkt.position[1] <= self.position[1];
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
            .push((packet_id::MOVE_PLAYER, move_pkt.encode()));

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

        // Actions de bloc (PMMP `handlePlayerBlockActions` → `handlePlayerActionFromData`).
        for action in &pkt.block_actions {
            debug!(
                "[{}] block_action type={} pos={:?} face={}",
                self.addr, action.action_type, action.position, action.face
            );
            // RESPAWN (7) — client a cliqué Respawn après l'écran de mort.
            if action.action_type == 7 {
                responses.extend(self.handle_respawn_request());
                continue;
            }
            if !self.handle_block_break_action(
                action.action_type,
                action.position,
                action.face,
                &mut responses,
            ) {
                debug!(
                    "[{}] action de bloc {} non gérée",
                    self.addr, action.action_type
                );
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
            let mut flags = entity_flags::PLAYER_BASE;
            if self.is_sprinting {
                flags |= entity_flags::SPRINTING;
            }
            if self.is_sneaking {
                flags |= entity_flags::SNEAKING;
            }
            let _player_name = self.display_name.clone().unwrap_or_default();
            let actor_data = SetActorData {
                runtime_entity_id: self.entity_runtime_id,
                metadata: vec![(
                    0,
                    7, // MetadataValue::Long type id
                    mc_rs_proto::packets::player::MetadataValue::Long(flags),
                )],
                tick: self.tick,
            };
            self.broadcasts
                .push((packet_id::SET_ACTOR_DATA, actor_data.encode()));
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
                let chest_id = BLOCKS.get("minecraft:chest");
                let trapped_chest_id = BLOCKS.get("minecraft:trapped_chest");
                let crafting_id = BLOCKS.get("minecraft:crafting_table");
                let furnace_id = BLOCKS.get("minecraft:furnace");
                let anvil_id = BLOCKS.get("minecraft:anvil");
                let chipped_anvil_id = BLOCKS.get("minecraft:chipped_anvil");
                let damaged_anvil_id = BLOCKS.get("minecraft:damaged_anvil");
                let enchanting_id = BLOCKS.get("minecraft:enchanting_table");
                if BLOCKS.is_bed(clicked_id) {
                    self.spawn_position = [bx as f32 + 0.5, by as f32 + 1.0, bz as f32 + 0.5];
                    info!(
                        "[{}] Bed interact → spawn override ({}, {}, {})",
                        self.addr, bx, by, bz
                    );
                    let msg = mc_rs_proto::packets::player::Text::system("§eRespawn point set");
                    responses.push(self.encode_compressed_packet(packet_id::TEXT, &msg));
                } else if clicked_id == chest_id || clicked_id == trapped_chest_id {
                    self.pending_chest_open = Some((bx, by, bz));
                    info!(
                        "[{}] Chest interact request at ({}, {}, {}) — queued",
                        self.addr, bx, by, bz
                    );
                } else if clicked_id == crafting_id {
                    self.pending_block_ui_open =
                        Some(super::PendingBlockUiOpen::CraftingTable { pos: (bx, by, bz) });
                    info!(
                        "[{}] Crafting table interact at ({},{},{})",
                        self.addr, bx, by, bz
                    );
                } else if clicked_id == anvil_id
                    || clicked_id == chipped_anvil_id
                    || clicked_id == damaged_anvil_id
                {
                    self.pending_block_ui_open =
                        Some(super::PendingBlockUiOpen::Anvil { pos: (bx, by, bz) });
                    info!("[{}] Anvil interact at ({},{},{})", self.addr, bx, by, bz);
                } else if clicked_id == enchanting_id {
                    self.pending_block_ui_open =
                        Some(super::PendingBlockUiOpen::Enchanting { pos: (bx, by, bz) });
                    info!(
                        "[{}] Enchanting table interact at ({},{},{})",
                        self.addr, bx, by, bz
                    );
                } else if clicked_id == furnace_id {
                    self.pending_block_ui_open =
                        Some(super::PendingBlockUiOpen::Furnace { pos: (bx, by, bz) });
                    info!("[{}] Furnace interact at ({},{},{})", self.addr, bx, by, bz);
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

        // Portail Nether : si le joueur se tient dans un bloc de portail, avance le
        // timer et déclenche le transfert le cas échéant (DIM-F).
        let mut portal_resp = self.check_nether_portal();
        responses.append(&mut portal_resp);

        // Portail End : entrer dans un bloc end_portal déclenche le voyage (DIM-H).
        let mut end_resp = self.check_end_portal();
        responses.append(&mut end_resp);

        responses
    }

    /// PlayerActionPacket standalone (0x24) handler. Port PMMP
    /// `InGamePacketHandler::handlePlayerAction` → `handlePlayerActionFromData`.
    /// Structure : actorRuntimeId(varU64) + action(varI32) + blockPos(3×VarInt)
    /// + resultPos(3×VarInt) + face(varI32). Action = PlayerAction constants.
    pub(super) fn handle_player_action(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let _actor_runtime_id = reader.read_var_u64().unwrap_or(0);
        let action = reader.read_var_i32().unwrap_or(-1);
        // BlockPos = 3 var_i32 SIGNED (gophertunnel BlockPos / PMMP
        // CommonTypes::getBlockPosition). Lire Y en unsigned décale
        // toutes les coords (zig-zag misread, Y=68 → 136).
        let _bx = reader.read_var_i32().unwrap_or(0);
        let _by = reader.read_var_i32().unwrap_or(0);
        let _bz = reader.read_var_i32().unwrap_or(0);
        let _rx = reader.read_var_i32().unwrap_or(0);
        let _ry = reader.read_var_i32().unwrap_or(0);
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

    /// BlockActorData (0x38) S→C ET C→S. Reçu quand un client édite un bloc
    /// avec NBT (sign, item_frame, command_block). Format :
    ///   BlockPos (3 var_i32) + raw network NBT (terminé par TAG_End).
    /// On le stocke dans pending_block_actor_updates pour que main.rs broadcast
    /// + persist dans SignManager.
    pub(super) fn handle_block_actor_data(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let bx = reader.read_var_i32().unwrap_or(0);
        let by = reader.read_var_i32().unwrap_or(0);
        let bz = reader.read_var_i32().unwrap_or(0);
        // Le reste = NBT brut jusqu'à TAG_End (auto-terminated).
        let nbt_bytes = reader.remaining_bytes().to_vec();
        info!(
            "[{}] BlockActorData at ({},{},{}) nbt_len={}",
            self.addr,
            bx,
            by,
            bz,
            nbt_bytes.len()
        );
        self.pending_block_actor_updates
            .push(super::PendingBlockActorUpdate {
                position: (bx, by, bz),
                nbt: nbt_bytes,
            });
        Vec::new()
    }

    /// Allumage d'un portail Nether (flint&steel sur obsidienne) : détecte un cadre
    /// valide et le remplit de blocs portail ; sinon pose un bloc de feu. (DIM-F)
    fn try_light_nether_portal(
        &mut self,
        interaction: &ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let cx = interaction.block_position[0];
        let cy = interaction.block_position[1];
        let cz = interaction.block_position[2];
        let (dx, dy, dz) = match interaction.face {
            0 => (0, -1, 0),
            1 => (0, 1, 0),
            2 => (0, 0, -1),
            3 => (0, 0, 1),
            4 => (-1, 0, 0),
            5 => (1, 0, 0),
            _ => (0, 1, 0),
        };
        let (ax, ay, az) = (cx + dx, cy + dy, cz + dz);

        let obsidian = BLOCKS.get("minecraft:obsidian");
        let portal_id = BLOCKS.get("minecraft:portal");
        let fire_id = BLOCKS.get("minecraft:fire");

        let clicked_is_obsidian = {
            let mut cache = self.chunk_cache.lock().unwrap();
            cache.get_block(cx, cy, cz) == obsidian
        };
        if !clicked_is_obsidian {
            return;
        }

        let cells = {
            let mut cache = self.chunk_cache.lock().unwrap();
            crate::world::nether_portal_build::try_ignite(&mut cache, ax, ay, az)
        };

        match cells {
            Some(cells) => {
                for c in cells {
                    let update = UpdateBlock {
                        position: c,
                        runtime_id: portal_id,
                        flags: 3,
                        layer: 0,
                    };
                    let payload = update.encode();
                    responses
                        .push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &payload));
                    self.broadcasts.push((packet_id::UPDATE_BLOCK, payload));
                }
                info!(
                    "[{}] Nether portal lit at ({},{},{})",
                    self.addr, ax, ay, az
                );
            }
            None => {
                // Pas de cadre valide : flint&steel pose simplement du feu.
                let placed = {
                    let mut cache = self.chunk_cache.lock().unwrap();
                    if cache.get_block(ax, ay, az) == BLOCKS.air {
                        cache.set_block(ax, ay, az, fire_id);
                        true
                    } else {
                        false
                    }
                };
                if placed {
                    let update = UpdateBlock {
                        position: [ax, ay, az],
                        runtime_id: fire_id,
                        flags: 3,
                        layer: 0,
                    };
                    let payload = update.encode();
                    responses
                        .push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &payload));
                    self.broadcasts.push((packet_id::UPDATE_BLOCK, payload));
                }
            }
        }
    }

    /// Eye of ender sur un cadre `end_portal_frame` : forme le portail End si
    /// l'anneau de 12 cadres est complet, et diffuse les 9 blocs `end_portal`. (DIM-H)
    fn try_form_end_portal_frame(
        &mut self,
        interaction: &ItemInteractionData,
        responses: &mut Vec<Vec<u8>>,
    ) {
        let cx = interaction.block_position[0];
        let cy = interaction.block_position[1];
        let cz = interaction.block_position[2];

        let clicked_is_frame = {
            let mut cache = self.chunk_cache.lock().unwrap();
            cache.get_block(cx, cy, cz) == BLOCKS.get("minecraft:end_portal_frame")
        };
        if !clicked_is_frame {
            return;
        }

        let cells = {
            let mut cache = self.chunk_cache.lock().unwrap();
            crate::world::end_portal_build::try_form_end_portal(&mut cache, cx, cy, cz)
        };
        if let Some(cells) = cells {
            let portal_id = BLOCKS.get("minecraft:end_portal");
            for c in cells {
                let update = UpdateBlock {
                    position: c,
                    runtime_id: portal_id,
                    flags: 3,
                    layer: 0,
                };
                let payload = update.encode();
                responses.push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &payload));
                self.broadcasts.push((packet_id::UPDATE_BLOCK, payload));
            }
            info!(
                "[{}] End portal formed near ({},{},{})",
                self.addr, cx, cy, cz
            );
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
            self.addr,
            held_slot_idx,
            held.item.id,
            held.item.count,
            interaction.face,
            self.gamemode
        );
        if held.item.is_air() {
            info!("[{}] block_place: held is AIR, abort", self.addr);
            return;
        }

        // Allumage d'un portail Nether : flint&steel utilisé sur de l'obsidienne.
        if crate::item_registry::item_name_by_id(held.item.id) == Some("minecraft:flint_and_steel")
        {
            self.try_light_nether_portal(interaction, responses);
            return;
        }

        // Eye of ender sur un cadre de portail End : forme le portail si l'anneau
        // de 12 cadres est complet (DIM-H, simplifié — pas de suivi par-œil).
        if crate::item_registry::item_name_by_id(held.item.id) == Some("minecraft:ender_eye") {
            self.try_form_end_portal_frame(interaction, responses);
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
        // Waterlogging (BDS) : un bloc qui peut contenir de l'eau, posé dans une source d'eau ou à la
        // place d'un bloc gorgé d'eau, la garde dans la seconde couche ; sinon l'eau disparaît.
        let mut liquid_change = None;
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
            let liquid_before = cache.get_liquid(tx, ty, tz);
            let source = if crate::block_liquid::is_water_source(existing_id) {
                existing_id
            } else {
                liquid_before
            };
            cache.set_block(tx, ty, tz, block_runtime_id);
            // set_block marque déjà le chunk dirty → autosave. PAS de
            // save_chunk_now (flush LevelDB synchrone sous guard = gel).
            if source != BLOCKS.air && crate::block_liquid::can_contain_liquid(block_runtime_id) {
                cache.set_liquid(tx, ty, tz, source);
            }
            let liquid_after = cache.get_liquid(tx, ty, tz);
            if liquid_after != liquid_before {
                liquid_change = Some(liquid_after);
            }
        }

        // Send UpdateBlock
        let update = UpdateBlock {
            position: [tx, ty, tz],
            runtime_id: block_runtime_id,
            flags: 3,
            layer: 0,
        };
        let update_payload = update.encode();
        responses.push(self.encode_compressed_packet(packet_id::UPDATE_BLOCK, &update_payload));
        self.broadcasts
            .push((packet_id::UPDATE_BLOCK, update_payload));
        if let Some(liquid) = liquid_change {
            self.broadcast_block_layer([tx, ty, tz], liquid, 1, responses);
        }

        // Send place sound
        let block_center = [tx as f32 + 0.5, ty as f32 + 0.5, tz as f32 + 0.5];
        let sound = LevelSoundEvent::block_sound(
            LevelSoundEvent::PLACE,
            block_center,
            block_runtime_id as i32,
        );
        let sound_payload = sound.encode();
        responses.push(self.encode_compressed_packet(packet_id::LEVEL_SOUND_EVENT, &sound_payload));
        self.broadcasts
            .push((packet_id::LEVEL_SOUND_EVENT, sound_payload));

        // Decrement item count in inventory via manager (track + queue sync).
        // Survival only — en créatif/spectateur les items sont infinis.
        if self.gamemode == 0 {
            let slot = self.inventory.held_slot as usize;
            let (prev_count, new_count, was_air) = {
                let cur = &self.inventory.slots[slot].item;
                let was_air = cur.is_air();
                let prev = cur.count;
                let next = prev.saturating_sub(1);
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
            info!(
                "[{}] Queued furnace register at ({tx},{ty},{tz})",
                self.addr
            );
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
            old
        } else {
            return;
        };
        // Le bloc disparaît ; une plante immergée (herbe marine, algue…) laisse son eau.
        self.break_block_and_broadcast([bx, by, bz], responses);

        let block_center = [bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5];

        let particle = LevelEvent {
            event_id: LevelEvent::PARTICLE_DESTROY,
            position: block_center,
            event_data: old_block_id as i32,
        };
        let particle_payload = particle.encode();
        responses.push(self.encode_compressed_packet(packet_id::LEVEL_EVENT, &particle_payload));
        self.broadcasts
            .push((packet_id::LEVEL_EVENT, particle_payload));

        // Drops sans outil : PMMP `Block::onNearbyBlockChange` → `World::useBreakOn($pos)` sans
        // joueur ni item (`VanillaItems::AIR()`).
        for drop_item in crate::block_break::drop_stacks(
            old_block_id,
            &crate::block_break::HeldTool::hand(),
            &mut rand::thread_rng(),
        ) {
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
                let ok = crate::block_attachment::check_support(&mut cache, nx, ny, nz, rule);
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
