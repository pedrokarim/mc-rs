# 04 - Couche réseau : RakNet

## Qu'est-ce que RakNet ?

RakNet est un protocole de transport fiable basé sur UDP, originellement créé par Jenkins Software (racheté par Oculus/Facebook). Minecraft Bedrock Edition utilise une version légèrement modifiée de RakNet.

RakNet fournit :
- **Fiabilité** — Accusés de réception (ACK/NACK), retransmission
- **Ordering** — Canaux ordonnés (0-31)
- **Fragmentation** — Découpage/réassemblage de paquets dépassant la MTU
- **Congestion control** — Contrôle du débit
- **Connection management** — Handshake, ping, déconnexion

## Magic bytes

Bedrock RakNet utilise une séquence magique de 16 octets dans les paquets offline :

```
00 FF FF 00 FE FE FE FE FD FD FD FD 12 34 56 78
```

Cette séquence est présente dans tous les paquets de handshake pour identifier le protocole.

## Paquets RakNet offline

### UnconnectedPing (0x01)

```
Packet ID    : 0x01 (1 octet)
TimeStamp    : int64 BE (8 octets)
Magic        : 16 octets
Client GUID  : int64 BE (8 octets)
```

### UnconnectedPong (0x1C)

```
Packet ID    : 0x1C (1 octet)
TimeStamp    : int64 BE (8 octets)
Server GUID  : int64 BE (8 octets)
Magic        : 16 octets
Data Length  : uint16 BE (2 octets)
MOTD String  : UTF-8 (format spécial, voir ci-dessous)
```

**Format MOTD (séparé par des `;`) :**
```
MCPE;Nom du serveur;protocole;version_jeu;joueurs_online;joueurs_max;server_guid;sous_nom;gamemode;gamemode_numeric;port_v4;port_v6;
```

Exemple :
```
MCPE;Mon Serveur MC-RS;924;1.26.0;0;20;12345678;MC-RS;Survival;1;19132;19133;
```

### OpenConnectionRequest1 (0x05)

```
Packet ID         : 0x05
Magic             : 16 octets
Protocol Version  : byte (actuellement 11)
MTU Padding       : Zéros pour remplir jusqu'à la MTU souhaitée
```

La taille totale de ce paquet indique la MTU que le client souhaite. Le serveur doit répondre avec une MTU ≤ celle demandée.

### OpenConnectionReply1 (0x06)

```
Packet ID    : 0x06
Magic        : 16 octets
Server GUID  : int64 BE
Use Security : bool (toujours false pour Bedrock)
MTU Size     : uint16 BE (MTU acceptée)
```

### OpenConnectionRequest2 (0x07)

```
Packet ID    : 0x07
Magic        : 16 octets
Server Addr  : RakNet Address (voir format ci-dessous)
MTU Size     : uint16 BE
Client GUID  : int64 BE
```

### OpenConnectionReply2 (0x08)

```
Packet ID     : 0x08
Magic         : 16 octets
Server GUID   : int64 BE
Client Addr   : RakNet Address
MTU Size      : uint16 BE
Use Encryption: bool (false)
```

### Format RakNet Address

```
Version : byte (4 = IPv4, 6 = IPv6)
[IPv4] :
  Address : 4 octets (inversés : ~byte pour chaque octet)
  Port    : uint16 BE
[IPv6] :
  Family     : uint16 LE
  Port       : uint16 BE
  Flow Info  : uint32 BE
  Address    : 16 octets
  Scope ID   : uint32 BE
```

## Paquets RakNet online (connecté)

### ConnectionRequest (0x09)

```
Packet ID     : 0x09
Client GUID   : int64 BE
TimeStamp     : int64 BE
Use Security  : bool
```

### ConnectionRequestAccepted (0x10)

```
Packet ID       : 0x10
Client Address  : RakNet Address
System Index    : uint16 BE
System Addresses: RakNet Address[20] (liste d'adresses système)
Request Time    : int64 BE
Accept Time     : int64 BE
```

### NewIncomingConnection (0x13)

```
Packet ID        : 0x13
Server Address   : RakNet Address
System Addresses : RakNet Address[20]
Request Time     : int64 BE
Accept Time      : int64 BE
```

