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
    /// Menu Menu Forge (`/menu`, `/mf open`), voir `connection/menu_forge.rs`.
    MenuForge,
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

/// Effet d’un clic sur un ancien formulaire de `/menu` (repli `/menu legacy`,
/// et référence des tests de parité avec les formulaires Menu Forge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LegacyClick {
    /// Commande lancée par le joueur ; le formulaire reste fermé.
    Command(String),
    /// Ouverture d’un autre panneau de `/menu`.
    Open(&'static str),
    /// Rien, le formulaire reste fermé.
    Nothing,
}

/// Type en attente et JSON de l’ancien formulaire `panel` de `/menu`, tels
/// qu’écrits à la main avant Menu Forge (préfixe de titre : layout du pack `mcrs_ui`).
pub(crate) fn legacy_form(panel: &str) -> Option<(PendingFormKind, String)> {
    Some(match panel {
        "hub" => {
            let title = format!("{FLAG_GRID} §l§6mc-rs§r §eHUB");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Choisis une action :","buttons":[{{"text":"§a▶ Téléporter au spawn"}},{{"text":"§b▶ Mode Créatif"}},{{"text":"§e▶ Mode Survie"}},{{"text":"§d▶ Régler sur Jour"}},{{"text":"§9▶ Régler sur Nuit"}},{{"text":"§7▶ Infos biome"}},{{"text":"§6▶ §lUI Showcase"}}]}}"#
            );
            (PendingFormKind::HubMenu, json)
        }
        "showcase" => {
            let title = format!("{FLAG_GRID} §l§6UI§r §eShowcase");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Sélectionne le layout à afficher :","buttons":[{{"text":"§e▶ Grid §7(button_grid)"}},{{"text":"§e▶ Left buttons §7(left_button)"}},{{"text":"§e▶ Bottom buttons §7(bottom_button)"}},{{"text":"§e▶ Image grid §7(image_grid)"}},{{"text":"§e▶ Square image §7(square_image)"}},{{"text":"§e▶ MOTD §7(motd)"}},{{"text":"§e▶ Store §7(store)"}},{{"text":"§e▶ Wrapped §7(wrapped)"}},{{"text":"§c↩ Retour"}}]}}"#
            );
            (PendingFormKind::UiShowcase, json)
        }
        "grid" => {
            let title = format!("{FLAG_GRID} §6Grid layout");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Layout: button_grid_panel — grille verticale de boutons rectangulaires.","buttons":[{{"text":"§eAction A"}},{{"text":"§eAction B"}},{{"text":"§eAction C"}},{{"text":"§eAction D"}},{{"text":"§eAction E"}},{{"text":"§eAction F"}},{{"text":"§c↩ Retour"}}]}}"#
            );
            (PendingFormKind::DemoGrid, json)
        }
        "left_button" => {
            let title = format!("{FLAG_LEFT_BUTTON} §6Left buttons");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Liste de boutons à gauche, description à droite. Survole un bouton pour afficher son texte.","buttons":[{{"text":"§ePartie classée"}},{{"text":"§ePartie rapide"}},{{"text":"§eTutoriel"}},{{"text":"§eParamètres"}},{{"text":"§c↩ Retour"}}]}}"#
            );
            (PendingFormKind::DemoLeftButton, json)
        }
        "bottom_button" => {
            let title = format!("{FLAG_BOTTOM_BUTTON} §6Bottom buttons");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Bannière en haut, liste de boutons en bas. Mise en scène d'un mode de jeu.","buttons":[{{"text":"§m§a Mode Bedwars"}},{{"text":"§eRejoindre solo"}},{{"text":"§eRejoindre duo"}},{{"text":"§eRejoindre squad"}},{{"text":"§c↩ Retour"}}]}}"#
            );
            (PendingFormKind::DemoBottomButton, json)
        }
        "image_grid" => {
            let title = format!("{FLAG_IMAGE_GRID} §6Image grid");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Grille d'images avec titre superposé. Idéal pour un sélecteur de map.","buttons":[{{"text":"Forêt\t§aDifficulté facile","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Désert\t§eDifficulté moyenne","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Montagne\t§cDifficulté difficile","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Marais\t§eDifficulté moyenne","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§c↩ Retour"}}]}}"#
            );
            (PendingFormKind::DemoImageGrid, json)
        }
        "square_image" => {
            let title = format!("{FLAG_SQUARE_IMAGE} §6Square image");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Affichage central d'une image carrée avec description en bas.","buttons":[{{"text":"§m§a image","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}}]}}"#
            );
            (PendingFormKind::DemoSquareImage, json)
        }
        "motd" => {
            let title = format!("{FLAG_MOTD} §6MOTD");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§7Bienvenue sur mc-rs.\n§7Le serveur Minecraft Bedrock le plus rapide écrit en Rust.\n\n§eAppuie sur un bouton pour continuer.","buttons":[{{"text":"§m§a Banner","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§eContinuer"}},{{"text":"§cQuitter"}}]}}"#
            );
            (PendingFormKind::DemoMotd, json)
        }
        "store" => {
            let title = format!("{FLAG_STORE} §6Store");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"§m§aPopulaire","buttons":[{{"text":"§m§a Populaire"}},{{"text":"§m§a Nouveautés"}},{{"text":"§m§a Promotions"}},{{"text":"Épée légendaire\t§a1500 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Pioche en diamant\t§a800 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Skin pirate\t§a2000 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"Cape dragon\t§a3500 coins","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}}]}}"#
            );
            (PendingFormKind::DemoStore, json)
        }
        "wrapped" => {
            let title = format!("{FLAG_WRAPPED} §6Wrapped");
            let json = format!(
                r#"{{"type":"form","title":"{title}","content":"https://mcrs.io/recap/2026","buttons":[{{"text":"§m§a Image 1","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§m§a Image 2","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§m§a Image 3","image":{{"type":"path","data":"textures/ui/mcrs/panels/loading_grid"}}}},{{"text":"§eContinuer"}},{{"text":"§cFermer"}}]}}"#
            );
            (PendingFormKind::DemoWrapped, json)
        }
        _ => return None,
    })
}

