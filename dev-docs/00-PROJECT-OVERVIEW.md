# MC-RS : PocketMine-MP en Rust

## Vision

Réécrire PocketMine-MP (serveur Minecraft Bedrock Edition en PHP) entièrement en Rust.
Objectif : un serveur performant, sûr (memory-safe), et extensible via un système de plugins.

## Protocole cible

- **Protocol version** : 924
- **Minecraft Bedrock** : 1.26.x
- **RakNet protocol** : 11

## Architecture PocketMine-MP (source)

PocketMine-MP est organisé en ~15 sous-systèmes majeurs :

| # | Sous-système | Description | Doc détaillée |
|---|---|---|---|
| 1 | **Server Core** | Boucle principale (20 TPS), bootstrap, config | [01-SERVER-CORE.md](01-SERVER-CORE.md) |
| 2 | **Network / RakNet** | Couche réseau UDP, RakLib, sessions | [02-NETWORK-RAKNET.md](02-NETWORK-RAKNET.md) |
| 3 | **Protocol / Packets** | 542 packets MCPE, sérialisation binaire | [03-PROTOCOL-PACKETS.md](03-PROTOCOL-PACKETS.md) |
| 4 | **Login Flow** | Authentification, encryption, handshake | [04-LOGIN-FLOW.md](04-LOGIN-FLOW.md) |
| 5 | **World System** | Mondes, chunks, sub-chunks, LevelDB | [05-WORLD-SYSTEM.md](05-WORLD-SYSTEM.md) |
| 6 | **Block System** | Blocs, états, palettes, tiles | [06-BLOCK-SYSTEM.md](06-BLOCK-SYSTEM.md) |
| 7 | **Entity System** | Entités, Living, Human, Player | [07-ENTITY-SYSTEM.md](07-ENTITY-SYSTEM.md) |
| 8 | **Item System** | Items, durabilité, enchantements | [08-ITEM-SYSTEM.md](08-ITEM-SYSTEM.md) |
| 9 | **Inventory System** | Inventaires, transactions, crafting grid | [09-INVENTORY-SYSTEM.md](09-INVENTORY-SYSTEM.md) |
| 10 | **Event System** | Événements, priorités, annulation | [10-EVENT-SYSTEM.md](10-EVENT-SYSTEM.md) |
| 11 | **Plugin System** | Chargement, API, lifecycle | [11-PLUGIN-SYSTEM.md](11-PLUGIN-SYSTEM.md) |
| 12 | **Command System** | Commandes, permissions, dispatch | [12-COMMAND-SYSTEM.md](12-COMMAND-SYSTEM.md) |
| 13 | **Scheduler / Async** | Tasks synchrones, async pool, workers | [13-SCHEDULER-ASYNC.md](13-SCHEDULER-ASYNC.md) |
| 14 | **World Generation** | Générateurs, bruit, populators | [14-WORLD-GENERATION.md](14-WORLD-GENERATION.md) |
| 15 | **Crafting System** | Recettes shaped/shapeless, fourneau, potions | [15-CRAFTING-SYSTEM.md](15-CRAFTING-SYSTEM.md) |

## Plan de développement par phases

Voir [99-ROADMAP.md](99-ROADMAP.md) pour le plan de développement phase par phase.

## Structure Rust prévue (crates)

Voir [98-RUST-ARCHITECTURE.md](98-RUST-ARCHITECTURE.md) pour l'architecture Rust cible.

## Références

- PocketMine-MP source : `.reference/PocketMine-MP/`
- BedrockProtocol : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/`
- RakLib : `.reference/PocketMine-MP/vendor/pocketmine/raklib/`
- BedrockData : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-data/`
- NBT : `.reference/PocketMine-MP/vendor/pocketmine/nbt/`
