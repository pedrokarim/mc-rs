# 99 - Roadmap : Plan de développement

## Philosophie

Avancer **phase par phase**, chaque phase est **testable et fonctionnelle** avant de passer à la suivante. On reproduit PocketMine en Rust, pas besoin de réinventer.

---

## 🚨 PRIORITÉ ABSOLUE — Phase INV : Port intégral InventoryManager PMMP

**Bloque tout le reste depuis des semaines. À traiter dans une session dédiée qui ne s'arrête pas tant que les 10 tests E2E ne passent pas.**

**Documents de référence obligatoires :**
- [`INVENTORY-ITEMS-SYSTEM.md`](INVENTORY-ITEMS-SYSTEM.md) — état exact + plan détaillé en 6 phases (A→F)
- [`09-INVENTORY-SYSTEM.md`](09-INVENTORY-SYSTEM.md) — architecture cible

**Bugs confirmés en production (testé client Bedrock 1.26.10) :**
- [ ] **Crash client à l'ouverture inventaire** (touche E)
  - Format ContainerOpen byte-correct mais le client crash quand même
  - Cause racine : pas d'`InventoryManager`, pas de stack ID tracking, pas de two-phase sync
- [ ] **Items au sol affichés en ombre + crash après plusieurs spawn**
  - AddItemActor envoyé, item visible comme silhouette transparente
  - Crash client après plusieurs items
