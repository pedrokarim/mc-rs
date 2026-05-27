use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::consts::*;
use crate::protocol::datagram::*;
use crate::protocol::online::*;
use crate::protocol::{datagram, id};
use crate::reliability::recv::ReceiveLayer;
use crate::reliability::send::SendLayer;

/// Session state in the RakNet connection lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
}

/// Event emitted by a session to the server.
#[derive(Debug)]
pub enum SessionEvent {
    /// Session completed the full RakNet handshake.
    Connected,
    /// Session disconnected.
    Disconnected,
    /// A user-level packet (0xFE game batch) was received.
    Packet(Vec<u8>),
}

/// A RakNet session for a single connected client.
pub struct RakNetSession {
    pub addr: SocketAddr,
    pub client_guid: i64,
    pub mtu: u16,
    pub state: SessionState,

    socket: Arc<UdpSocket>,
    recv_layer: ReceiveLayer,
    send_layer: SendLayer,

    start_time: Instant,
    last_recv_time: Instant,
    last_ping_time: Instant,

    // ── Freeze diagnostics ──
    diag_last_log: Instant,
    diag_last_snapshot: Instant,
    /// (datagrams_in + dropped_window, packets_out) at last diag checkpoint.
    diag_last_in: u64,
    diag_last_out: u64,
    diag_last_progress: Instant,
    diag_frozen_reported: bool,

    event_tx: mpsc::UnboundedSender<SessionEvent>,
}

impl RakNetSession {
    pub fn new(
        addr: SocketAddr,
        client_guid: i64,
        mtu: u16,
        socket: Arc<UdpSocket>,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
    ) -> Self {
        Self {
            addr,
            client_guid,
            mtu,
            state: SessionState::Connecting,
            socket,
            recv_layer: ReceiveLayer::new(),
            send_layer: SendLayer::new(mtu),
            start_time: Instant::now(),
            last_recv_time: Instant::now(),
            last_ping_time: Instant::now(),
            diag_last_log: Instant::now(),
            diag_last_snapshot: Instant::now(),
            diag_last_in: 0,
            diag_last_out: 0,
            diag_last_progress: Instant::now(),
            diag_frozen_reported: false,
            event_tx,
        }
    }

    /// Get elapsed time in milliseconds since session start.
    fn elapsed_ms(&self) -> i64 {
        self.start_time.elapsed().as_millis() as i64
    }

    /// Handle a raw packet received from the UDP socket.
    pub fn handle_raw_packet(&mut self, packet: &[u8]) {
        if packet.is_empty() {
            return;
        }
        self.last_recv_time = Instant::now();

        let packet_id = packet[0];

        if id::is_ack(packet_id) {
            self.handle_ack(packet);
        } else if id::is_nack(packet_id) {
            self.handle_nack(packet);
        } else if id::is_datagram(packet_id) {
            self.handle_datagram(packet);
        }
    }

    fn handle_ack(&mut self, packet: &[u8]) {
        let seq_numbers = datagram::decode_ack_records(packet);
        trace!("ACK from {}: {:?}", self.addr, seq_numbers);
        self.send_layer.on_ack(&seq_numbers);
    }

    fn handle_nack(&mut self, packet: &[u8]) {
        let seq_numbers = datagram::decode_ack_records(packet);
        trace!("NACK from {}: {:?}", self.addr, seq_numbers);
        self.send_layer.on_nack(&seq_numbers);
    }

    fn handle_datagram(&mut self, packet: &[u8]) {
        let Some(dg) = Datagram::decode(packet) else {
            warn!("Failed to decode datagram from {}", self.addr);
            return;
        };

        trace!(
            "Datagram seq={} with {} packets from {}",
            dg.seq_number,
            dg.packets.len(),
            self.addr
        );

        let user_packets = self.recv_layer.on_datagram(dg.seq_number, dg.packets);

        for payload in user_packets {
            self.handle_encapsulated_payload(&payload);
        }
    }

    fn handle_encapsulated_payload(&mut self, payload: &[u8]) {
        if payload.is_empty() {
            return;
        }

        let packet_id = payload[0];

        if packet_id < id::USER_PACKET_ENUM {
            // Internal RakNet packet
            self.handle_internal_packet(packet_id, payload);
        } else if self.state == SessionState::Connected {
            // User packet (game data)
            let _ = self.event_tx.send(SessionEvent::Packet(payload.to_vec()));
        }
    }

