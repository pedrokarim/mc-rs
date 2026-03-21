# 03 - Protocol & Packets

## PocketMine : Système de paquets

### Structure d'un paquet MCPE

Tous les paquets de jeu sont envoyés en batch compressé :

```
[0xFE] [Compressed Data]
         └─ [Packet 1][Packet 2]...

Chaque paquet dans le batch :
  [VarUInt32 length] [Header] [Payload]

Header (premier VarUInt32 du paquet) :
  Bits 0-9  : Packet ID (10 bits, max 1023)
  Bits 10-11: Sender sub-client ID (2 bits)
  Bits 12-13: Target sub-client ID (2 bits)
```

### Protocole version 924

- **542 types de paquets** définis
- Version Minecraft : `1.26.0`
- Définis dans `ProtocolInfo.php`

### Types de sérialisation

| Type | Encodage |
|---|---|
| `u8` / `i8` | 1 octet |
| `u16_le` / `i16_le` | 2 octets little-endian |
| `u32_le` / `i32_le` | 4 octets little-endian |
| `u64_le` / `i64_le` | 8 octets little-endian |
| `u16_be` / `i16_be` | 2 octets big-endian |
| `u32_be` / `i32_be` | 4 octets big-endian |
| `f32_le` | 4 octets IEEE 754 LE |
| `VarUInt32` | Variable-length unsigned 32-bit |
| `VarInt32` | Variable-length signed 32-bit (zigzag) |
| `VarUInt64` | Variable-length unsigned 64-bit |
| `VarInt64` | Variable-length signed 64-bit (zigzag) |
| `String` | VarUInt32 length + UTF-8 bytes |
| `Bool` | 1 octet (0 ou 1) |
| `Vec3` | 3x f32_le (x, y, z) |
| `Vec2` | 2x f32_le (x, z) |
| `BlockPos` | VarInt32 x, VarUInt32 y, VarInt32 z |
| `UUID` | 2x i64_le (most, least) |
| `NBT` | Network Little-Endian NBT |

### Paquets clés (login flow)

| ID | Nom | Direction | Phase |
|---|---|---|---|
| 0xc1 | RequestNetworkSettings | C→S | SessionStart |
| 0x8f | NetworkSettings | S→C | SessionStart |
| 0x01 | Login | C→S | Login |
| 0x03 | ServerToClientHandshake | S→C | Login |
| 0x04 | ClientToServerHandshake | C→S | Handshake |
| 0x02 | PlayStatus | S→C | Multi-phase |
| 0x06 | ResourcePacksInfo | S→C | ResourcePacks |
| 0x07 | ResourcePackStack | S→C | ResourcePacks |
| 0x08 | ResourcePackClientResponse | C→S | ResourcePacks |
| 0x0b | StartGame | S→C | PreSpawn |
| 0x77 | BiomeDefinitionList | S→C | PreSpawn |
| 0x79 | AvailableActorIdentifiers | S→C | PreSpawn |
| 0x00a1 | ItemRegistry | S→C | PreSpawn |
| 0x34 | CraftingData | S→C | PreSpawn |
| 0x91 | CreativeContent | S→C | PreSpawn |
| 0x3f | PlayerList | S→C | PreSpawn |
| 0x3a | LevelChunk | S→C | PreSpawn |
| 0x45 | RequestChunkRadius | C→S | PreSpawn |
| 0x46 | ChunkRadiusUpdated | S→C | PreSpawn |

### Paquets clés (gameplay)

| ID | Nom | Direction | Description |
|---|---|---|---|
| 0x90 | PlayerAuthInput | C→S | Mouvement + inputs joueur |
| 0x13 | MovePlayer | S→C | Position joueur (broadcast) |
| 0x15 | MoveActorAbsolute | S→C | Position entité |
| 0x1c | UpdateBlock | S→C | Changement de bloc |
| 0x12 | AddPlayer | S→C | Spawn joueur |
| 0x0c | AddActor | S→C | Spawn entité |
| 0x0e | RemoveActor | S→C | Despawn entité |
| 0x27 | SetEntityData | S→C | Métadonnées entité |
| 0x1d | AddPainting | S→C | Spawn peinture |
| 0x36 | InventoryTransaction | C→S | Transactions inventaire |
| 0x31 | InventoryContent | S→C | Contenu inventaire |
| 0x32 | InventorySlot | S→C | Slot individuel |
| 0x93 | ItemStackRequest | C→S | Requête stack |
| 0x94 | ItemStackResponse | S→C | Réponse stack |
| 0x09 | Text | Bi | Messages chat |
| 0x4c | AvailableCommands | S→C | Commandes disponibles |
| 0x4d | CommandRequest | C→S | Exécution commande |
| 0x4e | CommandOutput | S→C | Résultat commande |
| 0x05 | Disconnect | S→C | Déconnexion |

