# 12 - Roadmap de développement

## Phases du projet

```
Phase 0 : Fondations         ██████████ (2-3 mois)
Phase 1 : Prototype jouable  ██████████ (3-4 mois)
Phase 2 : Multi-joueur       ██████████ (2-3 mois)
Phase 3 : Gameplay core      ██████████ (4-6 mois)
Phase 4 : Plugins            ██████████ (2-3 mois)
Phase 5 : Polish             ██████████ (continu)
```

---

## Phase 0 : Fondations

**Objectif :** Construire les briques de base. Un client Bedrock peut se connecter et voir un monde vide.

### 0.1 — Setup du projet
- [x] Initialiser le workspace Cargo avec tous les crates
- [x] Setup CI/CD (GitHub Actions : build, test, clippy, fmt)
- [x] Configuration tracing (logging)
- [x] Fichier de configuration serveur (server.toml)
- [x] Structure de données de base (Vec3, BlockPos, ChunkPos, etc.)

### 0.2 — RakNet
- [x] Socket UDP asynchrone (tokio)
- [x] Paquets offline : UnconnectedPing/Pong (serveur visible dans la liste)
- [x] Handshake : OpenConnectionRequest/Reply 1 & 2
- [x] Paquets online : ConnectionRequest/Accepted, NewIncomingConnection
- [x] FrameSet : parsing et construction
- [x] Fiabilité : ACK/NACK, retransmission
- [x] Ordering channels
- [x] Fragmentation/réassemblage
- [x] ConnectedPing/Pong (keep-alive)
- [x] Gestion du timeout et déconnexion

**Milestone :** Le serveur apparaît dans la liste des serveurs LAN Bedrock.

### 0.3 — Protocole de base
- [x] NBT little-endian (reader/writer)
- [x] NBT réseau (variante VarInt)
- [x] Types de sérialisation (VarInt, VarLong, String, Vec3, BlockPos, etc.)
- [x] Game packet wrapper (0xFE)
- [x] Compression zlib
- [x] Décompression (zlib + snappy)
- [x] Batching de paquets

### 0.4 — Login
- [x] RequestNetworkSettings → NetworkSettings
- [x] LoginPacket parsing (JWT chain + client data)
- [x] Validation JWT (parsing + mode offline)
- [x] PlayStatus(LoginSuccess)
- [x] Mode offline (skip Xbox validation)

### 0.5 — Chiffrement
- [x] Génération de paire de clés ECDH P-384
- [x] ServerToClientHandshake (envoi clé publique)
- [x] ClientToServerHandshake
- [x] Dérivation clé AES-256 via SHA-256
- [x] Encryption/decryption AES-256-CFB8
- [x] Checksum SHA-256 des paquets

### 0.6 — Initialisation du monde
- [x] ResourcePacksInfo → ResourcePackClientResponse flow
- [x] ResourcePackStack → ResourcePackClientResponse(Completed)
- [x] StartGamePacket (construction du paquet avec toutes les données)
- [x] Block palette (FNV-1a hashing au lieu de charger depuis pmmp/BedrockData)
- [x] CreativeContent
- [x] BiomeDefinitionList
- [x] AvailableEntityIdentifiers
- [x] ChunkRadiusUpdated
- [x] NetworkChunkPublisherUpdate

### 0.7 — Chunks basiques
- [x] Générateur de monde plat (flat world)
- [x] Sérialisation SubChunk (format réseau, palette + packed array)
- [x] LevelChunk packet
- [x] Envoi des chunks autour du spawn
- [x] SetLocalPlayerAsInitialized + PlayStatus(PlayerSpawn)

**Milestone Phase 0 :** Un client Bedrock se connecte, voit un monde plat, et peut se tenir dessus.

---

## Phase 1 : Prototype jouable

**Objectif :** Un joueur seul peut se déplacer, casser et placer des blocs.

