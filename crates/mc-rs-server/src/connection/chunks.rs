use tracing::debug;

use mc_rs_proto::batch::CompressionAlgorithm;
use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::packet_id;

use super::{encode_shared_batch, Connection, ConnectionState, CHUNKS_PER_TICK};

impl Connection {
    /// Reorder the chunk load queue: spiral from player position, unload distant chunks.
    /// Called when the player changes chunk or periodically.
    pub fn order_chunks(&mut self) {
        self.chunk_load_queue.clear();
        let cx = self.last_chunk_x;
        let cz = self.last_chunk_z;
        let r = self.view_distance;
        let r_sq = r * r;

        // Collect all chunks in circular view distance, sorted by distance
        let mut candidates: Vec<(i32, i32, i32)> = Vec::new();
        for dx in -r..=r {
            for dz in -r..=r {
                let dist_sq = dx * dx + dz * dz;
                if dist_sq <= r_sq {
                    let chunk = (cx + dx, cz + dz);
                    if !self.sent_chunks.contains(&chunk) {
                        candidates.push((cx + dx, cz + dz, dist_sq));
                    }
                }
            }
        }

        // Sort by distance (nearest first = spiral-like)
        candidates.sort_by_key(|&(_, _, d)| d);

        for (x, z, _) in candidates {
            self.chunk_load_queue.push_back((x, z));
        }

        // Unload chunks outside view distance (+2 margin)
        let unload_r_sq = (r + 2) * (r + 2);
        let old: Vec<(i32, i32)> = self
            .sent_chunks
            .iter()
            .filter(|&&(sx, sz)| {
                let dx = sx - cx;
                let dz = sz - cz;
                dx * dx + dz * dz > unload_r_sq
            })
            .copied()
            .collect();
        for chunk in old {
            self.sent_chunks.remove(&chunk);
        }
    }

    /// Send up to CHUNKS_PER_TICK chunks from the queue.
    /// Called from the main tick loop, not from packet handlers.
    /// Returns response packets to send to this player.
    pub fn send_queued_chunks(&mut self) -> Vec<Vec<u8>> {
        // PMMP-style countdown: doChunkRequests() { if(nextChunkOrderRun-- <= 0) { orderChunks(); } }
        if self.chunk_order_countdown != u32::MAX {
            if self.chunk_order_countdown == 0 {
                self.order_chunks();
                self.chunk_order_countdown = u32::MAX; // idle until next trigger

                debug!(
                    "[{}] order_chunks: queue={}, sent_chunks={}, pos=({},{}), view_distance={}",
                    self.addr,
                    self.chunk_load_queue.len(),
                    self.sent_chunks.len(),
                    self.last_chunk_x,
                    self.last_chunk_z,
                    self.view_distance,
                );

                // Send NetworkChunkPublisherUpdate when there are chunks to load/unload
                if !self.chunk_load_queue.is_empty() {
                    let ncpu = NetworkChunkPublisherUpdate {
                        position: [
                            self.position[0] as i32,
                            self.position[1] as i32,
                            self.position[2] as i32,
                        ],
                        radius: (self.view_distance * 16) as u32,
                    };
                    let mut responses = vec![self.encode_compressed_packet(
                        packet_id::NETWORK_CHUNK_PUBLISHER_UPDATE,
                        &ncpu.encode(),
                    )];
                    // Send chunks in same batch
                    responses.extend(self.send_chunk_batch());
                    return responses;
                }
            } else {
                self.chunk_order_countdown -= 1;
            }
        }

        // Still send queued chunks even if no reorder happened
        let mut out = self.send_chunk_batch();

        // PMMP notifyTerrainReady : envoie PlayStatus(PlayerSpawn) quand
        // la queue initiale est vidée. Bloque le bug "chargement du serveur"
        // infini (client recevait PlayerSpawn trop tôt avant que les chunks
        // arrivent).
        if self.player_spawn_pending && self.chunk_load_queue.is_empty() {
            use mc_rs_proto::packets::login::{PlayStatus, PlayStatusType};
            use mc_rs_proto::packets::packet_id;
            let spawn_status = PlayStatus {
                status: PlayStatusType::PlayerSpawn,
            };
            out.push(
                self.encode_compressed_packet(packet_id::PLAY_STATUS, &spawn_status.encode()),
            );
            self.player_spawn_pending = false;
            debug!(
                "[{}] Terrain ready ({} chunks) — envoi PlayStatus(PlayerSpawn)",
                self.addr,
                self.sent_chunks.len()
            );
        }

        out
    }

