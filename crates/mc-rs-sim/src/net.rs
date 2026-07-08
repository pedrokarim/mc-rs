//! Client RakNet — pendant « côté joueur » de `mc-rs-raknet::session`.
//!
//! Réutilise tel quel `SendLayer` / `ReceiveLayer` (la fiabilité RakNet est
//! symétrique) et pilote le handshake de connexion depuis le point de vue du
//! client : `OpenConnectionRequest1/2` → `ConnectionRequest` →
//! `NewIncomingConnection`. Une fois connecté, expose l'envoi/réception de
//! batches de jeu (paquets `0xFE`) que la couche protocole Bedrock consomme.

use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::time::timeout;
use tracing::{debug, trace};

use mc_rs_raknet::consts::*;
use mc_rs_raknet::protocol::datagram::{
    decode_ack_records, encode_ack_nack, Datagram, EncapsulatedPacket, Reliability,
};
use mc_rs_raknet::protocol::id;
use mc_rs_raknet::protocol::offline::*;
use mc_rs_raknet::protocol::online::*;
use mc_rs_raknet::reliability::recv::ReceiveLayer;
use mc_rs_raknet::reliability::send::SendLayer;

/// Erreur de connexion RakNet côté client.
#[derive(Debug, thiserror::Error)]
pub enum RakError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("timeout pendant le handshake RakNet ({0})")]
    HandshakeTimeout(&'static str),
    #[error("le serveur a répondu IncompatibleProtocolVersion")]
    IncompatibleProtocol,
    #[error("déconnecté par le serveur pendant le handshake")]
    Disconnected,
}

/// Étape courante du handshake RakNet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Requête 1 envoyée, on attend Reply1.
    AwaitReply1,
    /// Requête 2 envoyée, on attend Reply2.
    AwaitReply2,
    /// ConnectionRequest envoyée, on attend ConnectionRequestAccepted.
    AwaitAccepted,
    /// Connecté au niveau RakNet (NewIncomingConnection envoyé).
    Connected,
}

/// Un client RakNet connecté à un serveur.
pub struct RakClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
    client_guid: i64,
    mtu: u16,
    stage: Stage,

    send: SendLayer,
    recv: ReceiveLayer,

    /// Paquets « offline » bruts en attente d'émission (requêtes de handshake,
    /// ping/pong non-connectés). Envoyés directement sur le socket, hors couche
    /// de fiabilité.
    raw_out: VecDeque<Vec<u8>>,
    /// Batches de jeu reçus (`0xFE …`), prêts à être décodés par la couche proto.
    incoming: VecDeque<Vec<u8>>,

    start: Instant,
    last_recv: Instant,
    last_ping: Instant,
    disconnected: bool,

    /// RTT du dernier ConnectedPing/Pong, en millisecondes.
    pub last_rtt_ms: Option<f64>,
}

impl RakClient {
    /// Ouvre une connexion RakNet complète vers `server_addr`.
    ///
    /// Effectue tout le handshake et ne rend la main qu'une fois connecté (ou
    /// en erreur / timeout).
    pub async fn open(server_addr: SocketAddr, mtu_target: u16) -> Result<Self, RakError> {
        // Socket éphémère IPv4 (le protocole d'adresse RakNet ne gère que v4 ici).
        let socket = UdpSocket::bind::<SocketAddr>("0.0.0.0:0".parse().unwrap()).await?;
        socket.connect(server_addr).await?;

        let client_guid: i64 = rand::random();
        let mtu = mtu_target.clamp(MIN_MTU_SIZE, MAX_MTU_SIZE);

        let mut client = Self {
            socket,
            server_addr,
            client_guid,
            mtu,
            stage: Stage::AwaitReply1,
            send: SendLayer::new(mtu),
            recv: ReceiveLayer::new(),
            raw_out: VecDeque::new(),
            incoming: VecDeque::new(),
            start: Instant::now(),
            last_recv: Instant::now(),
            last_ping: Instant::now(),
            disconnected: false,
            last_rtt_ms: None,
        };

        // Étape 1 : OpenConnectionRequest1 (rembourré à la MTU cible), envoyé
        // immédiatement.
        client.resend_current_request();
        client.flush().await;

        // Comme un vrai client RakNet, on réémet la requête offline de l'étape
        // courante tant qu'elle n'a pas de réponse (les paquets offline ne
        // passent pas par la couche de fiabilité, donc pas de retransmission
        // automatique). ConnectionRequest, lui, est reliable → réémis par la
        // couche d'envoi.
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut last_resend = Instant::now();
        while client.stage != Stage::Connected {
            if client.disconnected {
                return Err(RakError::Disconnected);
            }
            if Instant::now() >= deadline {
                let where_ = match client.stage {
                    Stage::AwaitReply1 => "reply1",
                    Stage::AwaitReply2 => "reply2",
                    Stage::AwaitAccepted => "accepted",
                    Stage::Connected => "connected",
                };
                return Err(RakError::HandshakeTimeout(where_));
            }
            if last_resend.elapsed() >= Duration::from_millis(500) {
                last_resend = Instant::now();
                client.resend_current_request();
            }
            client.pump(Duration::from_millis(20)).await?;
        }

        debug!(
            "RakNet connecté à {} (guid={}, mtu={})",
            server_addr, client_guid, client.mtu
        );
        Ok(client)
    }