/// Effet d’un clic sur l’entrée `index` d’un ancien formulaire (`spawn` : point de réapparition).
pub(crate) fn legacy_click(kind: PendingFormKind, index: usize, spawn: [f32; 3]) -> LegacyClick {
    let command = |line: &str| LegacyClick::Command(line.to_string());
    match kind {
        PendingFormKind::HubMenu => match index {
            0 => LegacyClick::Command(format!("tp {} {} {}", spawn[0], spawn[1], spawn[2])),
            1 => command("gamemode creative"),
            2 => command("gamemode survival"),
            3 => command("time set day"),
            4 => command("time set night"),
            5 => command("biome"),
            6 => LegacyClick::Open("showcase"),
            _ => LegacyClick::Nothing,
        },
        PendingFormKind::UiShowcase => match index {
            0 => LegacyClick::Open("grid"),
            1 => LegacyClick::Open("left_button"),
            2 => LegacyClick::Open("bottom_button"),
            3 => LegacyClick::Open("image_grid"),
            4 => LegacyClick::Open("square_image"),
            5 => LegacyClick::Open("motd"),
            6 => LegacyClick::Open("store"),
            7 => LegacyClick::Open("wrapped"),
            8 => LegacyClick::Open("hub"),
            _ => LegacyClick::Nothing,
        },
        // Bannière (0) et « Rejoindre » (1 à 3) : rien ; le dernier bouton revient au showcase.
        PendingFormKind::DemoBottomButton if index == 4 => LegacyClick::Open("showcase"),
        PendingFormKind::DemoBottomButton => LegacyClick::Nothing,
        PendingFormKind::DemoGrid
        | PendingFormKind::DemoLeftButton
        | PendingFormKind::DemoImageGrid
        | PendingFormKind::DemoSquareImage
        | PendingFormKind::DemoMotd
        | PendingFormKind::DemoStore
        | PendingFormKind::DemoWrapped => LegacyClick::Open("showcase"),
        PendingFormKind::MenuForge => LegacyClick::Nothing,
    }
}

impl Connection {
    fn allocate_form_id(&mut self) -> u32 {
        let form_id = self.next_form_id.max(1);
        self.next_form_id = form_id.wrapping_add(1).max(1);
        form_id
    }

    pub(super) fn encode_form(&mut self, kind: PendingFormKind, json: &str) -> Vec<u8> {
        let form_id = self.allocate_form_id();
        self.pending_form = Some(PendingForm { form_id, kind });
        let req = ModalFormRequest {
            form_id,
            form_data: json.to_string(),
        };
        self.encode_compressed_packet(packet_id::MODAL_FORM_REQUEST, &req.encode())
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
        let (kind, json) = legacy_form(panel)?;
        Some(self.encode_form(kind, &json))
    }

    /// Menu racine de l’ancien `/menu` (layout `grid`).
    pub fn build_hub_form_batch(&mut self) -> Vec<u8> {
        self.build_demo_panel_batch("hub").unwrap_or_default()
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

        // Menu Forge traite aussi les annulations (fin de la session du joueur).
        if let PendingFormKind::MenuForge = pending.kind {
            return self.handle_menu_forge_response(&resp);
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

        match legacy_click(pending.kind, index, self.spawn_position) {
            LegacyClick::Command(command) => {
                self.pending_commands.push(command);
                Vec::new()
            }
            LegacyClick::Open(panel) => self.build_demo_panel_batch(panel).into_iter().collect(),
            LegacyClick::Nothing => Vec::new(),
        }
    }
}
