# 02 - Network & RakNet

## PocketMine : Architecture réseau

### Vue d'ensemble

```
Client (Bedrock) ←→ UDP Socket ←→ RakNet (RakLib) ←→ MCPE Protocol ←→ NetworkSession ←→ Game Logic
```

### Couche 1 : UDP Socket

- Socket UDP sur port 19132 (par défaut)
- Un seul socket pour tous les clients
- Paquets identifiés par adresse IP:port source

### Couche 2 : RakNet (RakLib)

RakNet est le protocole de transport fiable au-dessus d'UDP.

**Constantes :**
- `RAKNET_PROTOCOL_VERSION = 11`
- `MCPE_RAKNET_PACKET_ID = 0xFE` (identifie les paquets de jeu)
- `MAGIC = [0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78]`

**Types de paquets RakNet :**

| ID | Paquet | Direction |
|---|---|---|
| 0x01 | UnconnectedPing | C→S |
| 0x02 | UnconnectedPingOpenConnections | C→S |
| 0x05 | OpenConnectionRequest1 | C→S |
| 0x06 | OpenConnectionReply1 | S→C |
| 0x07 | OpenConnectionRequest2 | C→S |
| 0x08 | OpenConnectionReply2 | S→C |
| 0x09 | ConnectionRequest | C→S |
| 0x10 | ConnectionRequestAccepted | S→C |
| 0x13 | NewIncomingConnection | C→S |
| 0x15 | DisconnectionNotification | Bi |
| 0x19 | IncompatibleProtocolVersion | S→C |
| 0x1c | UnconnectedPong | S→C |
| 0x00 | ConnectedPing | Bi |
| 0x03 | ConnectedPong | Bi |
| 0x80-0x8f | Datagram (game data) | Bi |
| 0xc0 | ACK | Bi |
| 0xa0 | NACK | Bi |

### Connexion RakNet (handshake)

```
Client                          Server
  |                               |
  |--- UnconnectedPing ---------->|
  |<-- UnconnectedPong -----------|
  |                               |
  |--- OpenConnectionRequest1 --->|  (MTU discovery, protocol version)
  |<-- OpenConnectionReply1 ------|  (server GUID, MTU)
  |                               |
  |--- OpenConnectionRequest2 --->|  (client address, MTU, GUID)
  |<-- OpenConnectionReply2 ------|  (server address, MTU, encryption)
  |                               |
  |=== RakNet Connected ===========|
  |                               |
  |--- ConnectionRequest -------->|  (client GUID, timestamp)
  |<-- ConnectionRequestAccepted -|  (client address, timestamps)
  |                               |
  |--- NewIncomingConnection ---->|  (server address, timestamps)
  |                               |
  |=== Session Established ========|
  |                               |
  |--- GamePacket (0xFE) -------->|  (MCPE packets inside)
  |<-- GamePacket (0xFE) ---------|
```

### Fiabilité RakNet

Chaque paquet encapsulé a un niveau de fiabilité :

| Valeur | Mode | Description |
|---|---|---|
| 0 | Unreliable | Pas de garantie |
| 1 | UnreliableSequenced | Séquencé, pas d'ACK |
| 2 | Reliable | Garanti livré |
| 3 | ReliableOrdered | Garanti + ordonné |
| 4 | ReliableSequenced | Garanti + séquencé |

**Datagrams** contiennent des `EncapsulatedPacket` :
- Header : reliability (3 bits), has split (1 bit)
- Message index (24-bit LE) si reliable
- Sequence index (24-bit LE) si sequenced
- Order index + order channel si ordered
- Split info si fragmenté : count, id, index

**ACK/NACK :**
- ACK confirme la réception de datagrams
- NACK signale des datagrams manquants → retransmission
- Format : ranges de sequence numbers (compact)

### Couche 3 : MCPE Protocol

Les paquets de jeu sont encapsulés dans des datagrams RakNet avec ID `0xFE` :

```
[0xFE] [Compressed Payload]
         └─ [Packet1] [Packet2] ... (batch)

Chaque Packet :
  [VarUInt32 length] [Packet ID (VarUInt32)] [Payload bytes]
```

**Compression :** Zlib (deflate) ou Snappy, configurable.
**Encryption :** AES-256-CTR après handshake (voir 04-LOGIN-FLOW.md).

### Couche 4 : NetworkSession

