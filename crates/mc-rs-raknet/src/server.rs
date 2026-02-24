use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::{Bytes, BytesMut};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, trace, warn};

use crate::address::RakNetAddress;
use crate::constants::*;
use crate::error::RakNetError;
use crate::packet::frame::{AckNack, FrameSet, Reliability};
use crate::packet::offline::{self, OfflinePacket, ServerMotd};
use crate::packet::online::{self, OnlinePacket};
use crate::reliability::compress_ack_records;
use crate::session::{RakNetSession, SessionState};

/// Events emitted by the RakNet server to the consumer.
#[derive(Debug)]
pub enum RakNetEvent {
    /// A new session has completed the full RakNet handshake.
    SessionConnected { addr: SocketAddr, guid: i64 },
    /// A session has disconnected.
    SessionDisconnected { addr: SocketAddr },
    /// A fully reassembled, ordered payload from a connected session.
    Packet { addr: SocketAddr, payload: Bytes },
}

/// Commands that can be sent to the RakNet server from another task.
#[derive(Debug)]
pub enum ServerCommand {
    /// Send a payload to a connected session.
    Send {
        addr: SocketAddr,
        payload: Bytes,
        reliability: Reliability,
        channel: u8,
    },
}

/// A cloneable handle for sending commands to the RakNet server from any task.
#[derive(Clone)]
pub struct ServerHandle {
    command_tx: mpsc::Sender<ServerCommand>,
}

impl ServerHandle {
    /// Queue a payload to be sent to a connected session.
    pub async fn send_to(
        &self,
        addr: SocketAddr,
        payload: Bytes,
        reliability: Reliability,
        channel: u8,
    ) {
        let _ = self
            .command_tx
            .send(ServerCommand::Send {
                addr,
                payload,
                reliability,
                channel,
            })
            .await;
    }
}

/// Configuration for the RakNet server.
pub struct RakNetConfig {
    pub address: SocketAddr,
    pub server_guid: i64,
    pub motd: ServerMotd,
    pub max_connections: usize,
}

/// The RakNet server — manages UDP socket and all sessions.
pub struct RakNetServer {
    socket: Arc<UdpSocket>,
    sessions: HashMap<SocketAddr, RakNetSession>,
    config: RakNetConfig,
    event_tx: mpsc::Sender<RakNetEvent>,
    command_rx: mpsc::Receiver<ServerCommand>,
}

impl RakNetServer {
    /// Bind the UDP socket and create the server. Returns the server, an
    /// event receiver for the consumer, and a handle for sending commands.
    pub async fn bind(
        config: RakNetConfig,
    ) -> Result<(Self, mpsc::Receiver<RakNetEvent>, ServerHandle), RakNetError> {
        let socket = UdpSocket::bind(config.address).await?;
        let (event_tx, event_rx) = mpsc::channel(256);
        let (command_tx, command_rx) = mpsc::channel(256);

        info!("RakNet server bound on {}", config.address);

        Ok((
            Self {
                socket: Arc::new(socket),
                sessions: HashMap::new(),
                config,
                event_tx,
                command_rx,
            },
            event_rx,
            ServerHandle { command_tx },
        ))
    }

