# 11 - Dépendances Rust (Crates)

## Cargo.toml du workspace

```toml
[workspace]
resolver = "2"
members = [
    "crates/mc-rs-raknet",
    "crates/mc-rs-proto",
    "crates/mc-rs-nbt",
    "crates/mc-rs-crypto",
    "crates/mc-rs-world",
    "crates/mc-rs-game",
    "crates/mc-rs-command",
    "crates/mc-rs-plugin-api",
    "crates/mc-rs-plugin-wasm",
    "crates/mc-rs-plugin-lua",
    "crates/mc-rs-server",
]

[workspace.dependencies]
# Versions partagées entre tous les crates
tokio = { version = "1", features = ["rt-multi-thread", "net", "time", "sync", "io-util", "macros", "signal"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
thiserror = "2"
anyhow = "1"
bytes = "1"
uuid = { version = "1", features = ["v4", "serde"] }
rand = "0.8"
```

## Dépendances par catégorie

### Runtime async et réseau

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **tokio** | 1.x | Runtime async, UDP sockets, timers, channels | Standard de facto pour l'async Rust. Multi-threaded, mature |
| **bytes** | 1.x | Buffers efficaces (BytesMut, Buf, BufMut) | Zero-copy, optimal pour le parsing réseau |
| **socket2** | 0.5 | Options socket avancées (SO_REUSEADDR, buffer sizes) | Pour configurer finement les sockets UDP |
| **rak-rs** | latest | Implémentation RakNet pour Bedrock | Spécifiquement conçu pour Minecraft Bedrock, async |

**Alternative RakNet :** Implémenter soi-même si `rak-rs` ne suffit pas. Voir `rust-raknet` comme référence.

### ECS (Entity Component System)

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **bevy_ecs** | 0.15+ | ECS pour entités et systèmes de jeu | Leader en Rust, parallélisme auto, change detection, events |

**Pourquoi `bevy_ecs` et pas les autres :**

| Crate | Pour | Contre |
|-------|------|--------|
| `bevy_ecs` | Systèmes parallèles, change detection, events, actif | Plus lourd en dépendances |
| `hecs` | Minimal, rapide, API simple | Pas de scheduling auto, pas d'events intégrés |
| `legion` | Rapide, bon scheduling | Moins actif, API moins ergonomique |
| `specs` | Mature, flexible | API datée, maintenance mode |

### Sérialisation

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **serde** | 1.x | Framework de sérialisation | Universel, derive macros |
| **serde_json** | 1.x | JSON (resource packs, behavior packs, forms) | Standard Rust JSON |
| **toml** | 0.8 | Configuration serveur | Lisible, commentaires, standard Rust |
| **byteorder** | 1.x | Lecture/écriture endian-aware | Pour le protocole (LE/BE) |
| **integer-encoding** | 4.x | VarInt/VarLong LEB128 | Standard, léger |

**Pour NBT :** Implémenter un crate custom `mc-rs-nbt` car :
- Bedrock utilise LE-NBT (pas standard)
- La variante réseau utilise des VarInt
- ~300-500 lignes, contrôle total

### Stockage

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **rusty-leveldb** | 3.x | Lecture/écriture monde Bedrock (LevelDB) | Pure Rust, pas de dépendance C |

**Alternative :** `rust-rocksdb` si la compatibilité exacte avec les mondes BDS est critique (RocksDB lit les fichiers LevelDB).

**Attention :** Tester la compatibilité `rusty-leveldb` avec les mondes générés par BDS. La compression Snappy dans LevelDB doit être supportée.

### Cryptographie

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **jsonwebtoken** | 9.x | Décodage/validation JWT Xbox Live | Standard Rust, supporte ES384 |
| **p384** | 0.13 | ECDH key exchange (courbe P-384) | RustCrypto, pure Rust |
| **elliptic-curve** | 0.13 | Traits EC génériques | Base pour `p384` |
| **aes** | 0.8 | Chiffrement AES-256 (bloc) | RustCrypto, AES-NI si disponible |
| **cfb8** | 0.8 | Mode CFB-8 (Cipher Feedback 8-bit) | Spécifique à Bedrock |
| **sha2** | 0.10 | SHA-256 (dérivation de clé, checksums) | RustCrypto, standard |
| **base64** | 0.22 | Encodage Base64 (JWT, clés) | Standard |

**Stack crypto complète — pure Rust, pas de dépendance C (OpenSSL) :**

```toml
jsonwebtoken = "9"
p384 = { version = "0.13", features = ["ecdh", "ecdsa"] }
elliptic-curve = { version = "0.13", features = ["ecdh"] }
aes = "0.8"
cfb8 = "0.8"
sha2 = "0.10"
base64 = "0.22"
```

### Génération de monde

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **noise** | 0.9 | Bruit procédural (Perlin, Simplex, FBM) | API riche, bon pour le prototypage |
| **simdnoise** | 3.x | Bruit SIMD-accéléré | Performances pour la production |

**Stratégie :** Utiliser `noise` pour le développement, basculer vers `simdnoise` pour les hot paths en production.