    fn handle_internal_packet(&mut self, packet_id: u8, payload: &[u8]) {
        match packet_id {
            id::CONNECTION_REQUEST => {
                self.handle_connection_request(payload);
            }
            id::NEW_INCOMING_CONNECTION => {
                self.handle_new_incoming_connection(payload);
            }
            id::CONNECTED_PING => {
                self.handle_connected_ping(payload);
            }
            id::DISCONNECTION_NOTIFICATION => {
                debug!("Disconnection notification from {}", self.addr);
                self.state = SessionState::Disconnected;
                let _ = self.event_tx.send(SessionEvent::Disconnected);
            }
            _ => {
                trace!(
                    "Unhandled internal packet 0x{:02X} from {}",
                    packet_id,
                    self.addr
                );
            }
        }
    }

    fn handle_connection_request(&mut self, payload: &[u8]) {
        let Some((_client_guid, send_time, _security)) = decode_connection_request(payload) else {
            warn!("Invalid ConnectionRequest from {}", self.addr);
            return;
        };

        debug!("ConnectionRequest from {} (time={})", self.addr, send_time);

        let pong_time = self.elapsed_ms();
        let response = encode_connection_request_accepted(&self.addr, send_time, pong_time);

        // Send as unreliable (as PMMP does for the first response)
        self.send_raw_encapsulated(response, Reliability::Unreliable);
    }

    fn handle_new_incoming_connection(&mut self, payload: &[u8]) {
        if decode_new_incoming_connection(payload).is_none() {
            warn!("Invalid NewIncomingConnection from {}", self.addr);
            return;
        }

        debug!(
            "NewIncomingConnection from {} — session connected!",
            self.addr
        );
        self.state = SessionState::Connected;
        let _ = self.event_tx.send(SessionEvent::Connected);
    }

    fn handle_connected_ping(&mut self, payload: &[u8]) {
        let Some(ping_time) = decode_connected_ping(payload) else {
            return;
        };
        let pong_time = self.elapsed_ms();
        let response = encode_connected_pong(ping_time, pong_time);
        self.send_raw_encapsulated(response, Reliability::Unreliable);
    }

    /// Send a user-level payload (e.g., 0xFE game batch) through the reliability layer.
    pub fn send_payload(&mut self, payload: Vec<u8>, reliability: Reliability, immediate: bool) {
        let pkt = EncapsulatedPacket {
            reliability,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: if reliability.is_sequenced_or_ordered() {
                Some(0)
            } else {
                None
            },
            split: None,
            body: payload,
        };
        self.send_layer.add_packet(pkt, immediate);
    }

    /// Send an internal RakNet packet.
    fn send_raw_encapsulated(&mut self, data: Vec<u8>, reliability: Reliability) {
        let pkt = EncapsulatedPacket {
            reliability,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: data,
        };
        self.send_layer.add_packet(pkt, true);
    }