### 1.1 — Mouvement
- [x] PlayerAuthInput parsing
- [x] Simulation de mouvement basique (gravité, collision AABB)
- [x] MovePlayer broadcast (multi-joueur)
- [x] Validation server-authoritative (basique)
- [x] Correction de position si nécessaire

### 1.2 — Blocs
- [x] Block registry (chargement de la palette complète)
- [x] Calcul du temps de minage
- [x] UpdateBlock packet
- [x] Block break → UpdateBlock(air) (via InventoryTransaction, creative mode)
- [x] Block place (InventoryTransaction)
- [ ] Block entities basiques (panneaux, coffres)

### 1.3 — Inventaire basique
- [x] Structure ItemStack
- [x] Inventaire joueur (36 slots + armure + offhand)
- [x] InventoryContent (envoi initial)
- [x] MobEquipment (item en main)
- [x] ItemStackRequest/Response (move, take, place, drop)
- [x] Drop d'items (AddItemEntity)
- [x] Ramassage d'items (TakeItemEntity)

### 1.4 — Chat et commandes de base
- [x] Text packet (chat)
- [x] Commandes basiques : /help, /stop, /say, /list
- [x] AvailableCommands packet
- [x] CommandRequest handling
- [x] CommandOutput

### 1.5 — Chunk loading dynamique
- [x] RequestChunkRadius (spawn + in-game)
- [x] Chargement de chunks selon la position du joueur
- [x] Déchargement des chunks hors de portée (client gère via publisher radius)
- [x] NetworkChunkPublisherUpdate à chaque déplacement

**Milestone Phase 1 :** Un joueur peut se déplacer, casser/placer des blocs, et discuter.

---

## Phase 2 : Multi-joueur

**Objectif :** Plusieurs joueurs dans le même monde.

