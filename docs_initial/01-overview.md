# 01 - Vue d'ensemble du projet MC-RS

## Qu'est-ce que MC-RS ?

MC-RS est un logiciel serveur pour **Minecraft Bedrock Edition** (anciennement Minecraft Pocket Edition / Windows 10 Edition), écrit intégralement en **Rust**. L'objectif est de créer un serveur haute performance, modulaire et extensible, capable d'accueillir un grand nombre de joueurs tout en offrant un système de plugins robuste.

## Minecraft Bedrock Edition — Contexte

Minecraft Bedrock Edition est la version unifiée de Minecraft disponible sur :
- **Windows 10/11** (via Microsoft Store)
- **iOS / Android** (mobile)
- **Xbox One / Series X|S**
- **PlayStation 4/5**
- **Nintendo Switch**

Contrairement à l'édition Java (qui utilise TCP et un protocole big-endian), Bedrock utilise :
- **RakNet** comme protocole de transport (UDP)
- **Little-endian** pour la sérialisation NBT et la plupart des données
- Un système de **mouvement server-authoritative** (le serveur contrôle la position des joueurs)
- Un système d'**inventaire server-authoritative** (le serveur valide toutes les transactions)
- Un système d'**addons** natif (behavior packs + resource packs en JSON)
- Le format de monde **LevelDB** (au lieu du format Anvil/Region de Java)

## Pourquoi Rust ?

| Critère | Avantage Rust |
|---------|---------------|
| **Performance** | Comparable au C/C++, sans garbage collector. Idéal pour un serveur de jeu 20 TPS |
| **Sécurité mémoire** | Pas de segfault, data races détectées à la compilation |
| **Concurrence** | `tokio` pour l'async, `rayon` pour le parallélisme de données |
| **Écosystème** | Crates matures pour crypto, sérialisation, ECS, WASM |
| **Fiabilité** | Le système de types empêche des classes entières de bugs |
| **Interopérabilité** | FFI facile vers C pour LevelDB, scripting engines, etc. |

## Serveurs Bedrock existants — Positionnement

| Serveur | Langage | Forces | Faiblesses |
|---------|---------|--------|------------|
| **BDS** (officiel Mojang) | C++ (fermé) | Parité vanilla 100% | Fermé, non modifiable, ressources lourdes |
| **PocketMine-MP** | PHP 8.1+ | Mature, gros écosystème de plugins | Perf limitée (~50-100 joueurs), single-thread |
| **PowerNukkitX** | Java 17+ | API Bukkit-like familière | Parité vanilla ~50%, moins maintenu |
| **Dragonfly** | Go 1.21+ | Architecture propre, performant | Pas de plugins dynamiques, framework only |
| **Geyser** | Java | Pont Java↔Bedrock | Proxy, pas un serveur complet |
| **MC-RS** (ce projet) | Rust | Performance, sécurité, plugins WASM | Nouveau, tout à construire |

### Situation en Rust

Aucun serveur Bedrock complet n'existe en Rust à ce jour. Les projets existants sont :
- **rak-rs** (NetrexMC) — Implémentation RakNet pour Bedrock en Rust
- **bedrock-rs** (bedrock-crustaceans) — Bibliothèque de protocole Bedrock en Rust
- **Zuri** — Tentative de serveur inspiré de Dragonfly, stade précoce

**MC-RS a l'opportunité d'être le premier serveur Bedrock complet en Rust.**

## Objectifs du projet

### Objectifs primaires
1. **Connexion fonctionnelle** — Un client Bedrock peut se connecter, s'authentifier et spawn dans un monde
2. **Monde jouable** — Chunks générés, blocs cassables/plaçables, physique de base
3. **Multi-joueur** — Voir les autres joueurs, chat, interactions basiques
4. **Système de plugins** — API extensible avec support WASM et/ou Lua
5. **Performance** — Supporter 500+ joueurs sur du matériel raisonnable

### Objectifs secondaires
6. **Compatibilité addons** — Support partiel des behavior/resource packs Bedrock
7. **Commandes** — Système de commandes slash complet
8. **Anti-triche** — Validation server-authoritative du mouvement et de l'inventaire
9. **Persistance** — Sauvegarde/chargement de mondes au format LevelDB compatible BDS
10. **Multi-monde** — Overworld, Nether, End avec transfert de dimension

### Non-objectifs (pour l'instant)
- Parité vanilla 100% (irréaliste sans des années de travail)
- Support du Marketplace Bedrock
- Compatibilité avec l'édition Java (c'est le rôle de Geyser)

## Architecture haut niveau