- [ ] **Inventaire pas affiché correctement** (hotbar vide visuellement même avec items)
- [ ] **Drop d'item depuis inventaire ignoré** (touche Q ou drag hors UI)
- [ ] **ItemStackRequest pas géré proprement** (drag/drop dans l'UI inventaire)

**Plan d'attaque (résumé — détail dans INVENTORY-ITEMS-SYSTEM.md §4) :**

### Phase A — Infrastructure
- [ ] Créer `crates/mc-rs-server/src/inventory_manager.rs` (port `InventoryManager.php`)
- [ ] Étendre `PlayerInventory` : cursor, crafting_grid_2x2, crafting_result, listeners, dirty tracking
- [ ] Refactor `ItemStackWrapper::encode` pour accepter un stack_id paramètre (retirer hardcode `1`)

### Phase B — Port InventoryManager
- [ ] `register_inventories()` (associate windowId 0/119/120/124 + ComplexWindowMap UI)
- [ ] `sync_all()`, `sync_contents()`, `sync_slot()` avec **two-phase sync**
- [ ] `send_inventory_content_packets()` clear puis real (PMMP InventoryManager.php:542)
- [ ] `send_inventory_slot_packets()` clear puis real (PMMP InventoryManager.php:511)
- [ ] `on_client_open_main_inventory()` avec `on_current_window_remove()` + `open_window_deferred()`
- [ ] `on_client_remove_window()` (handshake close ack)
- [ ] `handle_item_stack_request()` complet (Take/Place/Swap/Drop/Destroy/CraftRecipe/...)
- [ ] `ItemStackResponseBuilder` proprement construit + envoi `ItemStackResponsePacket`

### Phase C — Brancher dans Connection
- [ ] Remplacer tous les handlers dans `connection/inventory.rs` par des appels au manager
- [ ] Au spawn : `register_inventories()` puis `sync_all()`

### Phase D — Items au sol (AddItemActor)
- [ ] Vérifier `entity::item_metadata()` complet (tous les flags PMMP §3.9 INVENTORY-ITEMS-SYSTEM.md)
- [ ] Vérifier `coreItemStackToNet()` correct (block_runtime_id pour items-blocs)
- [ ] Tests : casser bloc → item visible (pas une ombre), tombe, ramassable

### Phase E — Drop depuis inventaire
- [ ] `ItemStackRequest::Drop` → spawn item entity à eye + 1.3 avec motion direction*0.4
- [ ] Legacy `InventoryTransaction::Normal` SOURCE_WORLD → spawn équivalent

### Phase F — Validation E2E
Les 10 tests doivent TOUS passer (voir INVENTORY-ITEMS-SYSTEM.md §4 Phase F).

**Règle de la session inventaire : NE PAS S'ARRÊTER tant que les 10 tests E2E ne passent pas.**

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

## Phase LIB : Bibliothèque de modules PMMP portés (2026-04-14)

**Contexte** — Session marathon d'~280 modules portés depuis PMMP. Ils compilent et sont testés (~970 tests), mais la grande majorité sont des **squelettes isolés** (struct + constantes + tests unitaires) **non branchés** dans le runtime. À traiter au fur et à mesure que les phases les requièrent.

### Modules branchés (utilisés en runtime)
- [x] `mob_hp` → source autoritaire pour `MobKind::max_health()` (60+ mobs au lieu de 7 hardcodés)
- [x] `stack_sizes` → max stack dynamique dans `PlayerInventory::add_item` (swords=1, pearls=16, stone=64)

### Modules dormants (créés mais non branchés) — Catalogue
Le code est présent dans `crates/mc-rs-server/src/`, les tests passent isolément, **mais aucun consommateur runtime** ne les appelle encore. Les brancher demandera pour chacun un travail d'intégration concret (handlers, packets, ticks).

**Combat / santé / effets**
- `arrow`, `arrow_pickup_mode`, `bow`, `crossbow`, `trident`, `mace`, `shield`, `shield_decoration`
- `thrown_potion`, `snowball`, `fireball`, `wind_charge`, `ender_pearl`, `exp_bottle`
- `damage_sources`, `splash_damage`, `entity_fall` (doublon de `survival::FallState`)
- `hunger` (doublon de `survival::HungerManager`), `food` (doublon partiel)

**Mobs (structs squelettes, pas d'AI)**
- Passifs : `pig`, `cow`, `sheep`, `chicken`, `rabbit`, `wolf`, `cat`, `ocelot`, `parrot`, `fox`, `panda`, `bee`, `turtle`, `frog`, `sniffer`, `axolotl`, `dolphin`, `squid`, `glow_squid`, `bat`, `polar_bear`, `strider`, `goat`, `camel`, `allay`, `llama`, `horse`, `fish`, `mooshroom`, `tadpole`, `armadillo`, `happy_ghast`, `tropical_fish`
- Hostiles : `zombie`, `skeleton`, `creeper`, `enderman`, `spider`, `slime`, `magma_cube`, `blaze`, `ghast`, `witch`, `drowned`, `husk`, `phantom`, `silverfish`, `endermite`, `piglin`, `piglin_brute`, `zombified_piglin`, `hoglin`, `zoglin`, `vex`, `pillager`, `ravager`, `shulker`, `stray` / `wither_skeleton` / `bogged` (via `skeleton`), `guardian`, `breeze`, `creaking`, `warden`
- Boss : `wither`, `ender_dragon`, `dragon_fight`
- Utilitaires mobs : `mob_xp`, `entity_drops`, `entity_persistence`, `despawn_rules`, `mob_cap`, `ai_states`, `slime_spawning`

**Blocs & monde**
- Mécaniques : `piston`, `dispenser`, `hopper`, `chest_system`, `shulker_box`, `barrel`, `door`, `trapdoor`, `ladder`, `vine`, `slab`, `stairs`, `block_wall`, `torch`, `glass_pane`, `scaffolding`, `dripleaf`, `pointed_dripstone`, `candle`
- Redstone : `redstone_devices`, `redstone_wire`, `repeater`, `comparator`, `observer`, `target_block`, `tripwire`, `daylight_sensor`
- Special : `bed_color`, `cauldron` (déjà existant), `lectern_book`, `chiseled_bookshelf`, `decorated_pot`, `flower_pot`, `sign_text`, `item_frame`, `armor_stand`, `vault`, `trial_spawner`, `trial_loot`, `command_blocks`, `structure_block`, `jigsaw`, `crafter`, `spawner`, `magma_block`, `bubble_column`, `soul_fire`, `sculk_vein`, `sculk_sensor`, `honey_block`, `slime_block`, `dragon_egg`, `end_portal`, `obsidian_pillar`, `end_gateway`, `nether_portal_spawn`, `lightning`, `waterlogging`
- Croissance : `crop_growth`, `sapling_growth`, `leaves_decay`, `cocoa_beans`, `sugar_cane`, `bamboo`, `kelp`, `coral`, `sea_pickle`, `turtle_egg`, `frog_spawn`, `beehive`, `azalea`, `cave_vines`, `glow_berry`, `chorus_fruit`, `torchflower`, `pitcher_plant`, `spore_blossom`, `amethyst`, `amethyst_resonance`, `copper_oxidation`, `deepslate`, `ancient_debris`, `netherite_scrap`, `mushroom_biome`, `snow_layer`, `powdered_snow`, `ice_melt`, `fire_spread`, `fire_mechanics`

**Monde / biomes / structures**
- Biomes : `biome_color`, `nether_biomes`, `mangrove_swamp`, `cherry_grove`, `pale_garden`, `badlands_biome`, `lush_caves`, `dripstone_caves`, `deep_dark`, `bamboo_jungle`
- Structures : `village_structures`, `ancient_city`, `trial_chamber`, `end_islands`, `end_city`, `bastion_remnant`, `buried_treasure`, `shipwreck`, `ruined_portal`, `nether_fortress`, `ocean_monument`, `stronghold`, `woodland_mansion`, `witch_hut`, `jungle_temple`, `igloo`, `pillager_outpost`, `ocean_ruin`, `abandoned_mineshaft`
- Autres : `chunk_ticket`, `world_border_damage`, `spawn_chunks`, `level_db_keys`, `chunk_serializer_formats`, `chunk_radius`, `chunk_request`, `light_level`, `block_light_update`, `sky_light_update`, `tick_speed`, `random_tick`, `night_time`, `weather_state`, `world_events_map`, `world_time`

**Items & crafting**
- `furnace_fuel`, `furnace_recipes`, `crafting_recipes`, `campfire_recipes`, `smithing`, `smoker`, `stonecutter`, `grindstone`, `cartography`, `fletching`, `loom`, `workbench`, `inventory_2x2`, `stack_sizes` (✅), `item_stack_merge`, `inventory_drag`, `dropping_items`, `container_types`, `held_item`, `item_cooldown`, `bundle`, `written_book`, `enchanted_books`, `banner_blocks`, `banner_color`, `banner_pattern`, `trim_patterns`, `dye_colors`, `dye_recipes`, `armor_dye`, `armor_tier`, `armor_stand`, `map_rendering`, `music_disc`, `compass`, `spyglass`, `carrot_on_stick`, `saddles`, `fishing_rod`, `fishing_loot_table`, `loot_chest`, `trap_chest_loot`, `stray_loot`, `wither_skeleton_loot`, `blaze_loot`, `suspicious_sand`, `wolf_armor`, `ominous_item`, `heavy_core`, `breeze_rod`, `piglin_bartering`, `zombie_villager`, `villager`, `villager_gossip`, `village_trades_detailed`, `wandering_trader`, `trial_chamber`, `totem`, `elytra`, `leads`, `name_tag`, `food`, `potions_brewing`, `golem_crafting`, `spawn_egg_list`, `drop_xp`, `tool_types`, `block_break`, `block_pickaxe`, `block_entities_map`, `block_comparison`, `mining_effect`, `smoke_pillar`

**Infrastructure serveur**
- `whitelist`, `ops_list`, `rcon`, `query_protocol`, `server_settings`, `server_log`, `console_commands`, `datapack`, `pack_encoder`, `crash_report`, `signals`, `timings`, `async_task_pool`, `packet_loss`, `packet_compression`, `keepalive`, `ping_system`, `chat_cooldown`, `chat_filter`, `chatchannels`, `uuid_utils`, `language`, `locale_messages`, `text_colors`, `custom_nameplate`, `animation_defs`, `velocity_broadcast`, `respawn_system`, `player_list_packet`, `adventure_mode`, `spectator_mode`, `creative_inventory`, `tab_list`, `sidebar`, `title_packet`, `boss_bar`, `tag_system`, `team`, `pathfinder`, `permissions`, `scheduler_api`, `recipe_unlock`, `achievements`, `advancement_tree`, `bad_omen`, `custom_commands`, `command_selectors`, `scoreboard_crit`, `player_stats`, `xp`, `xp_sharing`, `hunger`, `entity_fall`, `entity_ids` (doublon), `position`, `vector_math`, `raytrace`, `rotation`, `movement_control`, `bounding_box`, `nbt_tags_ext`, `sounds_library`, `config_loader`, `binary_stream`, `player_abilities`, `damage_sources`, `sprinting`, `beacon_effects`, `conduit_power`, `anvil_damage`, `enchant_table`, `player_inventory_slots`, `block_breaking_progress`, `trade_offer_packet`, `tnt`, `end_crystal`, `falling_block`, `experience_orb`, `area_effect_cloud`, `cat_gift`, `weather_damage`, `strider`, `tnt_primed_source`, `mob_xp_drop_on_player_kill`, `motion`, `powder_snow_bucket`

### État réel
- **Compile** : ✅ `cargo build --release` clean
- **Tests** : ✅ 966 passent (unitaires uniquement, pas d'intégration)
- **Utilisable en prod** : Seule une fraction — voir phases 1-3 terminées. Le reste est de la doc structurée en code.

### Règle pour la suite
Quand une fonctionnalité doit être implémentée :
1. Chercher d'abord un module existant dans le catalogue ci-dessus
2. Si présent → brancher proprement dans le consommateur (handler/tick/packet)
3. Si absent → voir PMMP source, créer un port branché directement (pas de squelette isolé)
4. Retirer de ce catalogue à chaque branchement.

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
