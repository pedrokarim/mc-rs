use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, info, trace};

use crate::consts::*;
use crate::motd::Motd;
use crate::protocol::datagram::Reliability;
use crate::protocol::{id, offline};
use crate::session::{RakNetSession, SessionEvent};

/// A connected peer that the consumer can interact with.
pub struct RakNetPeer {
    pub addr: SocketAddr,
    pub event_rx: mpsc::UnboundedReceiver<SessionEvent>,
}

impl RakNetPeer {
    /// Receive the next event from this peer (connected, packet, disconnected).
    pub async fn recv(&mut self) -> Option<SessionEvent> {
        self.event_rx.recv().await
    }
}

/// The RakNet server — manages UDP socket and all sessions.
pub struct RakNetServer {
    socket: Arc<UdpSocket>,
    server_guid: i64,
    motd: Motd,
    sessions: HashMap<SocketAddr, RakNetSession>,
    pending_peers: Vec<RakNetPeer>,
}

impl RakNetServer {
    /// Bind the RakNet server to the given address.
    pub async fn bind(addr: SocketAddr, motd: Motd, server_guid: i64) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        info!("RakNet server listening on {}", addr);
        Ok(Self {
            socket: Arc::new(socket),
            server_guid,
            motd,
            sessions: HashMap::new(),
            pending_peers: Vec::new(),
        })
    }

    /// Take a newly connected peer (if any).
    pub fn accept(&mut self) -> Option<RakNetPeer> {
        self.pending_peers.pop()
    }

    /// Receive and process one UDP packet. Awaits until a packet arrives or timeout.
    /// Returns true if a packet was processed.
    pub async fn recv_and_process(&mut self) -> bool {
        let mut buf = [0u8; 2048];
        match self.socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                if len > 0 {
                    self.handle_packet(&buf[..len], addr).await;
                }
                true
            }
            Err(e) => {
                trace!("recv_from error: {}", e);
                false
            }
        }
    }

    /// Tick all sessions (flush ACKs, send pending, check timeouts).
    pub async fn tick_sessions(&mut self) {
        let mut disconnected = Vec::new();

        for (addr, session) in &mut self.sessions {
            session.tick().await;
            if session.is_disconnected() {
                disconnected.push(*addr);
            }
        }

        for addr in disconnected {
            self.sessions.remove(&addr);
            debug!("Session removed: {}", addr);
        }
    }

    /// Send a payload to a specific session.
    pub fn send_to_session(
        &mut self,
        addr: &SocketAddr,
        payload: Vec<u8>,
        reliability: Reliability,
        immediate: bool,
    ) {
        if let Some(session) = self.sessions.get_mut(addr) {
            session.send_payload(payload, reliability, immediate);
        }
    }

    async fn handle_packet(&mut self, packet: &[u8], addr: SocketAddr) {
        let packet_id = packet[0];
        trace!(
            "Received 0x{:02X} ({} bytes) from {}",
            packet_id,
            packet.len(),
            addr
        );

        match packet_id {
            id::UNCONNECTED_PING | id::UNCONNECTED_PING_OPEN => {
                self.handle_unconnected_ping(packet, addr).await;
            }
            id::OPEN_CONNECTION_REQUEST_1 => {
                self.handle_open_connection_request_1(packet, addr).await;
            }
            id::OPEN_CONNECTION_REQUEST_2 => {
                self.handle_open_connection_request_2(packet, addr).await;
            }
            _ if id::is_ack(packet_id)
                || id::is_nack(packet_id)
                || id::is_datagram(packet_id) =>
            {
                if let Some(session) = self.sessions.get_mut(&addr) {
                    session.handle_raw_packet(packet);
                }
            }
            _ => {
                trace!("Unknown packet 0x{:02X} from {}", packet_id, addr);
            }
        }
    }

    async fn handle_unconnected_ping(&self, packet: &[u8], addr: SocketAddr) {
        let Some((send_time, _)) = offline::decode_unconnected_ping(packet) else {
            return;
        };
        let motd_string = self.motd.to_string_payload();
        debug!("UnconnectedPing from {} — replying with MOTD", addr);
        let pong = offline::encode_unconnected_pong(send_time, self.server_guid, &motd_string);
        let _ = self.socket.send_to(&pong, addr).await;
    }

    async fn handle_open_connection_request_1(&self, packet: &[u8], addr: SocketAddr) {
        let Some((protocol, mtu_size)) = offline::decode_open_connection_request_1(packet) else {
            return;
        };

        debug!(
            "OpenConnectionRequest1 from {}: protocol={}, mtu={}",
            addr, protocol, mtu_size
        );

        if protocol != RAKNET_PROTOCOL_VERSION {
            let reply = offline::encode_incompatible_protocol(self.server_guid);
            let _ = self.socket.send_to(&reply, addr).await;
            return;
        }

        let reply_mtu = mtu_size.min(MAX_MTU_SIZE);
        let reply = offline::encode_open_connection_reply_1(self.server_guid, reply_mtu);
        let _ = self.socket.send_to(&reply, addr).await;
    }

    async fn handle_open_connection_request_2(&mut self, packet: &[u8], addr: SocketAddr) {
        let Some((_, mtu_size, client_guid)) =
            offline::decode_open_connection_request_2(packet)
        else {
            return;
        };

        let mtu = mtu_size.clamp(MIN_MTU_SIZE, MAX_MTU_SIZE);

        debug!(
            "OpenConnectionRequest2 from {}: mtu={}, guid={}",
            addr, mtu, client_guid
        );

        let reply = offline::encode_open_connection_reply_2(self.server_guid, &addr, mtu);
        let _ = self.socket.send_to(&reply, addr).await;

        // Create session with event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let session =
            RakNetSession::new(addr, client_guid, mtu, Arc::clone(&self.socket), event_tx);

        info!(
            "Session created for {} (guid={}, mtu={})",
            addr, client_guid, mtu
        );
        self.sessions.insert(addr, session);

        // Create peer for the consumer
        self.pending_peers.push(RakNetPeer { addr, event_rx });
    }

    /// Update the MOTD.
    pub fn set_motd(&mut self, motd: Motd) {
        self.motd = motd;
    }

    /// Get active session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the socket for external use.
    pub fn socket(&self) -> &Arc<UdpSocket> {
        &self.socket
    }
}
