# 03 - Architecture du serveur

## Philosophie

L'architecture de MC-RS s'inspire des meilleures pratiques observées dans les serveurs existants :

- **Dragonfly (Go)** : Séparation propre des couches, un goroutine par monde
- **PocketMine-MP (PHP)** : Écosystème de plugins riche, event system mature
- **BDS (officiel)** : Référence pour la "bonne" implémentation du protocole

Principes directeurs :
1. **Séparation des couches** — Chaque couche (transport, protocole, jeu) est indépendante
2. **Message passing** — Communication inter-threads par channels, pas par état partagé
3. **ECS pour les entités** — Entity Component System pour la flexibilité et les performances
4. **Tick déterministe** — La boucle de jeu est séquentielle et prédictible
5. **Plugins sandboxés** — Les plugins tiers ne peuvent pas crasher le serveur

## Modèle de concurrence

```
┌─────────────────────────────────────────────────────────────┐
│                        Tokio Runtime                         │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  RakNet      │  │  RakNet      │  │  RakNet          │   │
│  │  Session 1   │  │  Session 2   │  │  Session N       │   │
│  │  (async task)│  │  (async task)│  │  (async task)    │   │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │                 │                    │             │
│         └────────────┬────┴────────────────────┘             │
│                      │ mpsc channel                          │
│                      ▼                                       │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                Network Manager                        │    │
│  │  Décode les paquets, route vers le bon monde          │    │
│  └──────────────────────┬───────────────────────────────┘    │
│                         │ mpsc channel                       │
│                         ▼                                    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │              Game Thread (tick loop)                   │    │
│  │                                                       │    │
│  │  ┌─────────────────────────────────────────────────┐  │    │
│  │  │  World: Overworld                                │  │    │
│  │  │  ├── ECS World (bevy_ecs)                        │  │    │
│  │  │  ├── Chunk Manager                               │  │    │
│  │  │  ├── Entity Systems                              │  │    │
│  │  │  └── Block Tick Scheduler                        │  │    │
│  │  └─────────────────────────────────────────────────┘  │    │
│  │  ┌─────────────────────────────────────────────────┐  │    │
│  │  │  World: Nether                                   │  │    │
│  │  └─────────────────────────────────────────────────┘  │    │
│  │  ┌─────────────────────────────────────────────────┐  │    │
│  │  │  World: The End                                  │  │    │
│  │  └─────────────────────────────────────────────────┘  │    │
│  └──────────────────────┬───────────────────────────────┘    │
│                         │ spawn_blocking / rayon              │
│                         ▼                                    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │              Worker Pool                              │    │
│  │  - Chunk generation (CPU-intensive)                   │    │
│  │  - World save (I/O)                                   │    │
│  │  - Lighting calculation                               │    │
│  │  - Plugin WASM execution                              │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Threads principaux

| Thread | Rôle | Technologie |
|--------|------|-------------|
| **Network Tasks** | Un task tokio par connexion RakNet. Gère la fiabilité, fragmentation, encryption, compression | `tokio::task` |
| **Network Manager** | Décode les paquets de jeu, les route vers le game thread | `tokio::task` |
| **Game Thread** | Boucle de tick principale (50ms/tick). Exécute toute la logique de jeu | Thread dédié ou `tokio::task` |
| **Worker Pool** | Tâches CPU-intensives : génération de chunks, sauvegarde monde | `rayon` / `spawn_blocking` |

### Communication inter-threads

```rust
// Types de channels utilisés

// Network → Game : paquets décodés des joueurs
mpsc::Sender<(PlayerId, GamePacket)>

// Game → Network : paquets à envoyer
mpsc::Sender<(PlayerId, GamePacket)>
// ou broadcast pour paquets envoyés à tous
broadcast::Sender<(ChunkPos, GamePacket)>

// Game → Workers : requêtes de génération
mpsc::Sender<ChunkGenRequest>

// Workers → Game : chunks générés
mpsc::Sender<ChunkGenResult>