    /// Run the server main loop. Blocks until the shutdown signal is received.
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) {
        let mut recv_buf = vec![0u8; RECV_BUF_SIZE];
        let mut tick_interval = tokio::time::interval(SERVER_TICK_INTERVAL);

        loop {
            tokio::select! {
                result = self.socket.recv_from(&mut recv_buf) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Err(e) = self.handle_datagram(&recv_buf[..len], addr).await {
                                trace!("Error handling datagram from {addr}: {e}");
                            }
                        }
                        Err(e) => warn!("UDP recv error: {e}"),
                    }
                }
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        ServerCommand::Send { addr, payload, reliability, channel } => {
                            self.send_to(addr, payload, reliability, channel);
                        }
                    }
                }
                _ = tick_interval.tick() => {
                    self.tick().await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("RakNet server shutting down");
                        self.shutdown().await;
                        break;
                    }
                }
            }
        }
    }

    /// Update the MOTD (e.g. when player count changes).
    pub fn update_motd(&mut self, motd: ServerMotd) {
        self.config.motd = motd;
    }

    /// Send a payload to a connected session.
    pub fn send_to(
        &mut self,
        addr: SocketAddr,
        payload: Bytes,
        reliability: Reliability,
        channel: u8,
    ) {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.queue_frame(payload, reliability, channel);
        }
    }

    // -----------------------------------------------------------------------
    // Internal: datagram dispatch
    // -----------------------------------------------------------------------

    async fn handle_datagram(&mut self, data: &[u8], addr: SocketAddr) -> Result<(), RakNetError> {
        if data.is_empty() {
            return Ok(());
        }

        let packet_id = data[0];

        match packet_id {
            // Offline packets
            offline::id::UNCONNECTED_PING | offline::id::UNCONNECTED_PING_OPEN => {
                self.handle_unconnected_ping(data, addr).await
            }
            offline::id::OPEN_CONNECTION_REQUEST_1 => {
                self.handle_open_connection_request_1(data, addr).await
            }
            offline::id::OPEN_CONNECTION_REQUEST_2 => {
                self.handle_open_connection_request_2(data, addr).await
            }
            // Online packets
            0x80..=0x8D => self.handle_frameset(data, addr).await,
            AckNack::ACK_ID => self.handle_ack(data, addr),
            AckNack::NACK_ID => self.handle_nack(data, addr),
            _ => {
                trace!("Unknown packet 0x{packet_id:02X} from {addr}");
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------
    // Offline packet handlers
    // -----------------------------------------------------------------------

    async fn handle_unconnected_ping(
        &self,
        data: &[u8],
        addr: SocketAddr,
    ) -> Result<(), RakNetError> {
        let ping = OfflinePacket::decode(data, data.len())?;
        if let OfflinePacket::UnconnectedPing {
            send_timestamp,
            client_guid: _,
        } = ping
        {
            let motd = self.config.motd.to_motd_string();
            let pong = OfflinePacket::UnconnectedPong {
                send_timestamp,
                server_guid: self.config.server_guid,
                motd,
            };
            let mut buf = BytesMut::with_capacity(256);
            pong.encode(&mut buf);
            self.socket.send_to(&buf, addr).await?;
            trace!("Sent UnconnectedPong to {addr}");
        }
        Ok(())
    }

    async fn handle_open_connection_request_1(
        &mut self,
        data: &[u8],
        addr: SocketAddr,
    ) -> Result<(), RakNetError> {
        let packet = OfflinePacket::decode(data, data.len())?;
        if let OfflinePacket::OpenConnectionRequest1 {
            protocol_version,
            mtu_size,
        } = packet
        {
            if protocol_version != RAKNET_PROTOCOL_VERSION {
                debug!(
                    "OCR1 from {addr}: wrong protocol version {protocol_version} (expected {RAKNET_PROTOCOL_VERSION})"
                );
                // Still respond so client gets feedback
            }

            let mtu = mtu_size.clamp(MIN_MTU, MAX_MTU);
            debug!("OCR1 from {addr}: MTU={mtu}");

            // Create or update session
            self.sessions
                .entry(addr)
                .or_insert_with(|| RakNetSession::new(addr, mtu, 0));

            let reply = OfflinePacket::OpenConnectionReply1 {
                server_guid: self.config.server_guid,
                use_security: false,
                mtu_size: mtu,
            };
            let mut buf = BytesMut::with_capacity(64);
            reply.encode(&mut buf);
            self.socket.send_to(&buf, addr).await?;
        }
        Ok(())
    }

    async fn handle_open_connection_request_2(
        &mut self,
        data: &[u8],
        addr: SocketAddr,
    ) -> Result<(), RakNetError> {
        let packet = OfflinePacket::decode(data, data.len())?;
        if let OfflinePacket::OpenConnectionRequest2 {
            mtu_size,
            client_guid,
            ..
        } = packet
        {
            if self.sessions.len() >= self.config.max_connections {
                debug!("OCR2 from {addr}: max connections reached, rejecting");
                return Ok(());
            }

            let session = self
                .sessions
                .entry(addr)
                .or_insert_with(|| RakNetSession::new(addr, mtu_size, client_guid));

            let final_mtu = mtu_size.min(session.mtu).clamp(MIN_MTU, MAX_MTU);
            session.mtu = final_mtu;
            session.client_guid = client_guid;
            session.state = SessionState::HandshakeCompleted;
            session.last_activity = Instant::now();

            debug!("OCR2 from {addr}: GUID={client_guid}, MTU={final_mtu}");

            let reply = OfflinePacket::OpenConnectionReply2 {
                server_guid: self.config.server_guid,
                client_address: RakNetAddress::from(addr),
                mtu_size: final_mtu,
                encryption_enabled: false,
            };
            let mut buf = BytesMut::with_capacity(64);
            reply.encode(&mut buf);
            self.socket.send_to(&buf, addr).await?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Online packet handlers
    // -----------------------------------------------------------------------

    async fn handle_frameset(&mut self, data: &[u8], addr: SocketAddr) -> Result<(), RakNetError> {
        let session = match self.sessions.get_mut(&addr) {
            Some(s) => s,
            None => {
                trace!("FrameSet from unknown session {addr}");
                return Ok(());
            }
        };

        session.last_activity = Instant::now();
        let frameset = FrameSet::decode(data)?;
        let payloads = session.process_incoming_frameset(frameset);

        // We need to collect state before processing to avoid borrow issues
        let session_state = session.state;
        let session_guid = session.client_guid;

        for payload in payloads {
            if payload.is_empty() {
                continue;
            }
            let inner_id = payload[0];
            match inner_id {
                online::id::CONNECTION_REQUEST => {
                    self.handle_connection_request(&payload, addr).await?;
                }
                online::id::NEW_INCOMING_CONNECTION => {
                    self.handle_new_incoming_connection(addr, session_state, session_guid)
                        .await?;
                }
                online::id::CONNECTED_PING => {
                    self.handle_connected_ping(&payload, addr).await?;
                }
                online::id::CONNECTED_PONG => {
                    // Update activity (already done above)
                }
                online::id::DISCONNECTION_NOTIFICATION => {
                    info!("Session {addr} sent disconnect notification");
                    self.sessions.remove(&addr);
                    let _ = self
                        .event_tx
                        .send(RakNetEvent::SessionDisconnected { addr })
                        .await;
                    return Ok(());
                }
                0xFE => {
                    // Game packet wrapper — forward to consumer
                    let _ = self
                        .event_tx
                        .send(RakNetEvent::Packet {
                            addr,
                            payload: payload.slice(1..), // Skip the 0xFE byte
                        })
                        .await;
                }
                _ => {
                    trace!("Unknown online packet 0x{inner_id:02X} from {addr}");
                }
            }
        }
        Ok(())
    }

    async fn handle_connection_request(
        &mut self,
        data: &[u8],
        addr: SocketAddr,
    ) -> Result<(), RakNetError> {
        let packet = OnlinePacket::decode(data)?;
        if let OnlinePacket::ConnectionRequest {
            client_guid,
            timestamp,
            ..
        } = packet
        {
            let session = match self.sessions.get_mut(&addr) {
                Some(s) => s,
                None => return Ok(()),
            };

            session.client_guid = client_guid;
            session.state = SessionState::ConnectionPending;

            debug!("ConnectionRequest from {addr}, GUID={client_guid}");

            let mut system_addresses = [RakNetAddress::EMPTY_V4; 20];
            system_addresses[0] = RakNetAddress::from(addr);

            let accepted = OnlinePacket::ConnectionRequestAccepted {
                client_address: RakNetAddress::from(addr),
                system_index: 0,
                system_addresses,
                request_timestamp: timestamp,
                accept_timestamp: current_timestamp(),
            };

            let mut payload = BytesMut::with_capacity(256);
            accepted.encode(&mut payload);

            session.queue_frame(payload.freeze(), Reliability::ReliableOrdered, 0);
        }
        Ok(())
    }

    async fn handle_new_incoming_connection(
        &mut self,
        addr: SocketAddr,
        _state: SessionState,
        guid: i64,
    ) -> Result<(), RakNetError> {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.state = SessionState::Connected;
            info!("Session {addr} fully connected (GUID={guid})");
            let _ = self
                .event_tx
                .send(RakNetEvent::SessionConnected { addr, guid })
                .await;
        }
        Ok(())
    }

    async fn handle_connected_ping(
        &mut self,
        data: &[u8],
        addr: SocketAddr,
    ) -> Result<(), RakNetError> {
        let packet = OnlinePacket::decode(data)?;
        if let OnlinePacket::ConnectedPing { timestamp } = packet {
            let pong = OnlinePacket::ConnectedPong {
                ping_timestamp: timestamp,
                pong_timestamp: current_timestamp(),
            };
            let mut payload = BytesMut::with_capacity(32);
            pong.encode(&mut payload);

            if let Some(session) = self.sessions.get_mut(&addr) {
                session.queue_frame(payload.freeze(), Reliability::Unreliable, 0);
            }
        }
        Ok(())
    }

    fn handle_ack(&mut self, data: &[u8], addr: SocketAddr) -> Result<(), RakNetError> {
        let ack = AckNack::decode(data)?;
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.last_activity = Instant::now();
            session.handle_ack(&ack);
        }
        Ok(())
    }

    fn handle_nack(&mut self, data: &[u8], addr: SocketAddr) -> Result<(), RakNetError> {
        let nack = AckNack::decode(data)?;
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.last_activity = Instant::now();
            session.handle_nack(&nack);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Server tick
    // -----------------------------------------------------------------------

    async fn tick(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        let mut to_send: Vec<(SocketAddr, Bytes)> = Vec::new();

        for (addr, session) in &mut self.sessions {
            // Skip sessions still in offline handshake
            if session.state == SessionState::Connecting {
                if session.is_timed_out(now) {
                    to_remove.push(*addr);
                }
                continue;
            }

            // Flush ACK queue
            if !session.ack_queue.is_empty() {
                let records = compress_ack_records(&mut session.ack_queue);
                let ack = AckNack {
                    is_ack: true,
                    records,
                };
                let mut buf = BytesMut::with_capacity(64);
                ack.encode(&mut buf);
                to_send.push((*addr, buf.freeze()));
                session.ack_queue.clear();
            }

            // Check retransmission
            session.check_retransmit(now);

            // Flush send queue
            let datagrams = session.flush_send_queue();
            for dg in datagrams {
                to_send.push((*addr, dg));
            }

            // Send ConnectedPing if needed
            if session.state == SessionState::Connected && session.should_ping(now) {
                let ping = OnlinePacket::ConnectedPing {
                    timestamp: current_timestamp(),
                };
                let mut payload = BytesMut::with_capacity(16);
                ping.encode(&mut payload);
                session.queue_frame(payload.freeze(), Reliability::Unreliable, 0);
                session.last_ping_sent = now;
            }

            // Cleanup stale fragments
            session.cleanup_fragments();

            // Check timeout
            if session.is_timed_out(now) {
                to_remove.push(*addr);
            }
        }

        // Send all queued datagrams
        for (addr, data) in to_send {
            let _ = self.socket.send_to(&data, addr).await;
        }

        // Remove timed-out sessions
        for addr in to_remove {
            let was_connected = self
                .sessions
                .get(&addr)
                .map(|s| s.state == SessionState::Connected)
                .unwrap_or(false);
            self.sessions.remove(&addr);
            if was_connected {
                info!("Session {addr} timed out");
                let _ = self
                    .event_tx
                    .send(RakNetEvent::SessionDisconnected { addr })
                    .await;
            }
        }
    }

    async fn shutdown(&mut self) {
        // Send disconnect notification to all connected sessions
        for (addr, session) in &mut self.sessions {
            if session.state == SessionState::Connected {
                let disconnect = OnlinePacket::DisconnectionNotification;
                let mut payload = BytesMut::with_capacity(4);
                disconnect.encode(&mut payload);
                session.queue_frame(payload.freeze(), Reliability::ReliableOrdered, 0);

                let datagrams = session.flush_send_queue();
                for dg in datagrams {
                    let _ = self.socket.send_to(&dg, *addr).await;
                }
            }
        }
        self.sessions.clear();
    }
}

/// Get the current time in milliseconds since an arbitrary epoch (process start).
fn current_timestamp() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
