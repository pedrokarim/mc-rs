use tracing::{info, warn};

use super::Connection;
use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::*;

impl Connection {
    pub(super) fn handle_text(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(pkt) = Text::decode(reader) else {
            return Vec::new();
        };

        let player_name = self
            .display_name
            .clone()
            .unwrap_or_else(|| "Player".to_string());
        let xuid = self.xuid.clone().unwrap_or_default();

        // Check for commands (in case client sends via Text instead of CommandRequest)
        if pkt.message.starts_with('/') {
            self.pending_commands.push(pkt.message);
            return Vec::new();
        }

        info!("[CHAT] {}: {}", player_name, pkt.message);

        // Broadcast chat to all players (including self)
        let chat = Text::chat(&player_name, &pkt.message, &xuid);
        self.broadcasts
            .push(self.encode_compressed_packet(packet_id::TEXT, &chat));

        Vec::new()
    }

    pub(super) fn handle_command_request(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(command) = reader.read_string() else {
            warn!("[{}] Failed to read command string", self.addr);
            return Vec::new();
        };

        info!("[{}] CommandRequest received: {}", self.addr, command);
        self.pending_commands.push(command);
        Vec::new()
    }

    pub fn encode_system_message(&self, message: impl Into<String>) -> Vec<u8> {
        let msg = Text::system(&message.into());
        self.encode_compressed_packet(packet_id::TEXT, &msg)
    }

    pub(super) fn push_system_message(
        &self,
        responses: &mut Vec<Vec<u8>>,
        message: impl Into<String>,
    ) {
        responses.push(self.encode_system_message(message));
    }

    pub fn teleport_to(&mut self, position: [f32; 3]) -> Vec<Vec<u8>> {
        self.position = position;
        let move_pkt = MovePlayer {
            runtime_entity_id: self.entity_runtime_id,
            position: self.position,
            pitch: self.pitch,
            yaw: self.yaw,
            head_yaw: self.head_yaw,
            mode: 2,
            on_ground: true,
            riding_runtime_id: 0,
            tick: self.tick,
        };
        vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode())]
    }

    pub fn apply_gamemode_packets(&mut self, mode: i32) -> Vec<Vec<u8>> {
        self.apply_gamemode(mode)
    }

    /// Change the player's gamemode (PMMP syncGameMode flow).
    pub(super) fn apply_gamemode(&mut self, mode: i32) -> Vec<Vec<u8>> {
        let old_mode = self.gamemode;
        self.gamemode = mode;
        let mut responses = Vec::new();

        // 1. SetPlayerGameType -- single VarInt32
        let mut gt_writer = mc_rs_proto::io::ProtoWriter::with_capacity(4);
        gt_writer.write_var_i32(mode);
        responses.push(
            self.encode_compressed_packet(packet_id::SET_PLAYER_GAME_TYPE, gt_writer.as_bytes()),
        );

        // 2. UpdateAbilities -- per-gamemode
        let abilities = match mode {
            1 => UpdateAbilities::default_creative(self.entity_runtime_id as i64),
            3 => UpdateAbilities::default_spectator(self.entity_runtime_id as i64),
            _ => UpdateAbilities::default_survival(self.entity_runtime_id as i64),
        };
        responses
            .push(self.encode_compressed_packet(packet_id::UPDATE_ABILITIES, &abilities.encode()));

        // 3. UpdateAdventureSettings
        let adventure = UpdateAdventureSettings::default_survival();
        responses.push(
            self.encode_compressed_packet(
                packet_id::UPDATE_ADVENTURE_SETTINGS,
                &adventure.encode(),
            ),
        );

        // 4. SetActorData -- update collision/silent flags for spectator
        let player_name = self.display_name.clone().unwrap_or_default();
        let actor_data = if mode == 3 {
            SetActorData::player_spectator(self.entity_runtime_id, &player_name)
        } else {
            SetActorData::player_in_game(self.entity_runtime_id, &player_name)
        };
        responses
            .push(self.encode_compressed_packet(packet_id::SET_ACTOR_DATA, &actor_data.encode()));

        // 5. Broadcast despawn/respawn to other players
        if mode == 3 && old_mode != 3 {
            // Entering spectator -> despawn from others
            let remove = RemoveEntity {
                entity_unique_id: self.entity_runtime_id as i64,
            }
            .encode();
            self.broadcasts
                .push(self.encode_compressed_packet(packet_id::REMOVE_ACTOR, &remove));
        } else if mode != 3 && old_mode == 3 {
            // Leaving spectator -> respawn to others
            let uuid = self.uuid.map(|u| *u.as_bytes()).unwrap_or([0u8; 16]);
            let add = AddPlayer {
                uuid,
                username: player_name.clone(),
                runtime_entity_id: self.entity_runtime_id,
                platform_chat_id: String::new(),
                position: self.position,
                velocity: [0.0, 0.0, 0.0],
                pitch: self.pitch,
                yaw: self.yaw,
                head_yaw: self.head_yaw,
                gamemode: mode,
                entity_unique_id: self.entity_runtime_id as i64,
                permission_level: 1,
                command_permission: 0,
            }
            .encode();
            self.broadcasts
                .push(self.encode_compressed_packet(packet_id::ADD_PLAYER, &add));
        }

        info!(
            "[{}] Gamemode changed to {} for {}",
            self.addr,
            mode,
            self.display_name.as_deref().unwrap_or("Player")
        );

        responses
    }
}