Chaque client connecté a une `NetworkSession` qui gère :
- L'état de la connexion (state machine)
- L'envoi/réception de paquets
- Le rate limiting
- La compression
- L'encryption
- Le dispatch vers le handler approprié

**Rate limiting :**

| Paramètre | Valeur |
|---|---|
| Batch par tick | 2 |
| Buffer ticks (batch) | 100 |
| Game packets par tick | 2 |
| Buffer ticks (game) | 100 |
| Hard limit batch | 300 |

### Fichiers PocketMine de référence

```
src/network/mcpe/NetworkSession.php          → Session principale
src/network/mcpe/raklib/RakLibInterface.php  → Interface RakLib
vendor/pocketmine/raklib/src/protocol/       → Paquets RakNet
vendor/pocketmine/raklib/src/generic/        → Session, reliability layers
vendor/pocketmine/raklib/src/server/         → Serveur RakLib
```

---

## Équivalent Rust

### Crate : `mc-rs-raknet`

```rust
/// Couche RakNet - transport fiable sur UDP
pub struct RakNetServer {
    socket: UdpSocket,
    sessions: HashMap<SocketAddr, RakNetSession>,
    server_guid: u64,
    motd: String,
    max_connections: usize,
}

pub struct RakNetSession {
    address: SocketAddr,
    state: SessionState,
    mtu: u16,
    client_guid: u64,
    // Reliability
    send_sequence: u24,
    recv_sequence: u24,
    send_reliable_index: u24,
    recv_reliable_index: u24,
    send_order_index: [u24; 32],  // par channel
    recv_order_index: [u24; 32],
    // Buffers
    send_queue: VecDeque<EncapsulatedPacket>,
    ack_queue: Vec<u24>,
    nack_queue: Vec<u24>,
    resend_queue: BTreeMap<u24, Datagram>,
    // Split reassembly
    split_packets: HashMap<u16, SplitPacketAssembly>,
    // Ordering
    order_queues: [BTreeMap<u24, EncapsulatedPacket>; 32],
}

#[derive(Debug, Clone, Copy)]
pub enum SessionState {
    Unconnected,
    Connecting,    // OpenConnectionRequest reçu
    Connected,     // ConnectionRequest accepté
    Disconnecting,
    Disconnected,
}

pub struct EncapsulatedPacket {
    pub reliability: Reliability,
    pub message_index: Option<u24>,
    pub sequence_index: Option<u24>,
    pub order_index: Option<u24>,
    pub order_channel: Option<u8>,
    pub split: Option<SplitInfo>,
    pub body: Bytes,
}

pub struct SplitInfo {
    pub count: u32,
    pub id: u16,
    pub index: u32,
}

#[repr(u8)]
pub enum Reliability {
    Unreliable = 0,
    UnreliableSequenced = 1,
    Reliable = 2,
    ReliableOrdered = 3,
    ReliableSequenced = 4,
}
```

### Crate : `mc-rs-network`

```rust
/// Couche MCPE au-dessus de RakNet
pub struct NetworkManager {
    raknet: RakNetServer,
    sessions: HashMap<SocketAddr, NetworkSession>,
    compressor: Box<dyn Compressor>,
}

pub struct NetworkSession {
    address: SocketAddr,
    state: ConnectionState,
    handler: Box<dyn PacketHandler>,
    cipher: Option<EncryptionContext>,
    compressor: Arc<dyn Compressor>,
    player: Option<PlayerHandle>,
    // Rate limiting
    batch_budget: PacketBudget,
    game_packet_budget: PacketBudget,
}

pub enum ConnectionState {
    SessionStart,
    Login,
    Handshake,
    ResourcePacks,
    PreSpawn,
    SpawnResponse,
    InGame,
    Death,
}

pub trait PacketHandler: Send {
    fn handle_packet(&mut self, session: &mut NetworkSession, packet: &dyn Packet) -> Result<()>;
}

pub trait Compressor: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
}
```

### Dépendances Rust recommandées

| Crate | Usage |
|---|---|
| `tokio` | Async runtime (UDP socket, timers) |
| `bytes` | Buffer management efficace |
| `flate2` | Zlib compression |
| `snap` | Snappy compression |
| `aes` + `ctr` | AES-256-CTR encryption |
| `p384` | ECDSA P-384 (key exchange) |
| `sha2` | SHA-256 (checksums) |
| `jsonwebtoken` | JWT parsing (login) |