### Pattern d'implémentation PocketMine

Chaque paquet est une classe avec :
- `NETWORK_ID` : constante
- `encodePayload(ByteBufferWriter)` : sérialisation
- `decodePayload(ByteBufferReader)` : désérialisation
- `handle(PacketHandlerInterface)` : dispatch vers handler

### Fichiers de référence

```
vendor/pocketmine/bedrock-protocol/src/ProtocolInfo.php      → IDs
vendor/pocketmine/bedrock-protocol/src/DataPacket.php        → Base class
vendor/pocketmine/bedrock-protocol/src/*.php                 → 542 paquets
vendor/pocketmine/bedrock-protocol/src/serializer/           → Types complexes
vendor/pocketmine/encoding/src/                              → VarInt, LE, BE
```

---

## Équivalent Rust

### Crate : `mc-rs-proto`

```rust
/// Trait commun à tous les paquets
pub trait Packet: Send + Sync {
    const ID: u32;
    fn encode(&self, buf: &mut BytesMut) -> Result<()>;
    fn decode(buf: &mut Bytes) -> Result<Self> where Self: Sized;
}

/// Direction du paquet
pub trait ClientboundPacket: Packet {}
pub trait ServerboundPacket: Packet {}

/// Types de sérialisation
pub mod types {
    pub type VarUInt32 = u32;  // encodé en variable-length
    pub type VarInt32 = i32;   // encodé en zigzag + variable-length
    pub type VarUInt64 = u64;
    pub type VarInt64 = i64;

    #[derive(Debug, Clone, Copy)]
    pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

    #[derive(Debug, Clone, Copy)]
    pub struct Vec2 { pub x: f32, pub z: f32 }

    #[derive(Debug, Clone, Copy)]
    pub struct BlockPos { pub x: i32, pub y: u32, pub z: i32 }
}

/// Lecture/écriture binaire
pub trait ProtoEncode {
    fn proto_encode(&self, buf: &mut BytesMut) -> Result<()>;
}

pub trait ProtoDecode: Sized {
    fn proto_decode(buf: &mut Bytes) -> Result<Self>;
}

/// Registre de paquets
pub struct PacketPool {
    decoders: HashMap<u32, fn(&mut Bytes) -> Result<Box<dyn Packet>>>,
}

impl PacketPool {
    pub fn new() -> Self;           // enregistre tous les 542 paquets
    pub fn decode(&self, id: u32, buf: &mut Bytes) -> Result<Box<dyn Packet>>;
}
```

### Exemple de paquet

```rust
#[derive(Debug, Clone)]
pub struct PlayStatusPacket {
    pub status: PlayStatus,
}

#[repr(i32)]
pub enum PlayStatus {
    LoginSuccess = 0,
    LoginFailedClient = 1,
    LoginFailedServer = 2,
    PlayerSpawn = 3,
    LoginFailedInvalidTenant = 4,
    LoginFailedVanillaEdu = 5,
    LoginFailedEduVanilla = 6,
    LoginFailedServerFull = 7,
    LoginFailedEditorVanilla = 8,
    LoginFailedVanillaEditor = 9,
}

impl Packet for PlayStatusPacket {
    const ID: u32 = 0x02;

    fn encode(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(self.status as i32); // BE int32
        Ok(())
    }

    fn decode(buf: &mut Bytes) -> Result<Self> {
        let status = buf.get_i32(); // BE int32
        Ok(Self { status: PlayStatus::try_from(status)? })
    }
}

impl ClientboundPacket for PlayStatusPacket {}
```

### Organisation des fichiers

```
mc-rs-proto/
├── src/
│   ├── lib.rs
│   ├── types.rs           → Types de base (VarInt, Vec3, etc.)
│   ├── codec.rs           → ProtoEncode/ProtoDecode impls
│   ├── packet.rs          → Trait Packet + PacketPool
│   ├── batch.rs           → Compression/décompression de batch
│   └── packets/
│       ├── mod.rs
│       ├── login.rs           → LoginPacket, PlayStatusPacket, etc.
│       ├── handshake.rs       → Handshake packets
│       ├── resource_packs.rs  → Resource pack packets
│       ├── world.rs           → StartGame, LevelChunk, etc.
│       ├── entity.rs          → AddActor, RemoveActor, etc.
│       ├── player.rs          → MovePlayer, PlayerAuthInput, etc.
│       ├── inventory.rs       → Inventory packets
│       ├── command.rs         → Command packets
│       └── misc.rs            → Autres paquets
```

### Macro pour réduire le boilerplate

```rust
/// Macro pour définir un paquet simplement
macro_rules! define_packet {
    ($name:ident, $id:expr, { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty),*
        }

        impl Packet for $name {
            const ID: u32 = $id;
            // encode/decode générés automatiquement
            // si les types implémentent ProtoEncode/ProtoDecode
        }
    };
}
```
