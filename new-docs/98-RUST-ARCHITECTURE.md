# 98 - Architecture Rust Cible

## Workspace Cargo

```
mc-rs/
├── Cargo.toml                    (workspace)
├── server.toml                   (config serveur)
├── crates/
│   ├── mc-rs-server/             → Exécutable principal + boucle de jeu
│   ├── mc-rs-proto/              → Protocole MCPE (paquets, sérialisation)
│   ├── mc-rs-raknet/             → Transport RakNet sur UDP
│   ├── mc-rs-network/            → Session réseau, state machine, encryption
│   ├── mc-rs-crypto/             → AES-256-CTR, ECDSA P-384, JWT
│   ├── mc-rs-nbt/                → Named Binary Tag (Network LE)
│   ├── mc-rs-world/              → Mondes, chunks, LevelDB, génération
│   ├── mc-rs-block/              → Blocs, états, registre, tiles
│   ├── mc-rs-entity/             → Entités, Living, Human, Player
│   ├── mc-rs-item/               → Items, durabilité, enchantements
│   ├── mc-rs-inventory/          → Inventaires, transactions
│   ├── mc-rs-crafting/           → Recettes, crafting grid
│   ├── mc-rs-command/            → Commandes, dispatch, autocomplete
│   ├── mc-rs-event/              → Système d'événements
│   ├── mc-rs-plugin-api/         → API publique pour les plugins
│   ├── mc-rs-plugin-lua/         → Loader de plugins Lua
│   └── mc-rs-game/               → Game logic (permissions, chat, etc.)
├── data/
│   ├── biome_definitions.nbt     → Blobs NBT statiques
│   ├── entity_identifiers.nbt
│   ├── canonical_block_states.nbt
│   ├── creative_items.json
│   └── recipes/                  → Recettes JSON
├── plugins/                      → Dossier plugins
├── worlds/                       → Données de monde (LevelDB)
└── .reference/                   → Code source PocketMine (gitignored)
```

## Dépendances entre crates

```
mc-rs-server
├── mc-rs-network
│   ├── mc-rs-proto
│   │   ├── mc-rs-nbt
│   │   └── mc-rs-crypto (JWT parsing)
│   ├── mc-rs-raknet
│   └── mc-rs-crypto (AES, ECDSA)
├── mc-rs-world
│   ├── mc-rs-nbt
│   └── mc-rs-block
├── mc-rs-entity
│   ├── mc-rs-item
│   └── mc-rs-inventory
├── mc-rs-game
│   ├── mc-rs-command
│   ├── mc-rs-crafting
│   └── mc-rs-event
├── mc-rs-plugin-api
│   └── mc-rs-event
└── mc-rs-plugin-lua
    └── mc-rs-plugin-api
```

## Dépendances externes (crates.io)

| Crate | Version | Usage |
|---|---|---|
| `tokio` | 1.x | Async runtime (UDP, timers, I/O) |
| `bytes` | 1.x | Buffer management |
| `tracing` | 0.1 | Logging structuré |
| `tracing-subscriber` | 0.3 | Log output |
| `flate2` | 1.x | Zlib compression |
| `snap` | 1.x | Snappy compression |
| `aes` | 0.8 | AES-256 |
| `ctr` | 0.9 | CTR mode |
| `p384` | 0.13 | ECDSA P-384 |
| `sha2` | 0.10 | SHA-256 |
| `jsonwebtoken` | 9.x | JWT decode/verify |
| `serde` | 1.x | Serialization |
| `serde_json` | 1.x | JSON |
| `toml` | 0.8 | Config parsing |
| `uuid` | 1.x | UUID |
| `rand` | 0.8 | RNG |
| `noise` | 0.9 | Perlin/Simplex noise |
| `rusty-leveldb` | 3.x | LevelDB |
| `mlua` | 0.9 | Lua 5.4 pour plugins |
| `dashmap` | 5.x | Concurrent HashMap |
| `parking_lot` | 0.12 | Mutex/RwLock rapides |
| `byteorder` | 1.x | Byte order helpers |
| `base64` | 0.22 | Base64 encode/decode |
| `num-traits` | 0.2 | Numeric traits |

## Architecture détaillée par crate

### mc-rs-server (exécutable)

```rust
// Point d'entrée
fn main() {
    // Init tracing
    // Charger config (server.toml)
    // Créer Server
    // server.run() → boucle principale
}

pub struct Server {
    config: ServerConfig,
    network: NetworkManager,
    worlds: WorldManager,
    plugins: PluginManager,
    commands: CommandMap,
    crafting: CraftingManager,
    events: EventManager,
    scheduler: Scheduler,
    async_pool: AsyncPool,
    running: AtomicBool,
    tick: u64,
    tps_tracker: TpsTracker,
}
```

### mc-rs-raknet

```rust
// Async UDP server avec tokio
pub struct RakNetServer {
    socket: Arc<UdpSocket>,           // tokio UDP
    sessions: DashMap<SocketAddr, RakNetSession>,
    server_guid: u64,
}

// Boucle de réception async
impl RakNetServer {
    pub async fn run(&self, tx: Sender<IncomingPacket>) {
        let mut buf = [0u8; 2048];
        loop {
            let (len, addr) = self.socket.recv_from(&mut buf).await?;
            self.handle_raw_packet(addr, &buf[..len], &tx).await;
        }
    }
}
```

### mc-rs-proto

```rust
// Chaque paquet : struct + impl Packet
// ~542 paquets au total
// Organisés par module (login, world, entity, inventory, etc.)
// Sérialisation manuelle (pas de derive, comme PocketMine)
```

### mc-rs-network

```rust
// NetworkSession = state machine
// Un handler par état
// Gère compression + encryption
pub struct NetworkSession {
    state: Box<dyn StateHandler>,
    cipher: Option<EncryptionContext>,
    compressor: Arc<dyn Compressor>,
    send_queue: VecDeque<Bytes>,
}
```

### mc-rs-world

```rust
// World + Chunk + SubChunk + PalettedStorage
// LevelDB provider
// Generators (flat, normal, nether)
// Chunk loading/unloading
```

### mc-rs-event

```rust
// TypeId-based event dispatch
// Priority ordering
// Cancellable events
// Thread-safe (main thread only, mais Send pour registration)
```

## Patterns Rust à utiliser

### 1. ECS-light (pas un vrai ECS)

On ne va PAS utiliser un framework ECS (bevy_ecs, hecs). On reste proche de l'architecture PocketMine avec des structs et traits, car :
- Plus simple à comprendre
- Correspondance directe avec le code PHP
- Moins de magie

### 2. Error handling

```rust
// Chaque crate a son type d'erreur
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("unexpected end of buffer")]
    BufferUnderflow,
    #[error("invalid packet id: {0}")]
    InvalidPacketId(u32),
    // ...
}
```

### 3. Interior mutability

```rust
// Pour les données partagées entre systèmes
// Arc<RwLock<T>> pour les grosses structures
// Arc<Mutex<T>> pour les petites structures hot-path
// Pas de Rc (tout doit être Send+Sync)
```

### 4. Builder pattern pour les paquets complexes

```rust
let packet = StartGamePacket::builder()
    .entity_id(1)
    .runtime_entity_id(1)
    .player_gamemode(GameMode::Survival)
    .spawn_position(Vec3::new(0.0, 64.0, 0.0))
    // ...
    .build();
```

### 5. Feature flags pour les composants optionnels

```toml
[features]
default = ["lua-plugins", "zlib"]
lua-plugins = ["mlua"]
wasm-plugins = ["wasmtime"]
snappy = ["snap"]
zlib = ["flate2"]
```
