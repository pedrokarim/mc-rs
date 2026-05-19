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

/// Désactive `SIO_UDP_CONNRESET` sur le socket UDP (Windows uniquement).
///
/// Sans ça, recevoir un ICMP "port unreachable" (client fermé/en pause)
/// transforme tous les `recv_from` suivants en `Err(WSAECONNRESET)` →
/// busy-spin du main-loop. Référence : MSDN `SIO_UDP_CONNRESET`, comportement
/// connu de Winsock pour les sockets UDP. No-op sur les autres OS.
#[cfg(windows)]
fn disable_udp_conn_reset(socket: &UdpSocket) {
    use std::os::windows::io::AsRawSocket;

    // SIO_UDP_CONNRESET = _WSAIOW(IOC_VENDOR, 12) = 0x9800000C
    const SIO_UDP_CONNRESET: u32 = 0x9800_000C;

    #[allow(non_camel_case_types)]
    type SOCKET = usize;

    extern "system" {
        fn WSAIoctl(
            s: SOCKET,
            dw_io_control_code: u32,
            lpv_in_buffer: *const std::ffi::c_void,
            cb_in_buffer: u32,
            lpv_out_buffer: *mut std::ffi::c_void,
            cb_out_buffer: u32,
            lpcb_bytes_returned: *mut u32,
            lp_overlapped: *mut std::ffi::c_void,
            lp_completion_routine: *mut std::ffi::c_void,
        ) -> i32;
    }

    let handle = socket.as_raw_socket() as SOCKET;
    let mut enable: i32 = 0; // FALSE → désactive le comportement CONNRESET
    let mut bytes_returned: u32 = 0;
    // SAFETY : appel Winsock standard ; `enable` vit toute la durée de l'appel,
    // les pointeurs out sont valides, le socket est ouvert (créé juste avant).
    let ret = unsafe {
        WSAIoctl(
            handle,
            SIO_UDP_CONNRESET,
            &mut enable as *mut i32 as *const std::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned as *mut u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ret != 0 {
        tracing::warn!(
            "SIO_UDP_CONNRESET désactivation échouée (WSAIoctl ret={ret}); \
             le serveur reste vulnérable au spin si un client ferme brutalement"
        );
    } else {
        tracing::info!("SIO_UDP_CONNRESET désactivé (anti-spin Windows UDP)");
    }
}

#[cfg(not(windows))]
fn disable_udp_conn_reset(_socket: &UdpSocket) {}

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
        // Fix Windows critique : sans ceci, quand un client se met en pause
        // ou ferme brutalement, le `send_to` suivant vers son port mort
        // déclenche un ICMP "port unreachable" qui fait échouer TOUS les
        // `recv_from` suivants avec WSAECONNRESET (os error 10054), en
        // boucle → `recv_and_process` spin → le `tick_timer` est affamé →
        // serveur figé à 100 % CPU (process vivant, plus aucun tick). On
        // désactive ce comportement via SIO_UDP_CONNRESET = false.
        disable_udp_conn_reset(&socket);
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
                // Visibilité : log rate-limité (1er + tous les 500) de
                // l'erreur EXACTE (kind + code OS brut) pour diagnostiquer le
                // spin recv. trace! était filtré par RUST_LOG=info → invisible.
                use std::sync::atomic::{AtomicU64, Ordering};
                static ERR_COUNT: AtomicU64 = AtomicU64::new(0);
                let n = ERR_COUNT.fetch_add(1, Ordering::Relaxed);
                if n == 0 || n % 500 == 0 {
                    tracing::warn!(
                        "recv_from error #{n}: kind={:?} raw_os={:?} msg={}",
                        e.kind(),
                        e.raw_os_error(),
                        e
                    );
                }
                // Garde-fou anti-spin : si une erreur recv_from persiste, ne
                // PAS rendre la main instantanément (sinon l'arm `recv` du
                // select! monopolise le runtime et affame `tick_timer`). Petit
                // backoff au lieu d'un simple yield (yield seul ne suffit pas
                // si l'erreur est instantanée et continue).
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
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
            _ if id::is_ack(packet_id) || id::is_nack(packet_id) || id::is_datagram(packet_id) => {
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
        // Ping broadcast fire 1/s/interface → trop bruyant même en debug. Muted.
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
        let Some((_, mtu_size, client_guid)) = offline::decode_open_connection_request_2(packet)
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