// Shutdown signal
watch::Sender<bool>
```

## Boucle de tick

Le serveur tourne à **20 ticks par seconde** (50ms par tick) :

```rust
// Pseudo-code de la boucle de tick
let mut interval = tokio::time::interval(Duration::from_millis(50));
let mut current_tick: u64 = 0;

loop {
    interval.tick().await;
    let tick_start = Instant::now();

    // 1. Traiter les paquets entrants
    while let Ok(packet) = incoming_rx.try_recv() {
        handle_incoming_packet(packet);
    }

    // 2. Exécuter les systèmes ECS pour chaque monde
    for world in &mut worlds {
        world.tick(current_tick);
    }

    // 3. Traiter les tâches planifiées
    scheduled_tasks.process(current_tick);

    // 4. Envoyer les paquets sortants
    flush_outgoing_packets();

    // 5. Métriques
    let elapsed = tick_start.elapsed();
    if elapsed > Duration::from_millis(50) {
        warn!("Tick {} took {:?} (overrun!)", current_tick, elapsed);
    }

    current_tick += 1;
}
```

### Ordre de traitement dans un tick

1. **Paquets réseau entrants** — Mouvement, actions, chat
2. **Entity AI** — Pathfinding, comportements des mobs
3. **Physics** — Gravité, collisions, mouvement
4. **Block ticks** — Ticks aléatoires (crops), ticks planifiés (redstone, eau)
5. **Scheduled tasks** — Tâches différées (respawn, explosion)
6. **Chunk loading/unloading** — Charger/décharger selon les joueurs
7. **Entity spawning/despawning** — Spawn de mobs, despawn par distance
8. **Paquets sortants** — Envoyer les mises à jour aux joueurs

## Structure des crates

### `mc-rs-raknet`

```
mc-rs-raknet/
├── src/
│   ├── lib.rs
│   ├── socket.rs          # Socket UDP, bind, send/recv
│   ├── server.rs          # Accepteur de connexions RakNet
│   ├── session.rs         # Session RakNet individuelle
│   ├── reliability.rs     # Couche de fiabilité (ACK/NACK)
│   ├── ordering.rs        # Channels ordonnés (0-31)
│   ├── fragmentation.rs   # Découpage/réassemblage de gros paquets
│   ├── congestion.rs      # Contrôle de congestion
│   ├── offline.rs         # Paquets offline (ping/pong, handshake)
│   └── packet/
│       ├── mod.rs
│       ├── offline.rs     # UnconnectedPing, UnconnectedPong, etc.
│       ├── online.rs      # ConnectedPing, ConnectionRequest, etc.
│       └── frame.rs       # FrameSet, reliability types
```

### `mc-rs-proto`

```
mc-rs-proto/
├── src/
│   ├── lib.rs
│   ├── codec.rs           # Encode/decode du wrapper 0xFE
│   ├── compression.rs     # zlib, snappy
│   ├── encryption.rs      # AES-256-CFB8
│   ├── types.rs           # VarInt, Vec3, BlockPos, etc.
│   ├── batch.rs           # Batching de paquets
│   └── packets/
│       ├── mod.rs
│       ├── login.rs       # Login, PlayStatus, Handshake
│       ├── resource_pack.rs
│       ├── start_game.rs
│       ├── chunk.rs       # LevelChunk, SubChunk
│       ├── movement.rs    # MovePlayer, PlayerAuthInput
│       ├── entity.rs      # AddEntity, SetEntityData, etc.
│       ├── inventory.rs   # ItemStackRequest/Response
│       ├── world.rs       # UpdateBlock, BlockEvent
│       ├── command.rs     # AvailableCommands, CommandRequest
│       ├── form.rs        # ModalFormRequest/Response
│       ├── text.rs        # Text (chat)
│       └── ... (100+ types de paquets)
```

### `mc-rs-nbt`

```
mc-rs-nbt/
├── src/
│   ├── lib.rs
│   ├── tag.rs             # Types de tags NBT
│   ├── reader.rs          # Lecture NBT little-endian
│   ├── writer.rs          # Écriture NBT little-endian
│   ├── network.rs         # Variante réseau (VarInt)
│   └── serde.rs           # Intégration serde (Serialize/Deserialize)
```

### `mc-rs-crypto`

```
mc-rs-crypto/
├── src/
│   ├── lib.rs
│   ├── jwt.rs             # Parse/validation JWT Xbox Live
│   ├── ecdh.rs            # Échange de clés ECDH P-384
│   ├── aes.rs             # Chiffrement AES-256-CFB8
│   ├── key_exchange.rs    # Orchestration login encryption
│   └── xbox_auth.rs       # Validation chaîne Xbox Live
```

### `mc-rs-world`

```
mc-rs-world/
├── src/
│   ├── lib.rs
│   ├── chunk/
│   │   ├── mod.rs
│   │   ├── sub_chunk.rs   # SubChunk 16×16×16
│   │   ├── palette.rs     # Système de palette par sub-chunk
│   │   ├── column.rs      # Chunk column complète
│   │   └── section.rs     # Biome sections
│   ├── block/
│   │   ├── mod.rs
│   │   ├── state.rs       # Block states (name + properties)
│   │   ├── registry.rs    # Registre global des blocs
│   │   └── runtime_id.rs  # Mapping runtime IDs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── leveldb.rs     # Provider LevelDB
│   │   ├── keys.rs        # Format des clés LevelDB
│   │   └── provider.rs    # Trait WorldProvider
│   ├── generation/
│   │   ├── mod.rs
│   │   ├── overworld.rs   # Génération overworld
│   │   ├── nether.rs      # Génération nether
│   │   ├── end.rs         # Génération end
│   │   ├── flat.rs        # Monde plat
│   │   ├── void.rs        # Monde vide
│   │   ├── noise.rs       # Fonctions de bruit
│   │   ├── biome.rs       # Placement des biomes
│   │   └── structure/
│   │       ├── mod.rs
│   │       ├── village.rs
│   │       ├── temple.rs
│   │       ├── dungeon.rs
│   │       └── ...
│   └── level_dat.rs       # Lecture/écriture level.dat
```

### `mc-rs-game`

```
mc-rs-game/
├── src/
│   ├── lib.rs
│   ├── world.rs           # Monde de jeu (orchestre ECS + chunks)
│   ├── player/
│   │   ├── mod.rs
│   │   ├── handler.rs     # Traitement des paquets joueur
│   │   ├── movement.rs    # Validation mouvement server-auth
│   │   ├── inventory.rs   # Gestion inventaire
│   │   └── abilities.rs   # Capacités (vol, build, etc.)
│   ├── entity/
│   │   ├── mod.rs
│   │   ├── components.rs  # Composants ECS (Position, Health, etc.)
│   │   ├── systems.rs     # Systèmes ECS (physics, AI, etc.)
│   │   ├── ai/
│   │   │   ├── mod.rs
│   │   │   ├── pathfinding.rs
│   │   │   └── behaviors.rs
│   │   └── spawning.rs    # Règles de spawn
│   ├── block/
│   │   ├── mod.rs
│   │   ├── tick.rs        # Block ticks (random + scheduled)
│   │   ├── break.rs       # Cassage de blocs
│   │   ├── place.rs       # Placement de blocs
│   │   └── update.rs      # Propagation de mises à jour
│   ├── inventory/
│   │   ├── mod.rs
│   │   ├── container.rs   # Conteneurs (coffres, fours, etc.)
│   │   ├── crafting.rs    # Système de craft
│   │   ├── stack_request.rs # Traitement ItemStackRequest
│   │   └── creative.rs    # Inventaire créatif
│   ├── combat.rs          # Système de combat
│   ├── physics.rs         # Gravité, collisions
│   ├── scoreboard.rs      # Système de scoreboard
│   └── game_rules.rs      # Game rules
```

### `mc-rs-command`

```
mc-rs-command/
├── src/
│   ├── lib.rs
│   ├── tree.rs            # Arbre de commandes (type Brigadier)
│   ├── parser.rs          # Parsing des arguments
│   ├── registry.rs        # Registre des commandes
│   ├── selector.rs        # Entity selectors (@a, @p, @e, etc.)
│   ├── args/
│   │   ├── mod.rs
│   │   ├── position.rs    # Coordonnées (absolues, relatives ~)
│   │   ├── target.rs      # Sélecteurs d'entités
│   │   ├── item.rs        # Items
│   │   ├── block.rs       # Blocs
│   │   └── ...
│   └── builtin/
│       ├── mod.rs
│       ├── give.rs
│       ├── tp.rs
│       ├── gamemode.rs
│       ├── kill.rs
│       └── ...
```

### `mc-rs-plugin-api`

```
mc-rs-plugin-api/
├── src/
│   ├── lib.rs
│   ├── event.rs           # Système d'événements
│   ├── plugin.rs          # Trait Plugin
│   ├── api/
│   │   ├── mod.rs
│   │   ├── player.rs      # API joueur pour plugins
│   │   ├── world.rs       # API monde pour plugins
│   │   ├── entity.rs      # API entités pour plugins
│   │   ├── command.rs     # API commandes pour plugins
│   │   └── scheduler.rs   # API scheduler pour plugins
│   └── types.rs           # Types partagés
```

### `mc-rs-server`

```
mc-rs-server/
├── src/
│   ├── main.rs            # Point d'entrée
│   ├── config.rs          # Configuration (server.toml)
│   ├── server.rs          # Orchestrateur principal
│   ├── network.rs         # Couche réseau (lie RakNet + Proto)
│   ├── session.rs         # Session joueur (état de connexion)
│   └── console.rs         # Console serveur (REPL commandes)
```

## Patterns architecturaux

### Event System (pour plugins)

```rust
// Événement cancellable
pub struct PlayerBreakBlockEvent {
    pub player: PlayerId,
    pub position: BlockPos,
    pub block: BlockState,
    cancelled: bool,
}

