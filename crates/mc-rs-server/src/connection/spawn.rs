use std::sync::{Arc, Mutex};

use tracing::{debug, info};

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::login::*;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;
use mc_rs_proto::packets::world::*;

use crate::item_registry;
use crate::world::block_registry::BLOCKS;
use crate::world::chunk_cache::ChunkCache;
use crate::world::terrain_generator;

use super::{Connection, ConnectionState};

/// Encode UpdateAttributesPacket — format protocol 944 (dragonfly/gophertunnel).
///
/// Layout par attribut :
///   f32 Min, f32 Max, f32 Value, f32 DefaultMin, f32 DefaultMax, f32 Default,
///   string Name, VarU32 ModifierCount (+ modifiers)
///
/// PMMP 924 n'avait que `Min, Max, Value, Default` (4 floats). Protocol 944
/// a ajouté `DefaultMin` et `DefaultMax`. Sans ces deux champs, le client
/// désaligne le parsing et **crash** (observé : disconnect 280-400ms après
/// réception du paquet UpdateAttributes).
fn encode_update_attrs_inline(
    entity_runtime_id: u64,
    attrs: &[crate::attribute::Attribute],
) -> Vec<u8> {
    let mut w = mc_rs_proto::io::ProtoWriter::with_capacity(128);
    w.write_var_u64(entity_runtime_id);
    w.write_var_u32(attrs.len() as u32);
    for a in attrs {
        w.write_f32_le(a.min_value);
        w.write_f32_le(a.max_value);
        w.write_f32_le(a.current_value);
        // Protocol 944 : DefaultMin + DefaultMax juste avant Default.
        w.write_f32_le(a.min_value); // DefaultMin = same as Min (vanilla behavior)
        w.write_f32_le(a.max_value); // DefaultMax = same as Max
        w.write_f32_le(a.default_value);
        w.write_string(&a.id);
        w.write_var_u32(0); // modifier count
    }
    w.write_var_u64(0); // tick
    w.into_bytes()
}

pub(super) fn make_spawn_position(world_x: i32, world_y: i32, world_z: i32) -> [f32; 3] {
    let feet_y = (world_y + 1) as f32;
    [world_x as f32 + 0.5, feet_y + 1.621, world_z as f32 + 0.5]
}

fn find_surface_in_loaded_world(cache: &mut ChunkCache, world_x: i32, world_z: i32) -> Option<i32> {
    for world_y in (-64..=319).rev() {
        let block_id = cache.get_block(world_x, world_y, world_z);
        if block_id == BLOCKS.air || block_id == BLOCKS.water {
            continue;
        }
        // Exclure les non-solides (bamboo, fleurs, herbes, torches, vines…)
        // pour éviter un spawn perché sur un bambou ou une touffe d'herbe.
        let name = BLOCKS.name_for(block_id).unwrap_or("");
        if !crate::block_attachment::is_solid_support(name) {
            continue;
        }
        let head = cache.get_block(world_x, world_y + 1, world_z);
        let head_above = cache.get_block(world_x, world_y + 2, world_z);
        if head == BLOCKS.air && head_above == BLOCKS.air {
            return Some(world_y);
        }
    }

    None
}