### ConnectedPing (0x00)

```
Packet ID : 0x00
TimeStamp : int64 BE
```

### ConnectedPong (0x03)

```
Packet ID  : 0x03
Ping Time  : int64 BE
Pong Time  : int64 BE
```

### DisconnectionNotification (0x15)

```
Packet ID : 0x15
(pas de données supplémentaires)
```

## Couche de fiabilité — FrameSet

### Structure d'un FrameSet (0x80-0x8D)

```
Packet ID       : 0x80 à 0x8D (indique que c'est un FrameSet)
Sequence Number  : uint24_le (3 octets, numéro de séquence)

[Frames...] :     (une ou plusieurs frames empaquetées)
```

### Structure d'une Frame

```
Flags : byte
  Bits 5-7 : Reliability type (0-7)
  Bit 4    : Is split (fragmented)

[Selon reliability] :
  Reliable sequence number   : uint24_le (si reliable)
  Sequenced frame index      : uint24_le (si sequenced)
  Ordered frame index        : uint24_le (si ordered)
  Order channel              : byte (si ordered)

Length : uint16 BE (taille du body en BITS)

[Si is_split] :
  Split count  : uint32 BE (nombre total de fragments)
  Split ID     : uint16 BE (identifiant du paquet splitté)
  Split index  : uint32 BE (index de ce fragment)

Body : [length/8 octets]
```

### Types de fiabilité

