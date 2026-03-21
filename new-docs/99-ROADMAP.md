# 99 - Roadmap : Plan de développement

## Philosophie

Avancer **phase par phase**, chaque phase est **testable et fonctionnelle** avant de passer à la suivante. On reproduit PocketMine en Rust, pas besoin de réinventer.

---

## Phase 1 : Foundation (réseau + login)

**Objectif :** Un client peut se connecter, voir le serveur dans la liste, et arriver au monde.

### 1.1 - RakNet (`mc-rs-raknet`)
- [ ] Socket UDP (tokio)
- [ ] UnconnectedPing / UnconnectedPong (server list)
- [ ] OpenConnectionRequest/Reply 1 & 2
- [ ] ConnectionRequest / ConnectionRequestAccepted
- [ ] Datagrams + EncapsulatedPacket
- [ ] ACK / NACK
- [ ] Reliability layers (reliable, ordered)
- [ ] Split packet reassembly
- [ ] ConnectedPing / ConnectedPong

### 1.2 - Protocol base (`mc-rs-proto`)
- [ ] Types de base : VarInt, VarUInt, String, Vec3, BlockPos, UUID
- [ ] NBT Network LE (`mc-rs-nbt`)
- [ ] Trait Packet + encode/decode
- [ ] Batch compression/décompression (zlib)
- [ ] Paquets login : RequestNetworkSettings, NetworkSettings, Login, PlayStatus
- [ ] Paquets handshake : ServerToClientHandshake, ClientToServerHandshake

