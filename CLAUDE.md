# MC-RS — Minecraft Bedrock Edition Server in Rust

## Projet

Réécriture complète de PocketMine-MP (serveur Minecraft Bedrock PHP) en Rust.
Protocol version 975 (Minecraft Bedrock 1.26.20).

## Référence

Le code source de PocketMine-MP est dans `.reference/PocketMine-MP/`. C'est LA référence pour tout.
- Code serveur : `.reference/PocketMine-MP/src/`
- Protocole : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/`
- RakLib : `.reference/PocketMine-MP/vendor/pocketmine/raklib/src/`
- Données : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-data/`

**RÈGLE ABSOLUE** : Toujours vérifier le format des paquets dans le code PMMP avant d'implémenter. Ne jamais deviner un format binaire.

## Architecture

```
crates/
├── mc-rs-proto/      — Binary IO (ProtoReader/ProtoWriter), batch codec, tous les paquets
├── mc-rs-raknet/     — Transport UDP RakNet (sessions, reliability, ACK/NACK, split)
├── mc-rs-crypto/     — AES-256-CTR fakeGCM, ECDSA P-384, JWT
├── mc-rs-command/    — Registre de commandes, dispatch, actions
├── mc-rs-nbt/        — NBT (Named Binary Tag) — LE (disk) + Network (VarInt)
└── mc-rs-server/     — Exécutable principal
    ├── main.rs       — Boucle événementielle tokio, player registry, broadcasts
    ├── connection.rs — State machine (SessionStart→Login→Handshake→ResourcePacks→PreSpawn→SpawnResponse→InGame)
    ├── config.rs     — server.toml (motd, port, gamemode, difficulty, view_distance)
    ├── player_data.rs— Sauvegarde/chargement position joueur (JSON dans players/)
    ├── player_registry.rs — Tracking des joueurs connectés
    └── world/
        ├── chunk_cache.rs      — Cache mémoire + LevelDB persistence
        ├── chunk_serializer.rs — Sérialisation paletted storage (sub-chunks, biomes)
        ├── storage.rs          — LevelDB wrapper (rusty-leveldb)
        ├── terrain_generator.rs— Génération terrain Simplex 3D + biomes + ores + arbres
        ├── flat_generator.rs   — Génération flat world (block IDs)
        ├── tick.rs             — Cycle jour/nuit + météo
        ├── biome.rs            — 11 biomes
        ├── noise.rs            — Simplex noise
        ├── ore.rs              — Génération minerais
        ├── vegetation.rs       — Arbres + herbe
        └── random.rs           — RNG déterministe
```

## Code legacy (supprimé)

`old_crates/` et `docs_initial/` (première tentative du serveur + sa doc de conception) ont été
**supprimés du repo le 2026-05-28**. Ils polluaient la base de code et ne compilaient plus.

**NE JAMAIS les réutiliser ni s'en servir de référence d'implémentation.** Seule référence
canonique : PocketMine-MP dans `.reference/PocketMine-MP/`. Ce qu'ils contenaient est documenté
dans [`docs/ARCHIVE-legacy-code.md`](docs/ARCHIVE-legacy-code.md) ; le code reste récupérable
dans l'historique git (commit `788b429`) si jamais besoin de consulter.

## Builds et tests

```bash
cargo fmt && cargo clippy -- -D warnings && cargo build --release && cargo test
```

Pour lancer le serveur (le serveur écrit lui-même son log, PAS de redirection
shell) :
```bash
RUST_BACKTRACE=full RUST_LOG=info ./target/release/mc-rs-server.exe
```
- Logs runtime : `logs/server.<YYYY-MM-DD>.log` (rotation quotidienne).
- En cas de panic : `logs/CRASH-<timestamp>.log` (location + backtrace,
  écrit synchrone, survit au crash). `RUST_BACKTRACE=full` est requis pour
  que la backtrace ait les symboles.
- `.reference/server.log` est OBSOLÈTE — ne plus l'utiliser ni s'y fier.

Pour kill le serveur :
```bash
powershell -Command "Get-Process mc-rs-server -ErrorAction SilentlyContinue | Stop-Process -Force"
```

## État actuel

### Ce qui marche
- Connexion complète (RakNet, encryption Xbox Live, login)
- MOTD dans la liste de serveurs
- Terrain généré (Simplex 3D, 11 biomes, arbres, minerais, eau)
- Chat + 16 commandes avec tab-complete (/tp, /time, /weather, /gamemode, /stop, etc.)
- Multi-joueurs (PlayerList, AddPlayer, RemoveEntity)
- Cycle jour/nuit (20 minutes)
- Chunk loading dynamique au déplacement
- Player data persistence (position sauvegardée en JSON)
- World persistence (ChunkCache + LevelDB)
- Block breaking (UpdateBlock broadcast)
- SetActorData avec flags (BREATHING, HAS_GRAVITY, HAS_COLLISION)
- IA des mobs : framework générique inspiré d'Allay dans `crates/mc-rs-server/src/ai/`
  (sensors → behaviors priorisés → controllers, mémoire typée, navigation A\* au sol).
  Hostiles (zombie/skeleton/creeper) traquent le joueur via A\* (step-up/saut) et
  l'attaquent en mêlée (dégâts via `combat::attack_entity`) ; passifs errent et fuient
  quand blessés. Collision horizontale + step-up dans la physique des mobs. Référence =
  Allay (`.reference/Allay/.../entity/ai/`), car PMMP n'a pas d'IA de mobs.
  Skeleton-arc / Creeper-explosion = follow-ups non faits.

### Bugs connus
- **Gel per-connexion** : RÉSOLU (2026-05-19). Cause = fenêtre reliable RakNet
  wedgée par le paquet Login splitté (`recv.rs::handle_encapsulated` faisait
  `handle_split` avant l'enregistrement reliable). Détails et instrumentation
  conservée : `HANDOFF-FREEZE-BUG.md`.
- **Fly en survival** : probablement résolu par le fix `UPDATE_ABILITIES =
  0xBB` (l'ancien 0x12B était un ID fantôme). À reconfirmer si réapparition.

## Notes techniques importantes

- Les positions envoyées au client sont la position des **yeux** (pieds + 1.621), pas des pieds (PMMP Human.php getOffsetPosition)
- `blockNetworkIdsAreHashes=false` dans StartGame — les block IDs sont des indices séquentiels dans canonical_block_states.nbt
- TextPacket protocol 924 : `needsTranslation(bool) + category(u8) + type(u8)` (PAS type puis needsTranslation)
- CreativeContent : 2 counts (groups + items), pas 1
- SetActorData : PropertySyncData (2x VarUInt32) AVANT tick, pas après
- UpdateAbilities : bit 19 (VERTICAL_FLY_SPEED) DOIT être dans abilities_set
- BiomeDefinitionList protocol 924 : custom format (2x VarUInt32), PAS du NBT
- Le serveur tourne à 100 TPS (10ms/tick), le game tick à 20 TPS (1 game tick = 5 server ticks)

## Communication

L'utilisateur parle français. Il veut un serveur qui rivalise avec PocketMine-MP. Toujours se baser sur le code source PMMP pour implémenter. Ne jamais deviner, ne jamais prototyper — faire les choses proprement du premier coup.