### Concurrence

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **rayon** | 1.x | Parallélisme de données (chunk gen) | Work-stealing, parallélisme facile |
| **flume** | 0.11 | Channels sync/async | Fonctionne entre sync game thread et async network |
| **crossbeam-channel** | 0.5 | Channels haute performance (sync) | Si le game thread est purement sync |
| **dashmap** | 6.x | HashMap concurrente | Cache de chunks concurrent |
| **parking_lot** | 0.12 | Mutex/RwLock rapides | Remplacement du std mutex |

### Compression

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **flate2** | 1.x | Compression/décompression zlib | Standard, rapide |
| **snap** | 1.x | Compression/décompression Snappy | Pour le mode Snappy de Bedrock |

### Logging

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **tracing** | 0.1 | Logging structuré | Modern, spans async-aware |
| **tracing-subscriber** | 0.3 | Formatage et filtrage des logs | env-filter, fmt |
| **tracing-appender** | 0.2 | Écriture logs fichier avec rotation | Rotation automatique |

### Command parsing

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **nom** | 7.x | Parser combinateurs | Pour le parsing de commandes MC complexes |
| **clap** | 4.x | Arguments CLI du serveur | Standard pour les CLI Rust |

**Alternative à `nom` :** `winnow` (fork modernisé de nom) pour le parsing de commandes.

### Plugin system

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **wasmtime** | 27+ | Runtime WASM pour plugins sandboxés | Bytecode Alliance, production-ready |
| **mlua** | 0.10 | Scripting Lua (LuaJIT) | Async support, serde, sandboxing |

**Pourquoi `wasmtime` plutôt que `wasmer` :**
- Standards compliance (Component Model)
- Backing plus fort (Bytecode Alliance)
- Meilleure intégration epoch-based fuel metering

**Pourquoi `mlua` plutôt que `rlua` :**
- API plus moderne
- Support async (coroutines Lua ↔ Rust futures)
- LuaJIT support
- Meilleure gestion mémoire

### Utilitaires

| Crate | Version | Usage | Justification |
|-------|---------|-------|---------------|
| **uuid** | 1.x | UUIDs joueurs et entités | Standard, serde support |
| **rand** | 0.8 | Nombres aléatoires | Pour spawning, drops, world gen |
| **thiserror** | 2.x | Types d'erreur dérivés | Ergonomique, pas de boilerplate |
| **anyhow** | 1.x | Gestion d'erreur flexible | Pour la couche application |
| **bitflags** | 2.x | Bitflags typés | Pour entity flags, abilities, etc. |
| **indexmap** | 2.x | HashMap ordonnée | Pour les commandes, configs |
| **ahash** | 0.8 | Hash rapide | Remplace le hash par défaut de HashMap |
| **xxhash-rust** | 0.8 | Hash XXH64 | Pour le blob cache |

## Cargo.toml complet recommandé (crate serveur)

```toml
[package]
name = "mc-rs-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Workspace crates
mc-rs-raknet = { path = "../mc-rs-raknet" }
mc-rs-proto = { path = "../mc-rs-proto" }
mc-rs-nbt = { path = "../mc-rs-nbt" }
mc-rs-crypto = { path = "../mc-rs-crypto" }
mc-rs-world = { path = "../mc-rs-world" }
mc-rs-game = { path = "../mc-rs-game" }
mc-rs-command = { path = "../mc-rs-command" }
mc-rs-plugin-api = { path = "../mc-rs-plugin-api" }
mc-rs-plugin-wasm = { path = "../mc-rs-plugin-wasm" }
mc-rs-plugin-lua = { path = "../mc-rs-plugin-lua" }

# Async runtime
tokio = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }
toml = "0.8"

# Logging
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-appender = "0.2"

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# CLI
clap = { version = "4", features = ["derive"] }

# Concurrency
rayon = "1"
flume = "0.11"
dashmap = "6"
parking_lot = "0.12"

# Utils
uuid = { workspace = true }
rand = { workspace = true }
```

## Feature flags

```toml
[features]
default = ["lua-plugins", "wasm-plugins"]

# Activer le support des plugins Lua
lua-plugins = ["mc-rs-plugin-lua"]

# Activer le support des plugins WASM
wasm-plugins = ["mc-rs-plugin-wasm"]

# Mode développement (plus de logs, assertions)
dev = []

# Métriques Prometheus
metrics = ["dep:metrics", "dep:metrics-exporter-prometheus"]

# Support RCON (remote console)
rcon = []
```

## Versions minimales Rust

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
# Minimum Rust 1.75+ pour async trait stabilization
# Recommandé : dernière version stable
```

## Crates à surveiller

| Crate | Statut | Intérêt |
|-------|--------|---------|
| `bevy_ecs` | Actif, releases fréquentes | Vérifier les breaking changes à chaque release |
| `wasmtime` | Actif, releases majeures fréquentes | API peut changer entre versions majeures |
| `rak-rs` | Relativement jeune | Peut avoir des bugs, contribuer en upstream |
| `p384` | Stable, RustCrypto | Stable mais suivre les advisories |
| `rusty-leveldb` | Moins actif | Tester la compatibilité avec les mondes BDS |