pub(super) fn find_spawn_position(chunk_cache: &Arc<Mutex<ChunkCache>>, seed: u64) -> [f32; 3] {
    const SEARCH_STEP: i32 = 8;
    const MAX_RADIUS: i32 = 128;

    let mut fallback = None;
    if let Ok(mut cache) = chunk_cache.lock() {
        for radius in (0..=MAX_RADIUS).step_by(SEARCH_STEP as usize) {
            if radius == 0 {
                if let Some(surface_y) = find_surface_in_loaded_world(&mut cache, 0, 0) {
                    if surface_y > 62 {
                        return make_spawn_position(0, surface_y, 0);
                    }
                    fallback = Some((0, surface_y, 0));
                }
                continue;
            }

            for edge in (-radius..=radius).step_by(SEARCH_STEP as usize) {
                let perimeter_points = [
                    (-radius, edge),
                    (radius, edge),
                    (edge, -radius),
                    (edge, radius),
                ];

                for (world_x, world_z) in perimeter_points {
                    if let Some(surface_y) =
                        find_surface_in_loaded_world(&mut cache, world_x, world_z)
                    {
                        if fallback.is_none_or(|(_, best_y, _)| surface_y > best_y) {
                            fallback = Some((world_x, surface_y, world_z));
                        }

                        if surface_y > 62 {
                            return make_spawn_position(world_x, surface_y, world_z);
                        }
                    }
                }
            }
        }
    }

    if let Some((world_x, surface_y, world_z)) = fallback {
        make_spawn_position(world_x, surface_y, world_z)
    } else {
        terrain_generator::find_spawn_position(seed)
    }
}

impl Connection {
    pub(super) fn send_pre_spawn_packets(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();

        // StartGame
        let mut start_game =
            StartGame::default_with_id(self.entity_runtime_id as i64, self.position);
        start_game.player_gamemode = self.gamemode;
        start_game.world_gamemode = self.world_gamemode;
        start_game.difficulty = self.current_difficulty;
        start_game.world_name = self.config.world_name.clone();
        start_game.generator = self.config.generator_id;
        responses.push(self.encode_compressed_packet(packet_id::START_GAME, &start_game.encode()));

        responses.push(
            self.encode_compressed_packet(packet_id::ITEM_REGISTRY, item_registry::payload()),
        );

        // AvailableActorIdentifiers -- real NBT from PMMP
        static ENTITY_IDENTIFIERS_NBT: &[u8] = include_bytes!("../../data/entity_identifiers.nbt");
        responses.push(self.encode_compressed_packet(
            packet_id::AVAILABLE_ACTOR_IDENTIFIERS,
            ENTITY_IDENTIFIERS_NBT,
        ));

        // BiomeDefinitionList
        let mut biome_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        biome_writer.write_var_u32(0);
        biome_writer.write_var_u32(0);
        responses.push(
            self.encode_compressed_packet(
                packet_id::BIOME_DEFINITION_LIST,
                biome_writer.as_bytes(),
            ),
        );

        // 5. UpdateAttributes — format protocol 944 corrigé (6 floats/attr)
        let desync = self.attributes.drain_desync();
        if !desync.is_empty() {
            let payload = encode_update_attrs_inline(self.entity_runtime_id, &desync);
            responses.push(self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &payload));
        }

        // 6. AvailableCommands (PMMP PreSpawnPacketHandler.php:134-135 :
        //    syncAvailableCommands AVANT abilities). Envoi minimal — la liste
        //    complète sera resync plus tard via sync_available_commands_for_all.
        responses.push(self.encode_compressed_packet(
            packet_id::AVAILABLE_COMMANDS,
            &AvailableCommands::encode_rich(&[]),
        ));

        // 7. UpdateAbilities
        let is_op = self.is_op;
        let abilities = match self.gamemode {
            1 => UpdateAbilities::default_creative(self.entity_runtime_id as i64, is_op),
            3 => UpdateAbilities::default_spectator(self.entity_runtime_id as i64, is_op),
            _ => UpdateAbilities::default_survival(self.entity_runtime_id as i64, is_op),
        };
        responses
            .push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // 8. UpdateAdventureSettings
        let adventure = UpdateAdventureSettings::default_survival();
        responses.push(
            self.encode_compressed_packet(
                packet_id::UPDATE_ADVENTURE_SETTINGS,
                &adventure.encode(),
            ),
        );

        // 9. SetActorData
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_in_game(self.entity_runtime_id, &player_name);
        responses
            .push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // 10. Inventory sync (Main / UI 124 / Offhand / Armor)
        self.push_inventory_sync(&mut responses);

