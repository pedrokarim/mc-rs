# MC-RS Documentation

Documentation exhaustive pour la création d'un serveur Minecraft Bedrock Edition en Rust.

## Table des matières

| # | Document | Description |
|---|----------|-------------|
| 01 | [Vue d'ensemble](01-overview.md) | Vision du projet, positionnement, architecture haut niveau, structure du workspace |
| 02 | [Protocole](02-protocol.md) | Protocole Bedrock complet : types de données, séquence de connexion, table des paquets, détails des paquets clés |
| 03 | [Architecture](03-architecture.md) | Architecture du serveur : concurrence, boucle de tick, structure des crates, patterns |
| 04 | [Réseau (RakNet)](04-networking.md) | Couche transport RakNet : paquets offline/online, fiabilité, fragmentation, compression, chiffrement |
| 05 | [Monde](05-world.md) | Format du monde : LevelDB, sub-chunks, palettes, biomes, level.dat, génération |
| 06 | [Entités (ECS)](06-entities.md) | Système d'entités : composants, metadata, attributs, IA, pathfinding, spawn |
| 07 | [Gameplay](07-gameplay.md) | Mécaniques de jeu : physique, combat, faim, XP, commandes, formulaires, météo |
| 08 | [Plugins](08-plugins.md) | Système de plugins : API Rust, WASM, Lua, Behavior Packs, événements |
| 09 | [Sécurité](09-security.md) | Authentification Xbox Live, chiffrement AES, anti-triche, validation |
| 10 | [Performance](10-performance.md) | Optimisations : réseau, chunks, entités, mémoire, benchmarks |
| 11 | [Crates Rust](11-rust-crates.md) | Dépendances : toutes les crates nécessaires avec justification |
| 12 | [Roadmap](12-roadmap.md) | Plan de développement : 5 phases, milestones, checklists |

## Ressources externes

- [wiki.vg Bedrock Protocol](https://wiki.vg/Bedrock_Protocol) — Documentation protocole communautaire
- [pmmp/BedrockData](https://github.com/pmmp/BedrockData) — Données canoniques (palettes, items, recettes)
- [pmmp/BedrockProtocol](https://github.com/pmmp/BedrockProtocol) — Implémentation de référence en PHP
- [sandertv/gophertunnel](https://github.com/sandertv/gophertunnel) — Implémentation Go + outil ProxyPass
- [df-mc/dragonfly](https://github.com/df-mc/dragonfly) — Serveur Go (architecture de référence)
- [Bedrock Wiki](https://wiki.bedrock.dev/) — Documentation addons/modding Bedrock
- [minecraft.wiki](https://minecraft.wiki/) — Wiki technique Minecraft