| ID | Nom | Description |
|----|-----|-------------|
| 0 | Unreliable | Pas de garantie de livraison |
| 1 | UnreliableSequenced | Non fiable mais séquencé (les vieux sont ignorés) |
| 2 | Reliable | Garanti livré (avec retransmission) |
| 3 | ReliableOrdered | Fiable + ordonné (arrive dans l'ordre) |
| 4 | ReliableSequenced | Fiable + séquencé |
| 5 | UnreliableWithAckReceipt | Non fiable avec accusé |
| 6 | ReliableWithAckReceipt | Fiable avec accusé |
| 7 | ReliableOrderedWithAckReceipt | Fiable ordonné avec accusé |

**Bedrock utilise principalement** `ReliableOrdered` (3) pour les paquets de jeu.

### ACK (0xC0) et NACK (0xA0)

```
Packet ID   : 0xC0 (ACK) ou 0xA0 (NACK)
Record Count: uint16 BE

Pour chaque record :
  Is Range : bool
  [Si false] : Single sequence number (uint24_le)
  [Si true]  : Min (uint24_le), Max (uint24_le)
```

## Fragmentation

Quand un paquet dépasse la MTU (typiquement 1400 octets), il est fragmenté :

1. Le paquet est découpé en fragments de taille `MTU - overhead`
2. Chaque fragment est envoyé dans une frame avec `is_split = true`
3. Le récepteur réassemble quand il a reçu tous les fragments (`split_count` fragments)
4. Les fragments sont identifiés par `split_id` et ordonnés par `split_index`

**MTU typiques :** 576 (minimum), 1400 (typique), 1492 (max courant)

## Contrôle de congestion

RakNet utilise un algorithme de congestion simple :
- Fenêtre de congestion ajustée selon les ACK/NACK
- Backoff exponentiel sur perte de paquets
- La bande passante cible est estimée dynamiquement

## Game Packet Wrapper (0xFE)

Au-dessus de RakNet, tous les paquets de jeu Bedrock sont encapsulés :

```
Wrapper ID : 0xFE (1 octet)
[Contenu compressé/chiffré] :
  [Pour chaque sous-paquet dans le batch] :
    Packet Length : VarUInt
    Packet ID     : VarUInt
    Packet Data   : [length - sizeof(packet_id)] octets
```

### Pipeline de décodage

```
UDP recv
  → RakNet FrameSet parsing
    → Reliability/ordering/defragmentation
      → Frame body extraction
        → 0xFE wrapper detection
          → Decryption (AES-256-CFB8, si activé)
            → Decompression (zlib/snappy)
              → Batch splitting (VarInt length-prefixed)
                → Packet ID reading (VarUInt)
                  → Individual packet deserialization
```

### Pipeline d'encodage (inverse)

```
Individual packet serialization
  → Packet ID prepend (VarUInt)
    → Batch assembly (VarInt length-prefix each packet)
      → Compression (zlib/snappy)
        → Encryption (AES-256-CFB8, si activé)
          → 0xFE wrapper
            → RakNet framing (reliability, fragmentation if needed)
              → UDP send
```

## Compression

Négociée via `NetworkSettings` (envoyé en réponse à `RequestNetworkSettings`) :

```rust
// Algorithmes supportés
enum CompressionAlgorithm {
    Zlib = 0,      // deflate, le plus courant
    Snappy = 1,    // plus rapide, moins compressif
    None = 0xFFFF, // pas de compression
}
```

- **Seuil de compression** : Les paquets en dessous d'une certaine taille ne sont pas compressés
- **Zlib** : Meilleure compression, plus lent. Bon pour les serveurs avec beaucoup de joueurs
- **Snappy** : Plus rapide, moins de compression. Bon pour les LAN ou les serveurs puissants
- La compression est activée **après** `NetworkSettings` et **avant** le chiffrement dans le pipeline

## Chiffrement

Voir [09-security.md](09-security.md) pour les détails complets.

En bref :
1. Après `LoginPacket`, le serveur envoie `ServerToClientHandshake` avec sa clé publique ECDH P-384
2. Les deux côtés dérivent un secret partagé via ECDH
3. Une clé AES-256 est dérivée via SHA-256 du secret partagé
4. Tous les paquets suivants sont chiffrés en AES-256-CFB8 avec un compteur/checksum SHA-256

## Implémentation Rust recommandée

### Option 1 : Utiliser `rak-rs`

```toml
[dependencies]
rak-rs = "*"  # Vérifier la dernière version
```

`rak-rs` est spécifiquement conçu pour Minecraft Bedrock :
- Gère le MAGIC Bedrock
- Format MOTD de l'UnconnectedPong
- Async avec tokio
- Fiabilité, ordering, fragmentation

**Avantages :** Gain de temps considérable
**Inconvénients :** Peut avoir des edge cases, moins de contrôle

### Option 2 : Implémentation custom

Si `rak-rs` ne suffit pas, implémenter RakNet soi-même :

```rust
// Structure de base
pub struct RakNetServer {
    socket: UdpSocket,
    sessions: HashMap<SocketAddr, RakNetSession>,
    server_guid: u64,
    motd: ServerMotd,
}

pub struct RakNetSession {
    addr: SocketAddr,
    state: SessionState,
    mtu: u16,
    send_sequence: u24,
    recv_sequence: u24,
    send_reliable_index: u24,
    recv_reliable_tracking: BitSet,
    order_channels: [OrderChannel; 32],
    split_packets: HashMap<u16, SplitAssembler>,
    send_queue: Vec<Frame>,
    ack_queue: Vec<u24>,
    nack_queue: Vec<u24>,
}
```

**Ordre d'implémentation :**
1. Socket UDP + ping/pong offline
2. Handshake (OpenConnectionRequest/Reply 1 & 2)
3. Paquets online (ConnectionRequest/Accepted)
4. FrameSet basique (sans fragmentation)
5. Fiabilité (ACK/NACK, retransmission)
6. Ordering channels
7. Fragmentation
8. Contrôle de congestion

## Gestion des connexions

```
États d'une session RakNet :

[Disconnected]
    │
    ▼ (recv OpenConnectionRequest1)
[Connecting]
    │
    ▼ (recv OpenConnectionRequest2)
[HandshakeCompleted]
    │
    ▼ (recv ConnectionRequest)
[ConnectionPending]
    │
    ▼ (recv NewIncomingConnection)
[Connected]
    │
    ▼ (recv DisconnectionNotification / timeout)
[Disconnected]
```

## Timeouts et keep-alive

- **Ping/Pong interval :** ~5 secondes
- **Timeout :** Si aucun paquet reçu pendant ~10 secondes → déconnexion
- **Stale connection cleanup :** Vérifier périodiquement les sessions inactives
