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
    /// Menu racine du `/menu` — actions joueur + sous-menu "UI Showcase".
    HubMenu,
    /// Sous-menu "UI Showcase" — un bouton par layout custom du pack.
    UiShowcase,
    /// Démos individuelles pour chaque layout du pack `mcrs_ui`.
    DemoGrid,
    DemoLeftButton,
    DemoBottomButton,
    DemoImageGrid,
    DemoSquareImage,
    DemoMotd,
    DemoStore,
    DemoWrapped,
}

/// Préfixes injectés au début du `title` pour activer le layout custom côté
/// pack (`resource_packs/mcrs_ui/ui/server_form.json` matche ces sous-chaînes).
const FLAG_GRID: &str = "\u{00A7}m\u{00A7}a";
const FLAG_LEFT_BUTTON: &str = "\u{00A7}m\u{00A7}b";
const FLAG_BOTTOM_BUTTON: &str = "\u{00A7}m\u{00A7}c";
const FLAG_IMAGE_GRID: &str = "\u{00A7}m\u{00A7}d";
const FLAG_SQUARE_IMAGE: &str = "\u{00A7}m\u{00A7}e";
const FLAG_MOTD: &str = "\u{00A7}m\u{00A7}f";
const FLAG_STORE: &str = "\u{00A7}m\u{00A7}0";
const FLAG_WRAPPED: &str = "\u{00A7}m\u{00A7}1";

impl Connection {
    fn allocate_form_id(&mut self) -> u32 {
        let form_id = self.next_form_id.max(1);
        self.next_form_id = form_id.wrapping_add(1).max(1);
        form_id
    }

