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

pub(super) fn make_spawn_position(world_x: i32, world_y: i32, world_z: i32) -> [f32; 3] {
    let feet_y = (world_y + 1) as f32;
    [world_x as f32 + 0.5, feet_y + 1.621, world_z as f32 + 0.5]
}

pub fn hub_menu_item_id() -> i32 {
    item_registry::required_item_id("minecraft:compass")
}

fn find_surface_in_loaded_world(cache: &mut ChunkCache, world_x: i32, world_z: i32) -> Option<i32> {
    for world_y in (-64..=319).rev() {
        let block_id = cache.get_block(world_x, world_y, world_z);
        if block_id != BLOCKS.air && block_id != BLOCKS.water {
            let head = cache.get_block(world_x, world_y + 1, world_z);
            let head_above = cache.get_block(world_x, world_y + 2, world_z);
            if head == BLOCKS.air && head_above == BLOCKS.air {
                return Some(world_y);
            }
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
    pub(super) fn send_pre_spawn_packets(&self) -> Vec<Vec<u8>> {
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

        // BiomeDefinitionList -- empty (protocol 924 custom format)
        let mut biome_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        biome_writer.write_var_u32(0);
        biome_writer.write_var_u32(0);
        responses.push(
            self.encode_compressed_packet(
                packet_id::BIOME_DEFINITION_LIST,
                biome_writer.as_bytes(),
            ),
        );

        // 5. UpdateAttributes -- health, hunger, movement speed (BEFORE abilities per PMMP)
        let attributes = UpdateAttributes::default_survival(self.entity_runtime_id);
        responses.push(
            self.encode_compressed_packet(packet_id::UPDATE_ATTRIBUTES, &attributes.encode()),
        );

        // 6. AvailableCommands are synced after spawn from the shared command map.

        // 7. UpdateAbilities -- based on player's gamemode
        let abilities = if self.gamemode == 1 {
            UpdateAbilities::default_creative(self.entity_runtime_id as i64)
        } else {
            UpdateAbilities::default_survival(self.entity_runtime_id as i64)
        };
        responses
            .push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // 8. UpdateAdventureSettings -- PMMP sends this right after abilities
        let adventure = UpdateAdventureSettings::default_survival();
        responses.push(
            self.encode_compressed_packet(
                packet_id::UPDATE_ADVENTURE_SETTINGS,
                &adventure.encode(),
            ),
        );

        // 9. SetActorData -- entity metadata (gravity, breathing, collision)
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = SetActorData::player_in_game(self.entity_runtime_id, &player_name);
        responses
            .push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // 9. Inventory sync (PMMP syncAll + syncSelectedHotbarSlot)
        self.push_inventory_sync(&mut responses);

        // 10. CraftingData (empty)
        responses.push(
            self.encode_compressed_packet(packet_id::CRAFTING_DATA, &CraftingData::encode_empty()),
        );

        // 10. CreativeContent (empty)
        responses.push(self.encode_compressed_packet(
            packet_id::CREATIVE_CONTENT,
            &CreativeContent::encode_empty(),
        ));

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

        // PLAYER_SPAWN -- send after chunks
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        responses
            .push(self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode()));
        self.state = ConnectionState::SpawnResponse;
        debug!("[{}] -> SpawnResponse state", self.addr);

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

        let welcome =
            Text::system("Use /menu or right-click the compass in slot 1 to open the hub menu.");
        vec![self.encode_compressed_packet(packet_id::TEXT, &welcome)]
    }
}
