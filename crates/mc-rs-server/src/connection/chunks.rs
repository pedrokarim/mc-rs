use tracing::debug;

use mc_rs_proto::packets::chunks::*;
use mc_rs_proto::packets::packet_id;

use super::{Connection, ConnectionState, CHUNKS_PER_TICK};

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
                    "[{}] order_chunks: queue={}, sent_chunks={}, pos=({},{})",
                    self.addr,
                    self.chunk_load_queue.len(),
                    self.sent_chunks.len(),
                    self.last_chunk_x,
                    self.last_chunk_z,
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
        self.send_chunk_batch()
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
            responses
                .push(self.encode_compressed_packet(packet_id::LEVEL_CHUNK, &chunk_pkt.encode()));
            self.sent_chunks.insert((cx, cz));
            sent += 1;
        }

        if sent > 0 {
            debug!(
                "[{}] send_chunk_batch: sent={}, queue_remaining={} (was {})",
                self.addr,
                sent,
                self.chunk_load_queue.len(),
                queue_before,
            );
        }

        responses
    }

    pub fn should_stream_chunks(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::PreSpawn | ConnectionState::SpawnResponse | ConnectionState::InGame
        )
    }
}