```
┌─────────────────────────────────────────────────────────┐
│                     MC-RS Server                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │  Plugin   │    │   Command    │    │    Forms      │  │
│  │  System   │    │   System     │    │    UI         │  │
│  │(WASM/Lua) │    │ (Brigadier)  │    │  (Modal/     │  │
│  └─────┬─────┘    └──────┬───────┘    │   Custom)    │  │
│        │                 │            └──────┬────────┘  │
│  ┌─────┴─────────────────┴───────────────────┴────────┐  │
│  │                  Game Logic Layer                    │  │
│  │  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌───────┐  │  │
│  │  │  World  │ │ Entities │ │ Inventory │ │ Block │  │  │
│  │  │ Manager │ │  (ECS)   │ │  System   │ │ Ticks │  │  │
│  │  └────┬────┘ └────┬─────┘ └─────┬─────┘ └───┬───┘  │  │
│  └───────┼───────────┼─────────────┼───────────┼──────┘  │
│          │           │             │           │          │
│  ┌───────┴───────────┴─────────────┴───────────┴──────┐  │
│  │                Protocol Layer                       │  │
│  │  ┌──────────┐ ┌─────────────┐ ┌────────────────┐   │  │
│  │  │  Packet  │ │ Compression │ │  Encryption    │   │  │
│  │  │  Codec   │ │ (zlib/snappy│ │ (AES-256-CFB8) │   │  │
│  │  └────┬─────┘ └──────┬──────┘ └───────┬────────┘   │  │
│  └───────┼──────────────┼────────────────┼────────────┘  │
│          │              │                │               │
│  ┌───────┴──────────────┴────────────────┴────────────┐  │
│  │              RakNet Transport Layer                  │  │
│  │  UDP Socket ←→ Reliability ←→ Fragmentation         │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                  Storage Layer                       │  │
│  │  LevelDB (monde) │ Config (TOML) │ Player Data      │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Structure du workspace Cargo

```
mc-rs/
├── Cargo.toml                    # Workspace root
├── docs/                         # Cette documentation
├── data/                         # Données statiques (block palette, items, recipes)
├── crates/
│   ├── mc-rs-raknet/             # Transport RakNet (UDP, reliability, fragmentation)
│   ├── mc-rs-proto/              # Définitions de paquets, codec, sérialisation
│   ├── mc-rs-nbt/                # NBT little-endian + variante réseau
│   ├── mc-rs-crypto/             # ECDH P-384, AES-256-CFB8, JWT, Xbox Live auth
│   ├── mc-rs-world/              # LevelDB, chunks, block palette, biomes
│   ├── mc-rs-game/               # Logique de jeu, ECS, systèmes
│   ├── mc-rs-command/            # Framework de commandes
│   ├── mc-rs-plugin-api/         # Traits/interfaces pour les plugins
│   ├── mc-rs-plugin-wasm/        # Runtime WASM (wasmtime) pour plugins
│   ├── mc-rs-plugin-lua/         # Runtime Lua (mlua) pour scripts
│   └── mc-rs-server/             # Orchestration, point d'entrée, configuration
└── plugins/                      # Dossier pour les plugins utilisateur
```

## Protocole en bref

Le protocole Minecraft Bedrock s'empile ainsi :

```
          Application (Game Packets)
               ↓ batching (multiple packets par frame)
          Compression (zlib / snappy / none)
               ↓
          Encryption (AES-256-CFB8, optionnel mais requis pour Xbox auth)
               ↓
          Game Packet Wrapper (header 0xFE)
               ↓
          RakNet Framing (reliability, ordering, fragmentation)
               ↓
          UDP Transport
```

**Port par défaut :** `19132` (UDP)

**Séquence de connexion simplifiée :**
1. Client → `UnconnectedPing` → Serveur répond `UnconnectedPong` (liste de serveurs)
2. Handshake RakNet (OpenConnectionRequest/Reply 1 & 2)
3. Client envoie `RequestNetworkSettings` → Serveur répond `NetworkSettings` (active la compression)
4. Client envoie `LoginPacket` (JWT Xbox Live + données skin)
5. Serveur valide, établit le chiffrement (ECDH + AES)
6. Échange de resource packs
7. `StartGamePacket` (paquet massif avec toute la config du monde)
8. Envoi des chunks, entités, inventaire
9. `PlayStatusPacket(PlayerSpawn)` — Le joueur apparaît dans le monde

## Versions du protocole

Le protocole Bedrock change à chaque mise à jour majeure. Chaque version client a un **numéro de protocole** :

| Version du jeu | Protocole |
|---------------|-----------|
| 1.20.80 | 671 |
| 1.21.0 | 685 |
| 1.21.20 | 712 |
| 1.21.30 | 729 |
| 1.21.40 | 748 |
| 1.21.50 | 766 |
| 1.26.0 | 924 |

MC-RS ciblera d'abord la **dernière version stable** et maintiendra la compatibilité avec les 1-2 versions précédentes.

## Références clés

| Ressource | URL |
|-----------|-----|
| wiki.vg Bedrock Protocol | https://wiki.vg/Bedrock_Protocol |
| PMMP BedrockProtocol (PHP) | https://github.com/pmmp/BedrockProtocol |
| PMMP BedrockData | https://github.com/pmmp/BedrockData |
| Gophertunnel (Go) | https://github.com/sandertv/gophertunnel |
| Dragonfly (Go) | https://github.com/df-mc/dragonfly |
| CloudburstMC Protocol (Java) | https://github.com/CloudburstMC/Protocol |
| Bedrock Wiki (addons) | https://wiki.bedrock.dev/ |
| Minecraft Wiki (technique) | https://minecraft.wiki/ |
| rak-rs (Rust RakNet) | https://github.com/NetrexMC/RakNet |
| bedrock-rs (Rust protocol) | https://github.com/bedrock-crustaceans/bedrock-rs |