    /// (Ré)émet la requête offline de l'étape de handshake courante.
    fn resend_current_request(&mut self) {
        match self.stage {
            Stage::AwaitReply1 => {
                self.raw_out.push_back(encode_open_connection_request_1(
                    RAKNET_PROTOCOL_VERSION,
                    self.mtu,
                ));
            }
            Stage::AwaitReply2 => {
                self.raw_out.push_back(encode_open_connection_request_2(
                    &self.server_addr,
                    self.mtu,
                    self.client_guid,
                ));
            }
            // ConnectionRequest est reliable : la couche d'envoi le réémet.
            Stage::AwaitAccepted | Stage::Connected => {}
        }
    }

    /// Temps écoulé depuis le début de la session, en millisecondes.
    fn elapsed_ms(&self) -> i64 {
        self.start.elapsed().as_millis() as i64
    }

    /// Vrai si le serveur nous a déconnectés.
    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    /// Adresse locale (utile pour le logging / diagnostics).
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Envoie un batch de jeu (`0xFE …`) en reliable-ordered (canal 0), comme
    /// le fait un vrai client Bedrock.
    pub fn send_game_batch(&mut self, payload: Vec<u8>) {
        self.queue(payload, Reliability::ReliableOrdered);
    }

    /// Récupère le prochain batch de jeu reçu, s'il y en a un.
    pub fn next_incoming(&mut self) -> Option<Vec<u8>> {
        self.incoming.pop_front()
    }

    /// Une itération d'E/S : reçoit les paquets disponibles puis vide les files
    /// d'émission. `max_wait` borne l'attente s'il n'y a rien à recevoir.
    pub async fn pump(&mut self, max_wait: Duration) -> Result<(), RakError> {
        let mut batch: Vec<Vec<u8>> = Vec::new();
        let mut buf = [0u8; 2048];

        // Attente bornée du premier paquet (évite un busy-loop quand tout est calme).
        match timeout(max_wait, self.socket.recv(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => batch.push(buf[..n].to_vec()),
            Ok(Ok(_)) => {}
            // Erreur socket transitoire (ICMP port unreachable, etc.) : on
            // ignore, la boucle de handshake gère le timeout global.
            Ok(Err(e)) => trace!("recv err ignorée: {e}"),
            Err(_) => {} // timeout : rien reçu
        }

        // Draine le reste sans bloquer.
        loop {
            match self.socket.try_recv(&mut buf) {
                Ok(n) if n > 0 => batch.push(buf[..n].to_vec()),
                _ => break,
            }
        }

        for pkt in batch {
            self.handle_raw(&pkt);
        }

        self.maybe_ping();
        self.flush().await;
        Ok(())
    }

    /// Envoie un ConnectedPing périodique (garde la session vivante + mesure RTT).
    fn maybe_ping(&mut self) {
        if self.stage == Stage::Connected
            && self.last_ping.elapsed().as_secs_f64() > PING_INTERVAL_SECS
        {
            self.last_ping = Instant::now();
            let ping = encode_connected_ping(self.elapsed_ms());
            self.queue(ping, Reliability::Unreliable);
        }
    }

    /// Vide toutes les files d'émission sur le socket.
    async fn flush(&mut self) {
        // Paquets offline bruts (handshake).
        while let Some(raw) = self.raw_out.pop_front() {
            let _ = self.socket.send(&raw).await;
        }

        let now = self.start.elapsed().as_secs_f64();
        self.send.check_timeouts(now);
        self.send.flush_current_datagram();
        while let Some(dg) = self.send.next_datagram(now) {
            let _ = self.socket.send(&dg.encode()).await;
        }

        // Force l'avance de la fenêtre de réception (cf. bug de gel RakLib).
        self.recv.update();

        if !self.recv.ack_queue.is_empty() {
            let ack = encode_ack_nack(id::ACK, &mut self.recv.ack_queue);
            if !ack.is_empty() {
                let _ = self.socket.send(&ack).await;
            }
            self.recv.ack_queue.clear();
        }
        if !self.recv.nack_queue.is_empty() {
            let nack = encode_ack_nack(id::NACK, &mut self.recv.nack_queue);
            if !nack.is_empty() {
                let _ = self.socket.send(&nack).await;
            }
            self.recv.nack_queue.clear();
        }
    }

    /// Met un corps de paquet dans la file de fiabilité.
    fn queue(&mut self, body: Vec<u8>, reliability: Reliability) {
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
            body,
        };
        self.send.add_packet(pkt, true);
    }