        // 11. MobEquipment (PMMP syncSelectedHotbarSlot — held item du player).
        //     Sans ce paquet, le client ne sait pas quel item est "en main"
        //     au spawn.
        let held_slot = self.inventory.held_slot;
        let held_stack_id = self
            .inventory_manager
            .stack_id_of(crate::inventory_manager::InvKey::Main, held_slot as usize);
        let held_wrapper = ItemStackWrapper {
            stack_id: held_stack_id,
            item: self.inventory.slots[held_slot as usize].item.clone(),
        };
        let mob_eq = MobEquipment::encode_item(self.entity_runtime_id, &held_wrapper, held_slot);
        responses.push(self.encode_compressed_packet(packet_id::MOB_EQUIPMENT, &mob_eq));

        // 12. CreativeContent — classification vanilla EXACTE chargée depuis
        //     `data/creative/{construction,nature,equipment,items}.json`
        //     (copiés depuis bedrock-data de PMMP). Chaque fichier définit
        //     les sous-groupes avec leurs icônes et noms i18n.
        let creative_groups = crate::creative_content::groups();
        let creative_items = crate::creative_content::items();
        let (n_groups, n_items) = crate::creative_content::stats();
        info!(
            "[{}] CreativeContent: {} groups + {} items (vanilla PMMP)",
            self.addr, n_groups, n_items
        );
        responses.push(self.encode_compressed_packet(
            packet_id::CREATIVE_CONTENT,
            &CreativeContent::encode(&creative_groups, &creative_items),
        ));

        // 13. CraftingData
        responses.push(
            self.encode_compressed_packet(packet_id::CRAFTING_DATA, &CraftingData::encode_empty()),
        );

        // 14. PlayerList (PMMP syncPlayerList — self-entry + autres joueurs).
        //     Protocol 944 ajoute un champ `color` u32 LE ARGB par entry (fix
        //     appliqué dans PlayerList::encode ; sans ça le client désaligne
        //     la boucle des verified-skins et crash juste après PreSpawn).
        let uuid_bytes: [u8; 16] = self
            .uuid
            .as_ref()
            .map(|u| *u.as_bytes())
            .unwrap_or([0u8; 16]);
        let self_xuid = self.xuid.clone().unwrap_or_default();
        let self_skin = self
            .skin
            .as_ref()
            .map(|s| s.to_serialized(&self_xuid))
            .unwrap_or_default();
        let self_entry = PlayerListAdd {
            uuid: uuid_bytes,
            entity_id: self.entity_runtime_id as i64,
            username: self.display_name.clone().unwrap_or_default(),
            xuid: self_xuid,
            platform_chat_id: String::new(),
            build_platform: 0,
            skin: self_skin,
            is_teacher: false,
            is_host: false,
            is_subclient: false,
        };
        let player_list = PlayerList {
            action: 0,
            entries: vec![self_entry],
        };
        responses
            .push(self.encode_compressed_packet(packet_id::PLAYER_LIST, &player_list.encode()));

        info!("[{}] Sent {} PreSpawn packets", self.addr, responses.len());

