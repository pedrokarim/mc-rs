use serde_json::json;
use tracing::{debug, warn};

use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::forms::*;
use mc_rs_proto::packets::packet_id;

use crate::world::biome;
use crate::world::terrain_generator;

use super::{Connection, PendingForm};

impl Connection {
    pub(super) fn open_form(&mut self, form: PendingForm, form_data: String) -> Vec<Vec<u8>> {
        let form_id = self.next_form_id;
        self.next_form_id = self.next_form_id.wrapping_add(1).max(1);
        self.pending_forms.insert(form_id, form);

        let request = ModalFormRequest { form_id, form_data };
        vec![self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &request.encode())]
    }

    pub(super) fn open_hub_menu(&mut self) -> Vec<Vec<u8>> {
        let form_json = json!({
            "type": "form",
            "title": "§l§bMC-RS Hub",
            "content": "§7Prototype de menu Bedrock inspire des hubs type Hive.\n§fCompass: slot 1\n§8Version simple sans resource pack custom.",
            "buttons": [
                { "text": "§lSpawn Plaza\n§r§7Retourner au spawn" },
                { "text": "§lCreative Flight\n§r§7Passer en creatif" },
                { "text": "§lSurvival Loop\n§r§7Revenir en survie" },
                { "text": "§lBiome Scanner\n§r§7Afficher le biome courant" }
            ]
        });

        self.open_form(PendingForm::HubMenu, form_json.to_string())
    }

    fn handle_hub_menu_selection(&mut self, button_index: u32) -> Vec<Vec<u8>> {
        match button_index {
            0 => {
                self.position = self.spawn_position;
                let move_pkt = mc_rs_proto::packets::player::MovePlayer {
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
                let mut responses =
                    vec![self.encode_compressed_packet(packet_id::MOVE_PLAYER, &move_pkt.encode())];
                self.push_system_message(&mut responses, "Teleported to spawn plaza.");
                responses
            }
            1 => {
                let mut responses = self.apply_gamemode(1);
                self.push_system_message(
                    &mut responses,
                    "Creative mode enabled from the hub menu.",
                );
                responses
            }
            2 => {
                let mut responses = self.apply_gamemode(0);
                self.push_system_message(&mut responses, "Back to survival mode.");
                responses
            }
            3 => {
                let world_x = self.position[0].floor() as i32;
                let world_z = self.position[2].floor() as i32;
                let debug = terrain_generator::get_biome_debug_info(
                    world_x,
                    world_z,
                    self.config.world_seed,
                );
                let biome_def = biome::get_biome(debug.biome_id);
                let mut responses = Vec::new();
                self.push_system_message(
                    &mut responses,
                    format!(
                        "Biome: {} (id={}) | temp={:.3} rain={:.3} | surface_y={} | terrain={:.0}..{:.0} | chunk=({}, {})",
                        biome::biome_name(debug.biome_id),
                        debug.biome_id,
                        debug.temperature,
                        debug.rainfall,
                        debug.surface_y,
                        biome_def.min_elevation,
                        biome_def.max_elevation,
                        world_x.div_euclid(16),
                        world_z.div_euclid(16),
                    ),
                );
                responses
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn handle_modal_form_response(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
        let Ok(response) = ModalFormResponse::decode(reader) else {
            warn!("[{}] Failed to decode ModalFormResponse", self.addr);
            return Vec::new();
        };

        let Some(form) = self.pending_forms.remove(&response.form_id) else {
            debug!(
                "[{}] Ignoring response for unknown form_id={}",
                self.addr, response.form_id
            );
            return Vec::new();
        };

        if let Some(reason) = response.cancel_reason {
            debug!(
                "[{}] Form {} closed with cancel_reason={}",
                self.addr, response.form_id, reason
            );
            return Vec::new();
        }

        match form {
            PendingForm::HubMenu => {
                let Some(raw) = response.response_data else {
                    return Vec::new();
                };

                let button_index = serde_json::from_str::<u32>(&raw)
                    .ok()
                    .or_else(|| raw.trim().parse::<u32>().ok());

                if let Some(button_index) = button_index {
                    self.handle_hub_menu_selection(button_index)
                } else {
                    warn!("[{}] Invalid hub menu response payload: {}", self.addr, raw);
                    Vec::new()
                }
            }
        }
    }

    pub fn open_hub_menu_packets(&mut self) -> Vec<Vec<u8>> {
        self.open_hub_menu()
    }
}