    /// Send a small batch of chunks from the load queue.
    pub(super) fn send_chunk_batch(&mut self) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();
        let mut sent = 0;
        let queue_before = self.chunk_load_queue.len();

        while sent < CHUNKS_PER_TICK {
            let Some((cx, cz)) = self.chunk_load_queue.pop_front() else {
                break;
            };
            if self.sent_chunks.contains(&(cx, cz)) {
                continue;
            }

            // Fast path : si le client utilise Zlib (le défaut wire), on partage
            // un batch pré-compressé par chunk via cached_zlib_batch. Tous les
            // joueurs qui chargent le même chunk évitent N compressions Zlib —
            // seul `prepare_for_send` (encryption per-player) s'applique.
            // Fallback : autres algos (Snappy/None, rares) → compression per-player.
            let conn_zlib = matches!(self.compression_algo, CompressionAlgorithm::Zlib);

            let raw_batch_opt: Option<Vec<u8>> = if conn_zlib {
                let mut cache = self.chunk_cache.lock().unwrap();
                let col = cache.get_chunk_mut(cx, cz);
                if col.cached_zlib_batch.is_none() {
                    let sub_count = col.sub_chunk_count;
                    let payload = col.get_network_payload().to_vec();
                    let chunk_pkt = LevelChunk {
                        chunk_x: cx,
                        chunk_z: cz,
                        dimension_id: 0,
                        sub_chunk_count: sub_count,
                        cache_enabled: false,
                        payload,
                    };
                    let shared = encode_shared_batch(
                        packet_id::LEVEL_CHUNK,
                        &chunk_pkt.encode(),
                        CompressionAlgorithm::Zlib,
                    );
                    col.cached_zlib_batch = Some(shared);
                }
                Some(col.cached_zlib_batch.as_ref().unwrap().clone())
            } else {
                None
            };

            if let Some(raw_batch) = raw_batch_opt {
                let prepared = self.prepare_for_send(raw_batch);
                responses.push(prepared);
            } else {
                let (sub_count, payload) = {
                    let mut cache = self.chunk_cache.lock().unwrap();
                    let col = cache.get_chunk_mut(cx, cz);
                    (col.sub_chunk_count, col.get_network_payload().to_vec())
                };
                let chunk_pkt = LevelChunk {
                    chunk_x: cx,
                    chunk_z: cz,
                    dimension_id: 0,
                    sub_chunk_count: sub_count,
                    cache_enabled: false,
                    payload,
                };
                responses.push(
                    self.encode_compressed_packet(packet_id::LEVEL_CHUNK, &chunk_pkt.encode()),
                );
            }
            self.sent_chunks.insert((cx, cz));
            sent += 1;
        }

        if sent > 0 {
            debug!(
                "[{}] send_chunk_batch: sent={}, queue_before={}, queue_after={}, sent_chunks_total={}, countdown={}",
                self.addr,
                sent,
                queue_before,
                self.chunk_load_queue.len(),
                self.sent_chunks.len(),
                self.chunk_order_countdown,
            );
        }

        responses
    }

    pub fn should_stream_chunks(&self) -> bool {
        // PMMP : le streaming démarre après que le client envoie
        // `RequestChunkRadius` (handle_request_chunk_radius bascule en
        // SpawnResponse). Streamer en PreSpawn = chunks envoyés avant que
        // le client connaisse son radius / soit prêt → ils sont ignorés
        // ou bloquent le spawn.
        matches!(
            self.state,
            ConnectionState::SpawnResponse | ConnectionState::InGame
        )
    }
}
