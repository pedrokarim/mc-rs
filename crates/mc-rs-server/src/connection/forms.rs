use mc_rs_proto::io::ProtoReader;
use mc_rs_proto::packets::forms::{ModalFormRequest, ModalFormResponse};
use mc_rs_proto::packets::packet_id;
use tracing::debug;

use super::Connection;

/// Forme active envoyée au client et en attente d'une `ModalFormResponse`.
#[derive(Debug, Clone)]
pub struct PendingForm {
    pub form_id: u32,
    pub kind: PendingFormKind,
}

#[derive(Debug, Clone, Copy)]
pub enum PendingFormKind {
    /// Menu hub style Hive — 6 boutons (spawn, gamemode, time, biome).
    HubMenu,
}

impl Connection {
    /// Construit le batch `ModalFormRequest` pour le menu hub.
    /// Tracke le `form_id` dans `pending_form` pour router la réponse.
    pub fn build_hub_form_batch(&mut self) -> Vec<u8> {
        let form_id = self.next_form_id.max(1);
        self.next_form_id = form_id.wrapping_add(1).max(1);
        self.pending_form = Some(PendingForm {
            form_id,
            kind: PendingFormKind::HubMenu,
        });

        // Bedrock ActionForm — JSON inline. Codes couleur § interprétés.
        let json = r#"{"type":"form","title":"§l§6HIVE§r §eHUB","content":"§7Choisis une action :","buttons":[{"text":"§a▶ Téléporter au spawn"},{"text":"§b▶ Mode Créatif"},{"text":"§e▶ Mode Survie"},{"text":"§d▶ Régler sur Jour"},{"text":"§9▶ Régler sur Nuit"},{"text":"§7▶ Infos biome"}]}"#;

        let req = ModalFormRequest {
            form_id,
            form_data: json.to_string(),
        };
        self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &req.encode())
    }

    /// Décode `ModalFormResponse` et pousse la commande pending qui correspond
    /// au bouton choisi. La commande est exécutée par la main loop via le
    /// pipeline `dispatch_command_line` standard.
    pub(super) fn handle_modal_form_response(
        &mut self,
        reader: &mut ProtoReader,
    ) -> Vec<Vec<u8>> {
        let Ok(resp) = ModalFormResponse::decode(reader) else {
            return Vec::new();
        };

        let Some(pending) = self.pending_form.take() else {
            debug!("[{}] ModalFormResponse with no pending form", self.addr);
            return Vec::new();
        };

        if pending.form_id != resp.form_id {
            debug!(
                "[{}] ModalFormResponse mismatch: pending={} got={}",
                self.addr, pending.form_id, resp.form_id
            );
            return Vec::new();
        }

        if resp.cancel_reason.is_some() {
            return Vec::new();
        }
        let Some(data) = resp.response_data else {
            return Vec::new();
        };
        let Ok(index): Result<usize, _> = data.trim().parse() else {
            return Vec::new();
        };

        match pending.kind {
            PendingFormKind::HubMenu => {
                let cmd = match index {
                    0 => Some(format!(
                        "tp {} {} {}",
                        self.spawn_position[0],
                        self.spawn_position[1],
                        self.spawn_position[2]
                    )),
                    1 => Some("gamemode creative".into()),
                    2 => Some("gamemode survival".into()),
                    3 => Some("time set day".into()),
                    4 => Some("time set night".into()),
                    5 => Some("biome".into()),
                    _ => None,
                };
                if let Some(c) = cmd {
                    self.pending_commands.push(c);
                }
            }
        }
        Vec::new()
    }
}