        responses
    }

    pub(super) fn handle_request_chunk_radius(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let radius = reader.read_var_i32().unwrap_or(4);
        let clamped = radius.clamp(2, self.config.max_view_distance);
        self.view_distance = clamped;
        info!(
            "[{}] RequestChunkRadius: {} (responding with {})",
            self.addr, radius, clamped
        );

        let mut responses = Vec::new();

        // ChunkRadiusUpdated
        let radius_pkt = ChunkRadiusUpdated { radius: clamped };
        responses.push(
            self.encode_compressed_packet(packet_id::CHUNK_RADIUS_UPDATED, &radius_pkt.encode()),
        );

        // NetworkChunkPublisherUpdate
        let spawn_x = self.position[0] as i32;
        let spawn_y = self.position[1] as i32;
        let spawn_z = self.position[2] as i32;
        let publisher = NetworkChunkPublisherUpdate {
            position: [spawn_x, spawn_y, spawn_z],
            radius: (clamped * 16) as u32,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &publisher.encode(),
        ));

        let spawn_chunk_x = spawn_x >> 4;
        let spawn_chunk_z = spawn_z >> 4;
        self.last_chunk_x = spawn_chunk_x;
        self.last_chunk_z = spawn_chunk_z;
        self.order_chunks();
        self.chunk_order_countdown = u32::MAX;
        responses.extend(self.send_chunk_batch());
        info!(
            "[{}] Queued spawn chunks (radius={}), first batch={}, remaining_queue={}",
            self.addr,
            clamped,
            self.sent_chunks.len(),
            self.chunk_load_queue.len()
        );

        // PMMP `notifyTerrainReady()` envoie PlayStatus(PlayerSpawn)
        // SEULEMENT après que les chunks essentiels soient streamés. Envoyer
        // PlayerSpawn ici (avec ~4 chunks envoyés sur 400+) fait croire au
        // client que le terrain est prêt → bloqué en "chargement du serveur".
        // Diffère via le flag : `send_queued_chunks` enverra PlayerSpawn
        // quand la queue sera vidée.
        self.state = ConnectionState::SpawnResponse;
        self.player_spawn_pending = true;
        debug!("[{}] -> SpawnResponse state (PlayerSpawn différé)", self.addr);

        responses
    }

    /// Re-handle RequestChunkRadius en cours de jeu : le client peut renvoyer
    /// le paquet quand l'utilisateur change la "Render Distance" dans les
    /// settings vidéo. Contrairement à la version PreSpawn, on ne renvoie pas
    /// PLAY_STATUS et on ne change pas l'état — on se contente de mettre à
    /// jour le radius, re-queuer les chunks dans la nouvelle frontière, et
    /// notifier le client via ChunkRadiusUpdated + NetworkChunkPublisherUpdate.
    /// `order_chunks` retire les chunks devenus hors-vue de `sent_chunks` et
    /// queue les nouveaux qui entrent en vue.
    pub(super) fn handle_request_chunk_radius_ingame(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        let radius = reader.read_var_i32().unwrap_or(self.view_distance);
        let clamped = radius.clamp(2, self.config.max_view_distance);
        if clamped == self.view_distance {
            return Vec::new();
        }
        let previous = self.view_distance;
        self.view_distance = clamped;
        info!(
            "[{}] RequestChunkRadius (in-game): {} → {} (was {})",
            self.addr, radius, clamped, previous
        );

        let mut responses = Vec::new();

        let radius_pkt = ChunkRadiusUpdated { radius: clamped };
        responses.push(
            self.encode_compressed_packet(packet_id::CHUNK_RADIUS_UPDATED, &radius_pkt.encode()),
        );

        let px = self.position[0] as i32;
        let py = self.position[1] as i32;
        let pz = self.position[2] as i32;
        let publisher = NetworkChunkPublisherUpdate {
            position: [px, py, pz],
            radius: (clamped * 16) as u32,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &publisher.encode(),
        ));

        // Recalcule la liste des chunks à streamer. Si on a réduit, les chunks
        // hors-vue sont retirés de `sent_chunks` ; si on a élargi, les nouveaux
        // sont push en queue. Le streaming est piloté par `send_queued_chunks`
        // au tick suivant.
        self.order_chunks();
        self.chunk_order_countdown = 0;

        responses
    }

    #[allow(dead_code)]
    pub(super) fn send_player_spawn(&mut self) -> Vec<Vec<u8>> {
        if self.state != ConnectionState::PreSpawn {
            return Vec::new();
        }
        info!("[{}] Sending PlayStatus(PLAYER_SPAWN)", self.addr);
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        let response =
            self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode());
        self.state = ConnectionState::SpawnResponse;
        debug!("[{}] -> SpawnResponse state", self.addr);
        vec![response]
    }

    pub(super) fn handle_set_local_player_as_initialized(&mut self) -> Vec<Vec<u8>> {
        info!(
            "[{}] {} is now in-game!",
            self.addr,
            self.display_name.as_deref().unwrap_or("Player")
        );
        self.state = ConnectionState::InGame;
        Vec::new()
    }
}
