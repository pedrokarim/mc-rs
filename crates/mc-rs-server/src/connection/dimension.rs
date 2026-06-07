//! Changement de dimension côté connexion (overworld ↔ nether ↔ end).
//!
//! Séquence wire reproduite de dragonfly `session.go` (valable depuis 1.19.50, où
//! c'est le **serveur** qui envoie l'ack de fin) :
//!   ChangeDimension → NetworkChunkPublisherUpdate → (chunks) → StopSound(silent)
//!   → PlayStatus(PlayerSpawn) → PlayerAction(DIMENSION_CHANGE_ACK).
//!
//! Le cœur du design (DIM-C) : `self.chunk_cache` est réassigné vers le cache de la
//! dimension cible (`self.worlds.arc(target)`), si bien que tout le code chunk
//! per-connexion (order_chunks / send_chunk_batch / get_block …) opère ensuite
//! automatiquement sur le bon monde.

use mc_rs_proto::packets::chunks::NetworkChunkPublisherUpdate;
use mc_rs_proto::packets::dimension::{ChangeDimension, PlayerAction};
use mc_rs_proto::packets::login::{PlayStatus, PlayStatusType};
use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::world::StopSound;
use tracing::info;

use super::Connection;

impl Connection {
    /// Transfère le joueur vers `target_dim` à `position` (position des yeux).
    /// No-op si déjà dans la dimension cible. Renvoie les batches à envoyer.
    ///
    /// `safe_scan` : si vrai, on cherche un Y sûr à (x,z) dans la dimension cible
    /// (commande `/dimension`). Les portails passent `false` car ils fournissent
    /// déjà une position d'arrivée exacte (à l'intérieur du portail destination).
    pub fn change_dimension(
        &mut self,
        target_dim: i32,
        position: [f32; 3],
        safe_scan: bool,
    ) -> Vec<Vec<u8>> {
        if target_dim == self.current_dimension {
            return Vec::new();
        }
        let mut responses = Vec::new();

        // 1. Bascule le monde courant AVANT de calculer la position d'arrivée.
        self.current_dimension = target_dim;
        self.chunk_cache = self.worlds.arc(target_dim);
        let position = if safe_scan {
            self.safe_arrival_position(target_dim, position)
        } else {
            position
        };
        self.position = position;
        self.sent_chunks.clear();
        self.chunk_load_queue.clear();
        self.visible_entities.clear();

        let px = position[0] as i32;
        let py = position[1] as i32;
        let pz = position[2] as i32;
        self.last_chunk_x = px >> 4;
        self.last_chunk_z = pz >> 4;

        // 2. ChangeDimension — déclenche le loading screen côté client.
        let change = ChangeDimension {
            dimension: target_dim,
            position,
            respawn: false,
            loading_screen_id: None,
        };
        responses
            .push(self.encode_compressed_packet(packet_id::CHANGE_DIMENSION, &change.encode()));

        // 3. NetworkChunkPublisherUpdate centré sur la nouvelle position.
        let publisher = NetworkChunkPublisherUpdate {
            position: [px, py, pz],
            radius: (self.view_distance * 16) as u32,
        };
        responses.push(self.encode_compressed_packet(
            packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
            &publisher.encode(),
        ));

        // 4. Premier batch de chunks ; le reste streame via send_queued_chunks.
        self.order_chunks();
        self.chunk_order_countdown = u32::MAX;
        responses.extend(self.send_chunk_batch());

        // 5. StopSound(silent) — coupe l'audio résiduel de l'ancienne dimension.
        let stop = StopSound {
            sound_name: String::new(),
            stop_all: true,
            stop_legacy_music: false,
        };
        responses.push(self.encode_compressed_packet(packet_id::STOP_SOUND, &stop.encode()));

        // 6. PlayStatus(PlayerSpawn) — confirme le spawn dans la nouvelle dimension.
        let spawn_status = PlayStatus {
            status: PlayStatusType::PlayerSpawn,
        };
        responses
            .push(self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode()));

        // 7. Ack serveur-pilotée (depuis 1.19.50 le serveur s'auto-ack).
        let ack = PlayerAction::dimension_change_ack(self.entity_runtime_id);
        responses.push(self.encode_compressed_packet(packet_id::PLAYER_ACTION, &ack.encode()));

