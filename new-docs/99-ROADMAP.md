# 99 - Roadmap : Plan de développement

## Philosophie

Avancer **phase par phase**, chaque phase est **testable et fonctionnelle** avant de passer à la suivante. On reproduit PocketMine en Rust, pas besoin de réinventer.

---

## Phase 1 : Foundation (réseau + login) — ✅ TERMINÉE

**Objectif :** Un client peut se connecter, voir le serveur dans la liste, et arriver au monde.

- [x] RakNet complet (UDP, sessions, reliability, ACK/NACK, split reassembly)
- [x] Protocol base (VarInt, batch codec zlib/snappy, packet header)
- [x] State machine (SessionStart → Login → Handshake → ResourcePacks → PreSpawn → InGame)
- [x] Login Xbox Live (JWT parsing, ECDH P-384, AES-256-CTR fakeGCM)
- [x] PreSpawn (StartGame, BiomeDefinitionList, ActorIdentifiers, CraftingData, CreativeContent)
- [x] Flat world chunks (bedrock + dirt + grass, paletted sub-chunks)
- [x] Le client se connecte et voit le monde plat !

## Phase 2 : Player basics — ✅ TERMINÉE

**Objectif :** Le joueur peut se déplacer, voir les autres joueurs, et chatter.

- [x] PlayerAuthInput (mouvement, position, rotation)
- [x] MovePlayer broadcast aux autres joueurs
- [x] TextPacket (chat bidirectionnel, format category PMMP)
- [x] CommandRequest handler (commandes via /)
- [x] AvailableCommands (tab-complete)
- [x] Player registry (entity ID unique par joueur)
- [x] AddPlayer / RemoveEntity packets
- [x] PlayerList sync (ADD/REMOVE)
- [x] Chunk loading dynamique (nouveaux chunks au déplacement)
- [x] Validation mouvement (NaN reject, void kill Y<-128)
- [x] Config serveur enrichie (gamemode, difficulty, view_distance, etc.)

## Phase 3 : Monde vivant — ✅ TERMINÉE

**Objectif :** Cycle jour/nuit, météo, commandes fonctionnelles.

- [x] World tick system (100 TPS)
- [x] Cycle jour/nuit (24000 ticks, SetTime broadcast)
- [x] Weather system (rain/thunder avec transitions smooth)
- [x] 15 commandes fonctionnelles dans crate mc-rs-command
- [x] /time set, /stop, /say, /weather, /spawn, /pos, /gamemode, /tp, /help, /list, /ping, /seed, /kill, /difficulty, /clear

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

### 3.3 - World persistence — ✅ TERMINÉE
- [x] LevelDB read/write (rusty-leveldb)
- [x] Sauvegarder les chunks modifiés (ChunkCache + dirty tracking)
- [x] Charger les chunks depuis LevelDB
- [ ] level.dat (métadonnées monde)
- [x] Auto-save périodique

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
- [x] Permission system (string permissions, defaults true/false/op, héritage via enfants)
- [x] Operator status
- [x] Ban list (player + IP)
- [x] Whitelist

### 5.4 - World generation améliorée — ✅ TERMINÉE
- [x] Normal generator (Simplex 3D noise, port PMMP)
- [x] 11 Biomes (sélection temp/rainfall, Gaussian smoothing)
- [x] Ground cover par biome (grass, sand, snow, gravel, dirt)
- [x] Ore populator (8 types, veines courbes)
- [x] Tree populator (chêne, placement par biome)
- [x] Tall grass / short grass par biome
- [x] Eau à Y=62
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
- [x] PluginManifest (plugin.yml)
- [x] Discover du dossier `plugins/` + chargement des manifests
- [x] Dependency resolution basique (`depend`, `softdepend`, `loadbefore`, ordre `STARTUP`/`POSTWORLD`)
- [x] Plugin data folder
- [x] Plugin lifecycle (load, enable, disable)
- [x] Plugin config

### 6.3 - Lua plugins
- [x] LuaPluginLoader
- [x] API Lua : lifecycle, commands, config
- [ ] API Lua : events, scheduler
- [x] Exemples de plugins

### 6.4 - Commandes avancées
- [x] Moteur de commandes serveur partagé (registre unique, dispatch joueur + console)
- [x] Command autocomplete (AvailableCommandsPacket filtré par permissions + soft enums dynamiques)
- [x] Target selectors (@a, @p, @r, @s, @e)
- [x] Commandes admin / communication / world / player principales
- [ ] Toutes les commandes vanilla restantes
- [x] Plugin commands
- [ ] Commandes gameplay restantes (xp, effect, enchant, particle)

Note 6.4 : le socle PocketMine-like est maintenant en place (permissions, visibilité par sender, sélecteurs, console locale, commandes serveur partagées, commandes plugin Lua). Il reste surtout les briques gameplay avancées et l'API plugin autour des events/scheduler.

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
- [x] Title/subtitle/actionbar
- [ ] Particles

### 7.3 - Administration
- [x] Console interactive (locale via stdin, toujours op)
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
