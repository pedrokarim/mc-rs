//! Simulation de clients non-conformes — robustesse du serveur.
//!
//! Envoie des entrées hostiles ou malformées (handshakes abandonnés, paquets
//! corrompus, floods, Login géant…) et vérifie après chaque attaque que le
//! serveur **reste vivant** (répond toujours à un UnconnectedPing normal).
//!
//! But : s'assurer que le serveur ne crashe pas et rejette proprement les
//! entrées hostiles. À utiliser uniquement contre **ton propre serveur**.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;
use tracing::debug;

use mc_rs_proto::codec;
use mc_rs_proto::io::ProtoWriter;
use mc_rs_proto::packets::packet_id;
use mc_rs_raknet::consts::RAKNET_PROTOCOL_VERSION;
use mc_rs_raknet::protocol::offline::*;

use crate::bedrock::{encode_player_auth_input, AuthInput};
use crate::net::RakClient;

/// Résultat d'un probe de robustesse.
#[derive(Debug, Clone)]
pub struct ProbeReport {
    pub name: String,
    pub detail: String,
    /// Le serveur répond-il encore après l'attaque ?
    pub server_alive: bool,
}

/// Ouvre un socket UDP connecté à la cible.
async fn connected_socket(target: SocketAddr) -> std::io::Result<UdpSocket> {
    let sock = UdpSocket::bind::<SocketAddr>("0.0.0.0:0".parse().unwrap()).await?;
    sock.connect(target).await?;
    Ok(sock)
}

/// Vérifie que le serveur répond à un UnconnectedPing (preuve de vie).
pub async fn server_responds(target: SocketAddr) -> bool {
    let Ok(sock) = connected_socket(target).await else {
        return false;
    };
    if sock
        .send(&encode_unconnected_ping(0, rand::random()))
        .await
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 2048];
    matches!(
        timeout(Duration::from_secs(2), sock.recv(&mut buf)).await,
        Ok(Ok(n)) if decode_unconnected_pong(&buf[..n]).is_some()
    )
}

/// Lance toute la suite de robustesse séquentiellement.
pub async fn run_suite(target: SocketAddr) -> Vec<ProbeReport> {
    let mut out = Vec::new();

    out.push(probe(target, "flood UnconnectedPing", flood_ping(target, 2000)).await);
    out.push(
        probe(
            target,
            "datagrammes UDP aléatoires",
            garbage_udp(target, 1000),
        )
        .await,
    );
    out.push(
        probe(
            target,
            "handshakes RakNet à moitié ouverts",
            half_open(target, 200),
        )
        .await,
    );
    out.push(
        probe(
            target,
            "batches de jeu corrompus",
            malformed_batches(target),
        )
        .await,
    );
    out.push(
        probe(
            target,
            "paquet valide, mauvais état",
            wrong_state_packet(target),
        )
        .await,
    );
    out.push(
        probe(
            target,
            "Login malformé + longueur géante",
            malformed_login(target),
        )
        .await,
    );

    out
}

/// Exécute `fut` (l'attaque) puis teste la survie du serveur.
async fn probe(
    target: SocketAddr,
    name: &str,
    fut: impl std::future::Future<Output = String>,
) -> ProbeReport {
    let detail = fut.await;
    // Laisse le serveur respirer, puis teste la survie (2 essais).
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut alive = server_responds(target).await;
    if !alive {
        tokio::time::sleep(Duration::from_millis(500)).await;
        alive = server_responds(target).await;
    }
    debug!("probe '{name}': {detail} (alive={alive})");
    ProbeReport {
        name: name.to_string(),
        detail,
        server_alive: alive,
    }
}

// ── Attaques ──

/// Envoie `n` UnconnectedPing aussi vite que possible.
async fn flood_ping(target: SocketAddr, n: usize) -> String {
    let Ok(sock) = connected_socket(target).await else {
        return "socket KO".into();
    };
    let mut sent = 0;
    for _ in 0..n {
        if sock
            .send(&encode_unconnected_ping(0, rand::random()))
            .await
            .is_ok()
        {
            sent += 1;
        }
    }
    // Compte les pongs reçus dans une courte fenêtre.
    let mut pongs = 0;
    let mut buf = [0u8; 2048];
    while let Ok(Ok(n)) = timeout(Duration::from_millis(300), sock.recv(&mut buf)).await {
        if decode_unconnected_pong(&buf[..n]).is_some() {
            pongs += 1;
        }
    }
    format!("{sent} pings envoyés, {pongs}+ pongs reçus")
}

/// Envoie `n` datagrammes UDP au contenu aléatoire (IDs invalides, tailles
/// variées).
async fn garbage_udp(target: SocketAddr, n: usize) -> String {
    let Ok(sock) = connected_socket(target).await else {
        return "socket KO".into();
    };
    let mut sent = 0;
    for i in 0..n {
        let len = 1 + (i * 7) % 1400;
        let mut pkt = vec![0u8; len];
        for (j, b) in pkt.iter_mut().enumerate() {
            *b = ((i.wrapping_mul(31).wrapping_add(j)) & 0xFF) as u8;
        }
        // Premier octet varié pour toucher tous les chemins de dispatch RakNet.
        pkt[0] = (i & 0xFF) as u8;
        if sock.send(&pkt).await.is_ok() {
            sent += 1;
        }
    }
    format!("{sent} datagrammes aléatoires envoyés")
}