        info!(
            "[{}] dimension change → {} at {:?}",
            self.addr, target_dim, position
        );
        responses
    }

    /// Trouve une position d'arrivée sûre à (x,z) dans `dim` : descend depuis le
    /// haut jusqu'à 2 blocs d'air superposés au-dessus d'un bloc solide (ni air, ni
    /// lave). Renvoie la position des yeux (pieds + 1.62). Retombe sur `hint` si rien.
    fn safe_arrival_position(&mut self, dim: i32, hint: [f32; 3]) -> [f32; 3] {
        use crate::world::block_registry::BLOCKS;

        let (min_y, max_y) = crate::dimensions::DimensionId::from_id(dim)
            .map(|d| (d.min_y(), d.max_y()))
            .unwrap_or((-64, 320));
        let fx = hint[0].floor() as i32;
        let fz = hint[2].floor() as i32;

        let feet = {
            let mut cache = self.chunk_cache.lock().unwrap();
            let mut found = None;
            let mut y = max_y - 2;
            while y > min_y {
                let here = cache.get_block(fx, y, fz);
                let above = cache.get_block(fx, y + 1, fz);
                let below = cache.get_block(fx, y - 1, fz);
                if here == BLOCKS.air
                    && above == BLOCKS.air
                    && below != BLOCKS.air
                    && below != BLOCKS.lava
                    && below != BLOCKS.water
                {
                    found = Some(y);
                    break;
                }
                y -= 1;
            }
            found
        };

        match feet {
            Some(y) => [hint[0], y as f32 + 1.62, hint[2]],
            None => hint,
        }
    }

    /// Vérifie chaque tick si le joueur se tient dans un bloc de portail Nether ;
    /// avance le timer et déclenche le transfert OW↔Nether le cas échéant.
    /// À appeler depuis le handler de mouvement.
    pub(super) fn check_nether_portal(&mut self) -> Vec<Vec<u8>> {
        use crate::world::nether_portal_build::is_portal_block;

        let fx = self.position[0].floor() as i32;
        let fz = self.position[2].floor() as i32;
        let feet_y = (self.position[1] - 1.62).floor() as i32;
        let eye_y = self.position[1].floor() as i32;

        let in_portal = {
            let mut cache = self.chunk_cache.lock().unwrap();
            is_portal_block(&mut cache, fx, feet_y, fz)
                || is_portal_block(&mut cache, fx, eye_y, fz)
        };

        if !in_portal {
            self.portal_state.leave_portal();
            return Vec::new();
        }

        // Synchronise la dimension de référence avant de ticker.
        if let Some(d) = crate::dimensions::DimensionId::from_id(self.current_dimension) {
            self.portal_state.last_dimension = d;
        }
        match self.portal_state.tick_in_portal() {
            Some(target) => self.nether_portal_teleport(target.as_id()),
            None => Vec::new(),
        }
    }

    /// Téléporte le joueur via un portail Nether : applique le ratio 1:8, crée un
    /// portail à la destination, et lance la séquence de changement de dimension
    /// avec une position d'arrivée exacte (à l'intérieur du portail destination).
    fn nether_portal_teleport(&mut self, target_dim: i32) -> Vec<Vec<u8>> {
        use crate::nether_portal::translate_position;
        use crate::world::nether_portal_build::{create_portal, find_ground};

        let from = crate::dimensions::DimensionId::from_id(self.current_dimension)
            .unwrap_or(crate::dimensions::DimensionId::Overworld);
        let to = crate::dimensions::DimensionId::from_id(target_dim)
            .unwrap_or(crate::dimensions::DimensionId::Overworld);
        let translated = translate_position(from, to, self.position);
        let fx = translated[0].floor() as i32;
        let fz = translated[2].floor() as i32;

        // Construit le portail destination dans le monde cible (les chunks envoyés
        // par change_dimension le contiendront déjà).
        let arrival_feet = {
            let target_arc = self.worlds.arc(target_dim);
            let mut cache = target_arc.lock().unwrap();
            let floor_y = find_ground(&mut cache, target_dim, fx, fz);
            create_portal(&mut cache, fx, floor_y, fz)
        };

        let eye = [arrival_feet[0], arrival_feet[1] + 1.62, arrival_feet[2]];
        self.change_dimension(target_dim, eye, false)
    }
}