impl Cancellable for PlayerBreakBlockEvent {
    fn is_cancelled(&self) -> bool { self.cancelled }
    fn set_cancelled(&mut self, val: bool) { self.cancelled = val; }
}

// Priorités d'écoute
pub enum EventPriority {
    Lowest,   // Exécuté en premier
    Low,
    Normal,
    High,
    Highest,
    Monitor,  // Exécuté en dernier, lecture seule
}
```

### Command Pattern (Brigadier-like)

```rust
// Déclaration de commande
cmd!("give")
    .description("Give items to a player")
    .permission("mc-rs.command.give")
    .then(
        arg("target", EntitySelector::players())
            .then(
                arg("item", ItemArg::new())
                    .then(
                        arg("amount", IntArg::range(1, 64))
                            .executes(give_command)
                    )
                    .executes(give_command_default_amount)
            )
    )
```

### Provider Pattern (stockage)

```rust
// Trait pour l'abstraction du stockage monde
pub trait WorldProvider: Send + Sync {
    fn load_chunk(&self, pos: ChunkPos) -> Option<ChunkColumn>;
    fn save_chunk(&self, pos: ChunkPos, chunk: &ChunkColumn) -> Result<()>;
    fn load_level_data(&self) -> Result<LevelData>;
    fn save_level_data(&self, data: &LevelData) -> Result<()>;
    fn load_player_data(&self, uuid: &Uuid) -> Option<PlayerData>;
    fn save_player_data(&self, uuid: &Uuid, data: &PlayerData) -> Result<()>;
}

// Implémentation LevelDB
pub struct LevelDbProvider { /* ... */ }
impl WorldProvider for LevelDbProvider { /* ... */ }
```