### 2.1 — Gestion multi-joueur
- [x] AddPlayer packet (spawn d'un autre joueur)
- [x] RemoveEntity (despawn)
- [x] PlayerList (tab list)
- [x] Broadcast des mouvements entre joueurs
- [x] Broadcast des actions (break/place/interact)
- [x] Skins (envoi des données skin entre joueurs)
- [ ] PlayerSkin packet (changement de skin)

### 2.2 — Interaction entre joueurs
- [x] Combat PvP (dégâts, knockback)
- [x] Invulnérabilité post-dégâts (10 ticks)
- [x] Animate packet (swing arm)
- [x] Effets de mort (respawn flow)

### 2.3 — Commandes joueur
- [x] /gamemode
- [x] /tp (teleport)
- [x] /give
- [x] /kill
- [x] /kick
- [x] /op, /deop
- [x] Entity selectors (@a, @p, @r, @e, @s)

### 2.4 — Permissions
- [x] Système de permissions basique (visitor, member, operator)
- [x] UpdateAbilities packet
- [x] Whitelist
- [x] Ban list (IP + joueur)

**Milestone Phase 2 :** Plusieurs joueurs peuvent jouer ensemble, se voir, combattre.

---

## Phase 3 : Gameplay core

**Objectif :** Le jeu est fonctionnel avec les mécaniques principales.

### 3.1 — Entités (ECS)
- [ ] Setup bevy_ecs
- [ ] Composants de base (Position, Velocity, Health, etc.)
- [ ] Systèmes de base (physics, movement, despawn)
- [ ] Spawn de mobs basiques (zombie, squelette, creeper)
- [ ] Entity metadata synchronization (SetEntityData)
- [ ] EntityEvent, UpdateAttributes

### 3.2 — IA des mobs
- [ ] Système de behaviors par priorité
- [ ] Pathfinding A* basique
- [ ] Behaviors : RandomStroll, LookAtPlayer, Float
- [ ] Behaviors hostiles : MeleeAttack, HurtByTarget, NearestAttackableTarget
- [ ] Behaviors passifs : Panic, TemptGoal, BreedGoal
- [ ] Spawn naturel avec caps

### 3.3 — Combat complet
- [ ] Dégâts par arme
- [ ] Enchantements offensifs et défensifs
- [ ] Effets de potion (Strength, Weakness, etc.)
- [ ] Coups critiques
- [ ] Réduction par armure
- [ ] Knockback
- [ ] Projectiles (flèches, tridents)

### 3.4 — Survie
- [ ] Système de faim (food, saturation, exhaustion)
- [ ] Régénération naturelle
- [ ] Famine (dégâts)
- [ ] Nourriture (consommation, valeurs nutritives)
- [ ] Dégâts de chute
- [ ] Noyade
- [ ] Feu et lave
- [ ] Suffocation

### 3.5 — Crafting
- [ ] CraftingData packet (envoi des recettes au login)
- [ ] ItemStackRequest(CraftRecipe)
- [ ] Recettes shaped et shapeless
- [ ] Four (smelting)
- [ ] Table d'enchantement
- [ ] Enclume
- [ ] Pierre à aiguiser
- [ ] Tailleur de pierre
- [ ] Métier à tisser

### 3.6 — Génération de monde
- [ ] Générateur overworld (biomes, terrain, grottes)
- [ ] Minerais
- [ ] Arbres et végétation
- [ ] Eau et lave
- [ ] Structures basiques (villages, donjons)
- [ ] Générateur Nether
- [ ] Générateur End

### 3.7 — Monde persistant
- [ ] LevelDB provider
- [ ] Sauvegarde/chargement de chunks
- [ ] level.dat
- [ ] Sauvegarde des joueurs
- [ ] Auto-save périodique

### 3.8 — Block ticks et redstone
- [ ] Random ticks (crops, herbe, feuilles)
- [ ] Scheduled ticks
- [ ] Eau et lave (flow)
- [ ] Sable/gravier (gravité)
- [ ] Redstone basique (wire, torch, repeater, lever)
- [ ] Pistons

### 3.9 — Expérience et enchantements
- [ ] Système XP (gain, perte, niveaux)
- [ ] Table d'enchantement
- [ ] Enchantements (stockage NBT, application d'effets)
- [ ] Réparation (enclume)

### 3.10 — Météo et cycle jour/nuit
- [ ] Cycle jour/nuit (daylight cycle)
- [ ] Game rule doDaylightCycle
- [ ] Pluie et orage
- [ ] Lightning (foudre)
- [ ] Commandes /time et /weather

**Milestone Phase 3 :** Le jeu survival est jouable avec mobs, combat, craft, mining.

---

## Phase 4 : Plugins et extensibilité

**Objectif :** Les développeurs tiers peuvent créer des plugins.

### 4.1 — Plugin API Rust
- [ ] Trait Plugin
- [ ] Système d'événements (EventBus)
- [ ] Tous les événements (joueur, entité, monde, serveur)
- [ ] API joueur, monde, entité
- [ ] Système de commandes plugin
- [ ] Scheduler (delayed, repeating tasks)
- [ ] Configuration par plugin

### 4.2 — Plugins WASM
- [ ] Runtime wasmtime
- [ ] Host functions (API serveur → plugin)
- [ ] Guest exports (plugin → serveur callbacks)
- [ ] Manifest plugin.toml
- [ ] Sandbox (CPU fuel, mémoire max)
- [ ] Hot-reload (/reload command)

### 4.3 — Scripts Lua
- [ ] Runtime mlua/LuaJIT
- [ ] API Lua (événements, joueurs, monde)
- [ ] Sandbox Lua (suppression os, io, debug)
- [ ] Hot-reload
- [ ] Commandes Lua

### 4.4 — Behavior Packs (Bedrock addons)
- [ ] Parsing manifest.json
- [ ] Entités custom (JSON)
- [ ] Items custom (JSON)
- [ ] Blocs custom (JSON)
- [ ] Recettes custom
- [ ] Loot tables
- [ ] Resource pack transfer (envoi aux clients)

### 4.5 — Formulaires (Forms UI)
- [ ] SimpleForm
- [ ] ModalForm
- [ ] CustomForm
- [ ] API forms pour les plugins
- [ ] ModalFormRequest/Response handling

**Milestone Phase 4 :** Les développeurs peuvent créer et distribuer des plugins.

---

## Phase 5 : Polish et fonctionnalités avancées

### 5.1 — Performance
- [ ] Profiling et optimisation des hot paths
- [ ] SIMD pour la génération de monde
- [ ] Chunk cache (blob protocol)
- [ ] Spatial indexing pour les entités
- [ ] Parallel chunk serialization

### 5.2 — Commandes avancées
- [ ] /setblock, /fill, /clone
- [ ] /execute (sous-commandes complexes)
- [ ] /scoreboard
- [ ] /tag
- [ ] /title
- [ ] /bossbar
- [ ] /particle
- [ ] /playsound

### 5.3 — Fonctionnalités serveur
- [ ] RCON (Remote Console)
- [ ] Query protocol (pour les sites de monitoring)
- [ ] Serveur transfer (/transfer)
- [ ] Multi-monde (Overworld + Nether + End avec portails)
- [ ] Ticking areas
- [ ] Console interactive (REPL)

### 5.4 — Anti-triche avancé
- [ ] Mouvement server-authoritative complet
- [ ] Detection de speed hack
- [ ] Detection de fly
- [ ] Detection de no-clip
- [ ] Mining speed validation
- [ ] Reach validation
- [ ] Rate limiting des actions

### 5.5 — Compatibilité
- [ ] Support multi-version protocole (dernières 2-3 versions)
- [ ] Import de mondes BDS existants
- [ ] Export de mondes compatible BDS

---

## Milestone tracker

| Milestone | Description | Critère de succès |
|-----------|-------------|-------------------|
| **M0** | Serveur visible | Le serveur apparaît dans la liste LAN |
| **M1** | Connexion | Un client se connecte et voit un monde plat |
| **M2** | Mouvement | Le joueur peut se déplacer dans le monde |
| **M3** | Blocs | Le joueur peut casser et placer des blocs |
| **M4** | Multi** | Plusieurs joueurs se voient et interagissent |
| **M5** | Survie | Combat, faim, mobs, craft fonctionnels |
| **M6** | Monde** | Monde généré avec biomes, grottes, structures |
| **M7** | Plugins | Un plugin peut modifier le comportement du serveur |
| **M8** | Production | Stable, performant, 100+ joueurs simultanés |

---

## Conseils de développement

### Tester fréquemment

- **ProxyPass** (gophertunnel) : Capturer le trafic entre un client et BDS pour comparer
- **Tests unitaires** pour chaque crate (sérialisation, crypto, NBT, etc.)
- **Tests d'intégration** : Client Bedrock → serveur MC-RS en CI
- **Comparer les paquets** : Sérialiser un paquet avec MC-RS et avec BDS, diff binaire

### Données de référence

- **pmmp/BedrockData** : Block palettes, item tables, recipes, creative content, entity identifiers, biome definitions — extraits de BDS, mis à jour régulièrement
- **Utiliser ces données plutôt que les hardcoder** — elles changent à chaque version

### Ne pas tout faire seul

- Contribuer aux crates existants (rak-rs, bedrock-rs) plutôt que tout réécrire
- Étudier le code de Dragonfly (Go) et PocketMine-MP (PHP) pour comprendre les subtilités
- La communauté Bedrock modding (discord PMMP, Bedrock Wiki) est une mine d'or

### Priorité à la boucle de feedback

L'ordre d'implémentation est conçu pour avoir un **feedback visuel** le plus tôt possible :
1. Voir le serveur dans la liste → Motivation
2. Se connecter et voir un monde → Validation du protocole
3. Bouger → Validation de la physique
4. Casser des blocs → Validation de l'interaction
5. Jouer à plusieurs → Validation du multi-joueur

Chaque milestone doit être **testable par un humain** avec un vrai client Bedrock.
