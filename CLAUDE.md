# MC-RS — Minecraft Bedrock Edition Server in Rust

## Projet

Réécriture complète de PocketMine-MP (serveur Minecraft Bedrock PHP) en Rust.
Protocol version 2169 (Minecraft Bedrock 1.26.45). Source unique : `CURRENT_PROTOCOL` /
`MINECRAFT_VERSION` dans `crates/mc-rs-proto/src/lib.rs` (ne jamais recopier la valeur ailleurs).

## Référence

Le code source de PocketMine-MP est dans `.reference/PocketMine-MP/`. Il reste LA référence pour la logique
serveur (handlers, inventaires, gameplay).
- Code serveur : `.reference/PocketMine-MP/src/`
- Protocole : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/`
- RakLib : `.reference/PocketMine-MP/vendor/pocketmine/raklib/src/`
- Données : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-data/`

**Fin de support de PocketMine-MP (2026-07-09)** : PMMP et BedrockData s'arrêtent au protocole 1001
(1.26.30). Pour les formats au-delà, les références sont, croisées entre elles : changelogs et schémas JSON
Mojang (`.reference/bedrock-protocol-docs`, tag de la release), gophertunnel (`git diff` entre tags ;
le dépôt local a des modifications personnelles, ne pas y faire de pull) et CloudburstProtocol
(`bedrock-codec/.../codec/vNNNN/`). Données de jeu : dragonfly. Détails et procédure :
[`dev-docs/31-PROTOCOL-2169.md`](dev-docs/31-PROTOCOL-2169.md).

**RÈGLE ABSOLUE** : Toujours vérifier le format des paquets dans les références avant d'implémenter (PMMP
jusqu'à 1001, puis Mojang + gophertunnel + CloudburstProtocol). Ne jamais deviner un format binaire.

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

## Organisation du dépôt

- `crates/` : le code (voir ci-dessus) ; `plugins/` et `resource_packs/` : exemples et packs livrés.
- `dev-docs/` : documentation interne (spécifications, audits, suivis), index dans
  [`dev-docs/README.md`](dev-docs/README.md) ; `dev-docs/archive/` pour les documents périmés.
- `docs/` : **site public** GitHub Pages (publié depuis `main:/docs`). N'y mettre que des pages du site.
- `scripts/` : scripts Python d'import de données et outils de dev (`scripts/dev/`), voir leur README.
- `benchmarks/rust/` (crate `benchmark` du workspace) et `benchmarks/compare/` (comparaison mc-rs,
  PMMP et BDS en Python).
- `menu_forge/` : menus Menu Forge servis par le moteur `crates/mc-rs-server/src/menu_forge/`,
  espace de travail du studio (`workspace/`) et son export Bedrock (`export/`), voir
  [`dev-docs/32-MENU-FORGE.md`](dev-docs/32-MENU-FORGE.md). Dossier de contenu au même titre que
  `plugins/` et `resource_packs/` (il ne peut pas aller dans ce dernier : chaque dossier y est chargé
  comme un pack).
- Rien de nouveau à la racine : un script va dans `scripts/`, une note dans `dev-docs/`. Les fichiers
  runtime (`server-state.json`, `webui.db`, `worlds/`, `players/`, `logs/`) sont ignorés par git.

## Code legacy (supprimé)

`old_crates/` et `docs_initial/` (première tentative du serveur + sa doc de conception) ont été
**supprimés du repo le 2026-05-28**. Ils polluaient la base de code et ne compilaient plus.

**NE JAMAIS les réutiliser ni s'en servir de référence d'implémentation.** Seule référence
canonique : PocketMine-MP dans `.reference/PocketMine-MP/`. Ce qu'ils contenaient est documenté
dans [`dev-docs/archive/ARCHIVE-legacy-code.md`](dev-docs/archive/ARCHIVE-legacy-code.md) ; le code reste récupérable
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
  Hostiles traquent le joueur via A\* (step-up/saut) : **zombie** mêlée, **skeleton** tir à
  l'arc (entité flèche `arrow_entity.rs`, kite + visée balistique), **creeper** explosion
  (`explosion.rs` : destruction de blocs + dégâts radiaux). Passifs errent et fuient quand
  blessés. Collision horizontale + step-up dans la physique des mobs. Réf = Allay
  (`.reference/Allay/.../entity/ai/`), car PMMP n'a pas d'IA de mobs. **Doc : `dev-docs/18-MOB-AI.md`**.
  Follow-ups restants : son/particule (explosion/flèche), dégâts par difficulté, lissage de chemin.

### Bugs connus
- **Gel per-connexion** : RÉSOLU (2026-05-19). Cause = fenêtre reliable RakNet
  wedgée par le paquet Login splitté (`recv.rs::handle_encapsulated` faisait
  `handle_split` avant l'enregistrement reliable). Détails et instrumentation
  conservée : `dev-docs/archive/HANDOFF-FREEZE-BUG.md`.
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
- Données 1.26.40 (palette, items, créatif) : dragonfly v0.11.4, dans `crates/mc-rs-server/data/`.
  Les sous-chunks sont sauvegardés avec des runtime IDs bruts : tout changement de palette exige de relancer
  `generate_block_registry` puis `generate_data_migrations` (migration appliquée à l'ouverture du monde,
  marqueur LevelDB `mcrs:palette_fingerprint`)
- Protocole 2168+ (« Cereal ») : optionnels précédés d'un booléen, parfois « doubles » (booléen externe
  toujours vrai + présence), variantes taguées par un index VarUInt32
- **Le vrai client 1.26.45 vérifie les contraintes du schéma Mojang, gophertunnel non.** Un bot qui se
  connecte ne prouve rien. Valeur d'énumération hors liste (ex. `BuildPlatform = 0`), `Json::Value` qui
  n'est pas du JSON (ex. géométrie de skin `""`), taille d'image incohérente : écran « Block » (code 90,
  `BadPacket`), sans aucun journal côté client. Pour chaque champ envoyé, lire le JSON Mojang
  (`enum`, `x-runtime-constraint-description`, `minimum`/`maximum`) et le comparer à dragonfly.
  Méthode en cas de « Block » : bisection avec `MCRS_EMPTY_PRESPAWN`, puis comparaison champ par champ.
  Causes déjà trouvées (biomes 4096 cases, BiomeDefinitionList vide, skin, pack) :
  [`dev-docs/31-PROTOCOL-2169.md`](dev-docs/31-PROTOCOL-2169.md) § 5

## Communication

L'utilisateur parle français. Il veut un serveur qui rivalise avec PocketMine-MP. Toujours se baser sur le code source PMMP pour implémenter. Ne jamais deviner, ne jamais prototyper — faire les choses proprement du premier coup.