/// Ouvre `n` connexions RakNet à moitié (OpenConnectionRequest1/2) sans jamais
/// les compléter, puis les abandonne.
async fn half_open(target: SocketAddr, n: usize) -> String {
    let mut opened = 0;
    for _ in 0..n {
        let Ok(sock) = connected_socket(target).await else {
            continue;
        };
        let guid: i64 = rand::random();
        // Req1 → (Reply1) → Req2, puis on laisse tomber.
        let _ = sock
            .send(&encode_open_connection_request_1(
                RAKNET_PROTOCOL_VERSION,
                1400,
            ))
            .await;
        let mut buf = [0u8; 2048];
        let _ = timeout(Duration::from_millis(50), sock.recv(&mut buf)).await;
        let _ = sock
            .send(&encode_open_connection_request_2(&target, 1400, guid))
            .await;
        opened += 1;
        // socket droppé → session serveur laissée à moitié ouverte.
    }
    format!("{opened} handshakes abandonnés après Request2")
}

/// Établit une session RakNet complète puis envoie des batches de jeu corrompus
/// (0xFE + octets aléatoires).
async fn malformed_batches(target: SocketAddr) -> String {
    let Ok(mut rak) = RakClient::open(target, 1400).await else {
        return "connexion RakNet KO".into();
    };
    let mut sent = 0;
    for i in 0..50u32 {
        let mut junk = vec![0xFE];
        junk.extend((0..(3 + i % 300)).map(|j| ((i.wrapping_mul(17) + j) & 0xFF) as u8));
        rak.send_game_batch(junk);
        let _ = rak.pump(Duration::from_millis(5)).await;
        sent += 1;
    }
    // Laisse partir les derniers datagrammes.
    for _ in 0..10 {
        let _ = rak.pump(Duration::from_millis(10)).await;
    }
    format!("{sent} batches corrompus envoyés sur session valide")
}

/// Envoie un paquet parfaitement valide mais dans le mauvais état (PlayerAuthInput
/// alors que le serveur attend RequestNetworkSettings).
async fn wrong_state_packet(target: SocketAddr) -> String {
    let Ok(mut rak) = RakClient::open(target, 1400).await else {
        return "connexion RakNet KO".into();
    };
    // PlayerAuthInput minimal mais bien formé (layout 2193, même encodeur que
    // les bots), cadré comme un batch SessionStart (sans octet d'algo, non
    // chiffré) : 0xFE + varint(len) + paquet.
    let payload = encode_player_auth_input(&AuthInput::default());
    let pkt = codec::encode_packet(packet_id::PLAYER_AUTH_INPUT, &payload);

    let mut batch = vec![0xFE];
    let mut inner = ProtoWriter::with_capacity(pkt.len() + 5);
    inner.write_var_u32(pkt.len() as u32);
    inner.write_raw(&pkt);
    batch.extend(inner.into_bytes());

    rak.send_game_batch(batch);
    for _ in 0..10 {
        let _ = rak.pump(Duration::from_millis(10)).await;
    }
    "PlayerAuthInput envoyé en état SessionStart".into()
}

/// Négocie NetworkSettings puis envoie un Login malformé : JSON invalide +
/// byte_array annonçant une taille énorme (test d'allocation/DoS).
async fn malformed_login(target: SocketAddr) -> String {
    let Ok(mut rak) = RakClient::open(target, 1400).await else {
        return "connexion RakNet KO".into();
    };

    // RequestNetworkSettings (batch SessionStart, sans octet d'algo).
    let mut proto = ProtoWriter::with_capacity(4);
    proto.write_i32_be(mc_rs_proto::CURRENT_PROTOCOL);
    let rns = codec::encode_packet(packet_id::REQUEST_NETWORK_SETTINGS, &proto.into_bytes());
    let mut batch = vec![0xFE];
    let mut inner = ProtoWriter::with_capacity(rns.len() + 5);
    inner.write_var_u32(rns.len() as u32);
    inner.write_raw(&rns);
    batch.extend(inner.into_bytes());
    rak.send_game_batch(batch);

    // Laisse le serveur passer en état Login.
    for _ in 0..10 {
        let _ = rak.pump(Duration::from_millis(20)).await;
    }

    // Login malformé : chain_data = JSON cassé, byte_array réclame 100 Mo mais
    // n'en fournit qu'une poignée.
    let mut w = ProtoWriter::with_capacity(64);
    w.write_i32_be(mc_rs_proto::CURRENT_PROTOCOL);
    w.write_var_u32(100_000_000); // taille byte_array annoncée = 100 Mo
    w.write_raw(b"{ not json at all >>>"); // beaucoup moins de données
    let login = codec::encode_packet(packet_id::LOGIN, &w.into_bytes());

    // Cadrage post-NetworkSettings : octet d'algo None (0xFF) + batch.
    let mut framed = vec![0xFE, 0xFF];
    let mut inner = ProtoWriter::with_capacity(login.len() + 5);
    inner.write_var_u32(login.len() as u32);
    inner.write_raw(&login);
    framed.extend(inner.into_bytes());
    rak.send_game_batch(framed);

    for _ in 0..10 {
        let _ = rak.pump(Duration::from_millis(20)).await;
    }
    "Login JSON cassé + byte_array 100 Mo envoyé".into()
}