    /// Traite un paquet UDP brut reçu du serveur.
    fn handle_raw(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            return;
        }
        self.last_recv = Instant::now();
        let id0 = buf[0];

        // ── Réponses offline du handshake ──
        match id0 {
            id::OPEN_CONNECTION_REPLY_1 => {
                if let Some((_guid, mtu)) = decode_open_connection_reply_1(buf) {
                    self.mtu = mtu.clamp(MIN_MTU_SIZE, MAX_MTU_SIZE);
                    let req2 = encode_open_connection_request_2(
                        &self.server_addr,
                        self.mtu,
                        self.client_guid,
                    );
                    self.raw_out.push_back(req2);
                    self.stage = Stage::AwaitReply2;
                }
                return;
            }
            id::OPEN_CONNECTION_REPLY_2 => {
                if let Some((_guid, mtu)) = decode_open_connection_reply_2(buf) {
                    self.mtu = mtu.clamp(MIN_MTU_SIZE, MAX_MTU_SIZE);
                    // (Re)crée la couche d'émission avec la MTU négociée.
                    self.send = SendLayer::new(self.mtu);
                    // ConnectionRequest (reliable-ordered).
                    let req = encode_connection_request(self.client_guid, self.elapsed_ms(), false);
                    self.queue(req, Reliability::ReliableOrdered);
                    self.stage = Stage::AwaitAccepted;
                }
                return;
            }
            id::INCOMPATIBLE_PROTOCOL_VERSION => {
                debug!("serveur: IncompatibleProtocolVersion");
                self.disconnected = true;
                return;
            }
            0x1C => {
                // UnconnectedPong : ignoré ici (utilisé seulement pour le MOTD).
                return;
            }
            _ => {}
        }

        // ── Trafic connecté : ACK / NACK / datagrammes ──
        if id::is_ack(id0) {
            self.send.on_ack(&decode_ack_records(buf));
        } else if id::is_nack(id0) {
            self.send.on_nack(&decode_ack_records(buf));
        } else if id::is_datagram(id0) {
            if let Some(dg) = Datagram::decode(buf) {
                let payloads = self.recv.on_datagram(dg.seq_number, dg.packets);
                for p in payloads {
                    self.handle_encapsulated(&p);
                }
            }
        }
    }

    /// Traite un paquet encapsulé (dé-fiabilisé, ré-ordonné).
    fn handle_encapsulated(&mut self, payload: &[u8]) {
        if payload.is_empty() {
            return;
        }
        let pid = payload[0];

        if pid >= id::USER_PACKET_ENUM {
            // Paquet de jeu (batch 0xFE) → couche protocole.
            self.incoming.push_back(payload.to_vec());
            return;
        }

        match pid {
            id::CONNECTION_REQUEST_ACCEPTED => {
                if let Some((request_time, pong_time)) = decode_connection_request_accepted(payload)
                {
                    let nic =
                        encode_new_incoming_connection(&self.server_addr, request_time, pong_time);
                    self.queue(nic, Reliability::ReliableOrdered);
                    self.stage = Stage::Connected;
                    self.last_ping = Instant::now();
                }
            }
            id::CONNECTED_PING => {
                if let Some(t) = decode_connected_ping(payload) {
                    let pong = encode_connected_pong(t, self.elapsed_ms());
                    self.queue(pong, Reliability::Unreliable);
                }
            }
            id::CONNECTED_PONG => {
                // pong[1..9] = notre ping_time d'origine.
                if payload.len() >= 9 {
                    let sent = i64::from_be_bytes(payload[1..9].try_into().unwrap());
                    let rtt = (self.elapsed_ms() - sent).max(0) as f64;
                    self.last_rtt_ms = Some(rtt);
                }
            }
            id::DISCONNECTION_NOTIFICATION => {
                debug!("serveur: DisconnectionNotification");
                self.disconnected = true;
            }
            other => {
                trace!("paquet interne non géré 0x{other:02X}");
            }
        }
    }

    /// Envoie une DisconnectionNotification et coupe proprement.
    pub async fn disconnect(&mut self) {
        let bye = encode_disconnection_notification();
        self.queue(bye, Reliability::ReliableOrdered);
        self.flush().await;
        self.disconnected = true;
    }
}