### 1.3 - Network session (`mc-rs-network`)
- [ ] State machine (SessionStart → Login → Handshake → ResourcePacks → PreSpawn → InGame)
- [ ] SessionStartHandler (RequestNetworkSettings → NetworkSettings)
- [ ] LoginHandler (Login → validation → ServerToClientHandshake)
- [ ] JWT parsing (pas de validation Xbox pour l'instant, mode offline)
- [ ] HandshakeHandler (encryption AES-256-CTR)
- [ ] ResourcePacksHandler (envoyer liste vide → client accepte)

### 1.4 - PreSpawn minimal
- [ ] StartGamePacket (minimal, hardcodé)
- [ ] BiomeDefinitionListPacket (blob NBT)
- [ ] AvailableActorIdentifiersPacket (blob NBT)
- [ ] CreativeContentPacket (vide)
- [ ] CraftingDataPacket (vide)
- [ ] PlayStatusPacket (PLAYER_SPAWN)
- [ ] RequestChunkRadius / ChunkRadiusUpdated

### 1.5 - Chunks minimaux
- [ ] Flat generator (bedrock + dirt + grass)
- [ ] LevelChunkPacket (sub-chunks palettés)
- [ ] Envoyer les chunks autour du spawn
- [ ] Le client peut voir le monde !

**Résultat Phase 1 :** Le joueur se connecte, voit un monde plat, mais ne peut rien faire.

---

## Phase 2 : Player basics

**Objectif :** Le joueur peut se déplacer, voir les autres joueurs, et avoir un inventaire basique.

### 2.1 - Mouvement
- [ ] PlayerAuthInputPacket (recevoir)
- [ ] MovePlayerPacket (broadcast aux autres)
- [ ] Validation de position (anti-fly basique)
- [ ] Chunk loading/unloading basé sur la position du joueur

### 2.2 - Multi-joueurs
- [ ] PlayerListPacket (add/remove)
- [ ] AddPlayerPacket (spawn d'un autre joueur)
- [ ] RemoveActorPacket (despawn)
- [ ] SetEntityDataPacket (métadonnées joueur)

### 2.3 - Inventaire basique
- [ ] PlayerInventory (36 slots)
- [ ] ArmorInventory (4 slots)
- [ ] InventoryContentPacket (envoyer l'inventaire)
- [ ] ItemStackRequest / ItemStackResponse basique

### 2.4 - Chat & commandes basiques
- [ ] TextPacket (chat)
- [ ] CommandRequestPacket
- [ ] Commandes : /help, /list, /stop, /gamemode
- [ ] AvailableCommandsPacket

**Résultat Phase 2 :** Multi-joueurs fonctionnel, déplacement, chat, commandes basiques.

---

## Phase 3 : World interaction

**Objectif :** Le joueur peut casser/placer des blocs et interagir avec le monde.

### 3.1 - Blocs
- [ ] Block registry (tous les blocs vanilla)
- [ ] Block states + runtime IDs
- [ ] UpdateBlockPacket
- [ ] Block breaking (PlayerAuthInput + animation)
- [ ] Block placing
- [ ] Block drops (items)

### 3.2 - Items complets
- [ ] Item registry (tous les items vanilla)
- [ ] Durabilité des outils
- [ ] Tool tiers (bois → netherite)
- [ ] Vitesse de minage selon l'outil

### 3.3 - World persistence
- [ ] LevelDB read/write
- [ ] Sauvegarder les chunks modifiés
- [ ] Charger les chunks depuis LevelDB
- [ ] level.dat (métadonnées monde)
- [ ] Auto-save périodique

### 3.4 - Block entities (tiles)
- [ ] Chest (inventaire 27 slots)
- [ ] Furnace (cuisson basique)
- [ ] Sign (texte)
- [ ] Bed (spawn point)

**Résultat Phase 3 :** Gameplay Minecraft basique : miner, construire, ouvrir des coffres.

---

## Phase 4 : Entities & Combat

**Objectif :** Entités, dégâts, mort, respawn.

### 4.1 - Système d'entités
- [ ] Entity base (position, vélocité, hitbox)
- [ ] Entity spawn/despawn (AddActorPacket, RemoveActorPacket)
- [ ] Entity movement (MoveActorAbsolutePacket)
- [ ] Entity metadata sync

### 4.2 - Combat
- [ ] Dégâts PvP
- [ ] Knockback
- [ ] Invincibilité frames
- [ ] Mort + respawn
- [ ] Effets de potion basiques

### 4.3 - Mobs passifs
- [ ] ItemEntity (items droppés)
- [ ] ExperienceOrb
- [ ] FallingBlock
- [ ] PrimedTNT

### 4.4 - Attributs & effets
- [ ] AttributeMap (santé, vitesse, dégâts)
- [ ] EffectManager (effets de potion)
- [ ] HungerManager
- [ ] ExperienceManager

**Résultat Phase 4 :** Combat fonctionnel, mobs basiques, système de survie.

---

## Phase 5 : Game systems

**Objectif :** Les systèmes de jeu complets.

### 5.1 - Crafting
- [ ] CraftingManager (recettes JSON)
- [ ] ShapedRecipe + ShapelessRecipe
- [ ] Crafting table (3x3)
- [ ] Inventory crafting (2x2)
- [ ] FurnaceRecipe (cuisson)

### 5.2 - Enchantements
- [ ] Table d'enchantement
- [ ] Enchantements sur items
- [ ] Effets d'enchantement (Sharpness, Protection, etc.)
- [ ] Anvil

### 5.3 - Permissions
- [ ] Permission system (hiérarchique)
- [ ] Operator status
- [ ] Ban list (player + IP)
- [ ] Whitelist

### 5.4 - World generation améliorée
- [ ] Normal generator (noise-based terrain)
- [ ] Biomes
- [ ] Ore populator
- [ ] Tree populator
- [ ] Caves
- [ ] Structures basiques

**Résultat Phase 5 :** Serveur de survie complet avec crafting et génération de terrain.

---

## Phase 6 : Plugin system

**Objectif :** Les plugins peuvent étendre le serveur.

### 6.1 - Event system
- [ ] EventManager (dispatch, priorités)
- [ ] Événements joueur (join, quit, chat, move, interact)
- [ ] Événements bloc (break, place)
- [ ] Événements entité (damage, death)
- [ ] Événements serveur (start, stop)

### 6.2 - Plugin API
- [ ] PluginManifest (plugin.yml)
- [ ] Plugin lifecycle (load, enable, disable)
- [ ] Dependency resolution
- [ ] Plugin data folder + config

### 6.3 - Lua plugins
- [ ] LuaPluginLoader
- [ ] API Lua : events, commands, scheduler
- [ ] Exemples de plugins

### 6.4 - Commandes avancées
- [ ] Toutes les commandes vanilla (~40)
- [ ] Plugin commands
- [ ] Command autocomplete (AvailableCommandsPacket)
- [ ] Target selectors (@a, @p, @r, @s, @e)

**Résultat Phase 6 :** Serveur extensible via plugins Lua.

---

## Phase 7 : Polish & Performance

**Objectif :** Prêt pour la production.

### 7.1 - Performance
- [ ] Chunk caching réseau
- [ ] Packet batching optimisé
- [ ] Compression async
- [ ] Entity culling (distance)
- [ ] View distance dynamique

### 7.2 - Fonctionnalités avancées
- [ ] Resource packs (envoi au client)
- [ ] Skins custom
- [ ] Scoreboard
- [ ] Boss bar
- [ ] Title/subtitle/actionbar
- [ ] Particles

### 7.3 - Administration
- [ ] Console interactive
- [ ] RCON
- [ ] Query protocol
- [ ] Server status
- [ ] Timings / profiling

### 7.4 - Xbox Live auth
- [ ] Validation JWT complète
- [ ] Fetch Mojang public keys
- [ ] XUID tracking

**Résultat Phase 7 :** Serveur de qualité production.

---

## Résumé des phases

| Phase | Description | Complexité |
|---|---|---|
| 1 | Foundation (réseau + login + monde plat) | ████░░░░░░ |
| 2 | Player basics (mouvement, chat, inventaire) | ████░░░░░░ |
| 3 | World interaction (blocs, persistence) | █████░░░░░ |
| 4 | Entities & Combat | ██████░░░░ |
| 5 | Game systems (crafting, enchant, worldgen) | ███████░░░ |
| 6 | Plugin system | ██████░░░░ |
| 7 | Polish & Performance | ████████░░ |

**On commence par la Phase 1.** Chaque phase est testable indépendamment.
