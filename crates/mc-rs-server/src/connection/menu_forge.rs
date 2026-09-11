//! Menus Menu Forge d’un joueur : ouverture, réponse aux clics et effets
//! (formulaire suivant, commandes, sons). Le moteur est `crate::menu_forge` ;
//! ce module ne fait que le relier à la connexion.

use mc_rs_proto::packets::forms::ModalFormResponse;
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::world::PlaySound;
use tracing::{debug, info, warn};

use super::forms::PendingFormKind;
use super::Connection;
use crate::menu_forge::{self, CommandRunner, Effect, NoLists, Session, Viewer};

/// `ModalFormCancelReason::UserBusy` : formulaire refusé, le joueur avait un autre écran ouvert.
const USER_BUSY: u8 = 1;

impl Connection {
    /// Joueur vu par le moteur : `{viewer.name}`, `{viewer.uuid}`, drapeaux
    /// `viewer.bedrock` et `viewer.op`, et `{mcrs.spawn}` (réapparition, `x y z`).
    pub fn menu_forge_viewer(&self) -> Viewer {
        let name = self.display_name.clone().unwrap_or_default();
        let uuid = self.uuid.map(|uuid| uuid.to_string()).unwrap_or_default();
        let [x, y, z] = self.spawn_position;
        let viewer = Viewer::new(&name, &uuid).with_variable("mcrs.spawn", format!("{x} {y} {z}"));
        if self.is_op {
            viewer.with_flag("viewer.op")
        } else {
            viewer
        }
    }

    /// Ouvre le menu Menu Forge `menu_id` (nouvelle session) : renvoie le lot à envoyer.
    pub fn open_menu_forge(&mut self, menu_id: &str) -> Result<Vec<u8>, String> {
        let mut session =
            Session::open(menu_forge::shared().catalog(), menu_id, &Default::default())?;
        let form = session.compose(&self.menu_forge_viewer(), &NoLists);
        for warning in session.take_warnings() {
            warn!("[menu-forge] [{}] {warning}", self.addr);
        }
        info!("[menu-forge] [{}] ouverture de « {menu_id} »", self.addr);
        self.menu_forge_session = Some(session);
        Ok(self.encode_form(PendingFormKind::MenuForge, &form.to_json()))
    }

    /// Réponse à un formulaire Menu Forge : exécute le clic, renvoie les lots à envoyer.
    pub(super) fn handle_menu_forge_response(
        &mut self,
        response: &ModalFormResponse,
    ) -> Vec<Vec<u8>> {
        if let Some(reason) = response.cancel_reason {
            if reason == USER_BUSY {
                warn!(
                    "[menu-forge] [{}] formulaire refusé par le client (autre écran ouvert)",
                    self.addr
                );
            } else {
                debug!("[menu-forge] [{}] menu fermé par le joueur", self.addr);
            }
            self.menu_forge_session = None;
            return Vec::new();
        }
        let index = response
            .response_data
            .as_deref()
            .and_then(|data| data.trim().parse::<usize>().ok());
        let (Some(index), Some(mut session)) = (index, self.menu_forge_session.take()) else {
            warn!(
                "[menu-forge] [{}] réponse sans index lisible : {:?}",
                self.addr, response.response_data
            );
            return Vec::new();
        };
        let effects = session.click(index, &self.menu_forge_viewer(), &NoLists);
        self.apply_menu_forge_effects(session, effects)
    }

    fn apply_menu_forge_effects(&mut self, session: Session, effects: Vec<Effect>) -> Vec<Vec<u8>> {
        let mut batches = Vec::new();
        let mut session = Some(session);
        for effect in effects {
            match effect {
                Effect::Show(form) => {
                    batches.push(self.encode_form(PendingFormKind::MenuForge, &form.to_json()));
                }
                Effect::Command {
                    line,
                    runner: CommandRunner::Player,
                } => self.pending_commands.push(line),
                Effect::Command {
                    line,
                    runner: CommandRunner::Console,
                } => self.pending_console_commands.push(line),
                Effect::Sound {
                    name,
                    volume,
                    pitch,
                } => {
                    let payload = PlaySound {
                        sound_name: name,
                        position: self.position,
                        volume,
                        pitch,
                    }
                    .encode();
                    batches.push(self.encode_compressed_packet(packet_id::PLAY_SOUND, &payload));
                }
                Effect::Custom { id, args } => info!(
                    "[menu-forge] [{}] action custom « {id} » ({args}) : aucun gestionnaire sur mc-rs",
                    self.addr
                ),
                Effect::Warning(message) => warn!("[menu-forge] [{}] {message}", self.addr),
                Effect::Closed => session = None,
            }
        }
        self.menu_forge_session = session;
        batches
    }
}