    /// Tick the session: flush ACKs/NACKs, send pending datagrams, check timeouts.
    /// Should be called regularly (e.g., every 10ms).
    pub async fn tick(&mut self) {
        let now_secs = self.start_time.elapsed().as_secs_f64();

        // Check session timeout
        if self.last_recv_time.elapsed().as_secs_f64() > SESSION_TIMEOUT_SECS {
            debug!("Session timeout for {}", self.addr);
            self.state = SessionState::Disconnected;
            let _ = self.event_tx.send(SessionEvent::Disconnected);
            return;
        }

        // Unrecoverable RakNet error (e.g. bad split packet): RakLib's design
        // mandates disconnecting the peer rather than silently dropping, which
        // would wedge the ordered channel forever. The client then reconnects
        // with a fresh reliability layer.
        if let Some(reason) = self.recv_layer.fatal_error() {
            warn!(
                "Disconnecting {} — fatal RakNet error: {}",
                self.addr, reason
            );
            self.state = SessionState::Disconnected;
            let _ = self.event_tx.send(SessionEvent::Disconnected);
            return;
        }

        // ── Freeze diagnostics (per-session) ──
        // Checked once/sec. The freeze fingerprint: the client is still
        // sending datagrams (in_total climbs) but NOTHING is delivered to the
        // game layer (out_total frozen) for several seconds. When that
        // happens we dump the FULL receive-layer state so the next repro tells
        // us EXACTLY which mechanism wedged — no more guessing.
        if self.state == SessionState::Connected
            && self.diag_last_log.elapsed().as_secs_f64() >= 1.0
        {
            self.diag_last_log = Instant::now();
            let d = self.recv_layer.diag();
            let in_total = d.datagrams_in + d.dropped_window;
            let out_total = d.packets_out;

            if out_total > self.diag_last_out {
                // Game packets are flowing — session healthy, re-arm detector.
                self.diag_last_progress = Instant::now();
                self.diag_frozen_reported = false;
            }

            let stalled_secs = self.diag_last_progress.elapsed().as_secs_f64();
            let client_still_sending = in_total > self.diag_last_in;

            if client_still_sending && out_total == self.diag_last_out && stalled_secs > 4.0 {
                if !self.diag_frozen_reported {
                    self.diag_frozen_reported = true;
                    warn!(
                        "FREEZE DETECTED {} stalled {:.0}s: in={} (drop_win={}) out={} | \
                         dgram_win={}..{} hi_seq={} | reliable_win={}..{} held={} | \
                         ordered={:?} | splits={} nack_pending={}",
                        self.addr,
                        stalled_secs,
                        d.datagrams_in,
                        d.dropped_window,
                        d.packets_out,
                        d.window_start,
                        d.window_end,
                        d.highest_seq,
                        d.reliable_window_start,
                        d.reliable_window_end,
                        d.reliable_held,
                        d.ordered_channels,
                        d.split_in_progress,
                        d.nack_pending,
                    );
                }
            } else if self.diag_last_snapshot.elapsed().as_secs_f64() >= 30.0 {
                self.diag_last_snapshot = Instant::now();
                tracing::info!(
                    "recv {} ok: in={} out={} dgram_win={}..{} rel_win={}..{} ord={:?} splits={}",
                    self.addr,
                    d.datagrams_in,
                    d.packets_out,
                    d.window_start,
                    d.window_end,
                    d.reliable_window_start,
                    d.reliable_window_end,
                    d.ordered_channels,
                    d.split_in_progress,
                );
            }

            self.diag_last_in = in_total;
            self.diag_last_out = out_total;
        }

        // Send ping periodically
        if self.state == SessionState::Connected
            && self.last_ping_time.elapsed().as_secs_f64() > PING_INTERVAL_SECS
        {
            self.last_ping_time = Instant::now();
            let ping = encode_connected_ping(self.elapsed_ms());
            self.send_raw_encapsulated(ping, Reliability::Unreliable);
        }

        // Check retransmission timeouts
        self.send_layer.check_timeouts(now_secs);

        // Flush send layer
        self.send_layer.flush_current_datagram();
        while let Some(dg) = self.send_layer.next_datagram(now_secs) {
            let encoded = dg.encode();
            let _ = self.socket.send_to(&encoded, self.addr).await;
        }

        // Force-advance the receive window past any NACKed-but-unrecovered
        // datagrams (RakLib ReceiveReliabilityLayer::update). MUST run before
        // the ACK/NACK flush. Without this the receive window wedges on a
        // single lost datagram and the session silently stops processing all
        // client packets (per-session freeze bug).
        self.recv_layer.update();

        // Send ACKs
        if !self.recv_layer.ack_queue.is_empty() {
            let ack_pkt = datagram::encode_ack_nack(id::ACK, &mut self.recv_layer.ack_queue);
            if !ack_pkt.is_empty() {
                let _ = self.socket.send_to(&ack_pkt, self.addr).await;
            }
            self.recv_layer.ack_queue.clear();
        }

        // Send NACKs
        if !self.recv_layer.nack_queue.is_empty() {
            let nack_pkt = datagram::encode_ack_nack(id::NACK, &mut self.recv_layer.nack_queue);
            if !nack_pkt.is_empty() {
                let _ = self.socket.send_to(&nack_pkt, self.addr).await;
            }
            self.recv_layer.nack_queue.clear();
        }
    }

    /// Check if the session is disconnected.
    pub fn is_disconnected(&self) -> bool {
        self.state == SessionState::Disconnected
    }
}