    fn encode_form(&mut self, kind: PendingFormKind, json: &str) -> Vec<u8> {
        let form_id = self.allocate_form_id();
        self.pending_form = Some(PendingForm { form_id, kind });
        let req = ModalFormRequest {
            form_id,
            form_data: json.to_string(),
        };
        self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &req.encode())
    }

    /// Menu racine — affiché par `/menu`. Layout `grid` (flag §m§a).
    pub fn build_hub_form_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_GRID} §l§6mc-rs§r §eHUB");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Choisis une action :","buttons":[{{"text":"§a▶ Téléporter au spawn"}},{{"text":"§b▶ Mode Créatif"}},{{"text":"§e▶ Mode Survie"}},{{"text":"§d▶ Régler sur Jour"}},{{"text":"§9▶ Régler sur Nuit"}},{{"text":"§7▶ Infos biome"}},{{"text":"§6▶ §lUI Showcase"}}]}}"#
        );
        self.encode_form(PendingFormKind::HubMenu, &json)
    }

    /// Sous-menu UI showcase — un bouton par layout custom.
    fn build_ui_showcase_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_GRID} §l§6UI§r §eShowcase");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Sélectionne le layout à afficher :","buttons":[{{"text":"§e▶ Grid §7(button_grid)"}},{{"text":"§e▶ Left buttons §7(left_button)"}},{{"text":"§e▶ Bottom buttons §7(bottom_button)"}},{{"text":"§e▶ Image grid §7(image_grid)"}},{{"text":"§e▶ Square image §7(square_image)"}},{{"text":"§e▶ MOTD §7(motd)"}},{{"text":"§e▶ Store §7(store)"}},{{"text":"§e▶ Wrapped §7(wrapped)"}},{{"text":"§c↩ Retour"}}]}}"#
        );
        self.encode_form(PendingFormKind::UiShowcase, &json)
    }

    fn build_demo_grid_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_GRID} §6Grid layout");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Layout: button_grid_panel — grille verticale de boutons rectangulaires.","buttons":[{{"text":"§eAction A"}},{{"text":"§eAction B"}},{{"text":"§eAction C"}},{{"text":"§eAction D"}},{{"text":"§eAction E"}},{{"text":"§eAction F"}},{{"text":"§c↩ Retour"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoGrid, &json)
    }

    fn build_demo_left_button_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_LEFT_BUTTON} §6Left buttons");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Liste de boutons à gauche, description à droite. Survole un bouton pour afficher son texte.","buttons":[{{"text":"§ePartie classée"}},{{"text":"§ePartie rapide"}},{{"text":"§eTutoriel"}},{{"text":"§eParamètres"}},{{"text":"§c↩ Retour"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoLeftButton, &json)
    }

    fn build_demo_bottom_button_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_BOTTOM_BUTTON} §6Bottom buttons");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Bannière en haut, liste de boutons en bas. Mise en scène d'un mode de jeu.","buttons":[{{"text":"§m§a Mode Bedwars"}},{{"text":"§eRejoindre solo"}},{{"text":"§eRejoindre duo"}},{{"text":"§eRejoindre squad"}},{{"text":"§c↩ Retour"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoBottomButton, &json)
    }

    fn build_demo_image_grid_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_IMAGE_GRID} §6Image grid");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Grille d'images avec titre superposé. Idéal pour un sélecteur de map.","buttons":[{{"text":"Forêt\t§aDifficulté facile","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Désert\t§eDifficulté moyenne","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Montagne\t§cDifficulté difficile","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Marais\t§eDifficulté moyenne","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§c↩ Retour"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoImageGrid, &json)
    }

    fn build_demo_square_image_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_SQUARE_IMAGE} §6Square image");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Affichage central d'une image carrée avec description en bas.","buttons":[{{"text":"§m§a image","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoSquareImage, &json)
    }

    fn build_demo_motd_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_MOTD} §6MOTD");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§7Bienvenue sur mc-rs.\n§7Le serveur Minecraft Bedrock le plus rapide écrit en Rust.\n\n§eAppuie sur un bouton pour continuer.","buttons":[{{"text":"§m§a Banner","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§eContinuer"}},{{"text":"§cQuitter"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoMotd, &json)
    }

    fn build_demo_store_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_STORE} §6Store");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"§m§aPopulaire","buttons":[{{"text":"§m§a Populaire"}},{{"text":"§m§a Nouveautés"}},{{"text":"§m§a Promotions"}},{{"text":"Épée légendaire\t§a1500 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Pioche en diamant\t§a800 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Skin pirate\t§a2000 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Cape dragon\t§a3500 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoStore, &json)
    }

    fn build_demo_wrapped_batch(&mut self) -> Vec<u8> {
        let title = format!("{FLAG_WRAPPED} §6Wrapped");
        let json = format!(
            r#"{{"type":"form","title":"{title}","content":"https://mcrs.io/recap/2026","buttons":[{{"text":"§m§a Image 1","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§m§a Image 2","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§m§a Image 3","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§eContinuer"}},{{"text":"§cFermer"}}]}}"#
        );
        self.encode_form(PendingFormKind::DemoWrapped, &json)
    }

    /// Liste des panels exposés en autocomplete `/menu <panel>`.
    pub const DEMO_PANEL_NAMES: &'static [&'static str] = &[
        "hub",
        "showcase",
        "grid",
        "left_button",
        "bottom_button",
        "image_grid",
        "square_image",
        "motd",
        "store",
        "wrapped",
    ];

    /// Construit le batch ouvrant la démo nommée `panel` (utilisé par
    /// `/menu <panel>`). Retourne `None` si `panel` n'est pas reconnu.
    pub fn build_demo_panel_batch(&mut self, panel: &str) -> Option<Vec<u8>> {
        Some(match panel {
            "hub" => self.build_hub_form_batch(),
            "showcase" => self.build_ui_showcase_batch(),
            "grid" => self.build_demo_grid_batch(),
            "left_button" => self.build_demo_left_button_batch(),
            "bottom_button" => self.build_demo_bottom_button_batch(),
            "image_grid" => self.build_demo_image_grid_batch(),
            "square_image" => self.build_demo_square_image_batch(),
            "motd" => self.build_demo_motd_batch(),
            "store" => self.build_demo_store_batch(),
            "wrapped" => self.build_demo_wrapped_batch(),
            _ => return None,
        })
    }

    /// Décode `ModalFormResponse` et déclenche soit une commande (`pending_commands`),
    /// soit un sous-menu (`pending_form_batches`).
    pub(super) fn handle_modal_form_response(&mut self, reader: &mut ProtoReader) -> Vec<Vec<u8>> {
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
            PendingFormKind::HubMenu => self.handle_hub_menu(index),
            PendingFormKind::UiShowcase => self.handle_ui_showcase(index),
            PendingFormKind::DemoBottomButton => self.handle_demo_bottom_button(index),
            PendingFormKind::DemoGrid
            | PendingFormKind::DemoLeftButton
            | PendingFormKind::DemoImageGrid
            | PendingFormKind::DemoSquareImage
            | PendingFormKind::DemoMotd
            | PendingFormKind::DemoStore
            | PendingFormKind::DemoWrapped => self.handle_demo_back_to_showcase(),
        }
    }

    fn handle_hub_menu(&mut self, index: usize) -> Vec<Vec<u8>> {
        let cmd = match index {
            0 => Some(format!(
                "tp {} {} {}",
                self.spawn_position[0], self.spawn_position[1], self.spawn_position[2]
            )),
            1 => Some("gamemode creative".into()),
            2 => Some("gamemode survival".into()),
            3 => Some("time set day".into()),
            4 => Some("time set night".into()),
            5 => Some("biome".into()),
            6 => {
                let batch = self.build_ui_showcase_batch();
                return vec![batch];
            }
            _ => None,
        };
        if let Some(c) = cmd {
            self.pending_commands.push(c);
        }
        Vec::new()
    }

    fn handle_ui_showcase(&mut self, index: usize) -> Vec<Vec<u8>> {
        let batch = match index {
            0 => self.build_demo_grid_batch(),
            1 => self.build_demo_left_button_batch(),
            2 => self.build_demo_bottom_button_batch(),
            3 => self.build_demo_image_grid_batch(),
            4 => self.build_demo_square_image_batch(),
            5 => self.build_demo_motd_batch(),
            6 => self.build_demo_store_batch(),
            7 => self.build_demo_wrapped_batch(),
            8 => self.build_hub_form_batch(),
            _ => return Vec::new(),
        };
        vec![batch]
    }

    fn handle_demo_bottom_button(&mut self, index: usize) -> Vec<Vec<u8>> {
        // index 0 = bannière (non cliquable côté UI mais le client peut renvoyer un index),
        // les boutons utiles commencent à 1. Index dernier = retour.
        if index == 4 {
            return vec![self.build_ui_showcase_batch()];
        }
        Vec::new()
    }

    fn handle_demo_back_to_showcase(&mut self) -> Vec<Vec<u8>> {
        vec![self.build_ui_showcase_batch()]
    }
}
