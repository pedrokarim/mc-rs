# 99 - Roadmap : Plan de développement

## Philosophie

Avancer **phase par phase**, chaque phase est **testable et fonctionnelle** avant de passer à la suivante. On reproduit PocketMine en Rust, pas besoin de réinventer.

---

## Phase INV : Port intégral InventoryManager PMMP — ✅ TERMINÉE

**Documents de référence :**
- [`INVENTORY-ITEMS-SYSTEM.md`](INVENTORY-ITEMS-SYSTEM.md) — état exact + plan détaillé en 6 phases (A→F)
- [`09-INVENTORY-SYSTEM.md`](09-INVENTORY-SYSTEM.md) — architecture cible

**Bugs résolus :**
- [x] **Crash client à l'ouverture inventaire** (touche E) — résolu via two-phase sync + ContainerOpen PMMP-strict + UPDATE_ABILITIES = 0xBB (bug protocol 944)
- [x] **Items au sol** : AddItemActor envoyé avec metadata complète, plus de crash multi-spawn
- [x] **Inventaire affiché correctement** — sync_all itère toutes les keys + two-phase
- [x] **Drop d'item** : ItemStackRequest::Drop branché, spawn item entity avec scatter
- [x] **ItemStackRequest** : Take/Place/Swap/Drop/Destroy/MineBlock/CraftCreative/CraftRecipe/CraftRecipeAuto tous gérés

**Réalisé** (résumé — détail dans INVENTORY-ITEMS-SYSTEM.md §4) :

### Phase A — Infrastructure ✅
- [x] `crates/mc-rs-server/src/inventory_manager.rs` (port `InventoryManager.php`)
- [x] `PlayerInventory` étendu : cursor, crafting_grid_2x2 + 3x3, anvil_input/material, enchant_input/material, block_container[27]
- [x] `ItemStackWrapper::encode` accepte stack_id paramètre

### Phase B — Port InventoryManager ✅
- [x] `register_inventories()` (associate windowId 0/119/120/124 + ComplexWindowMap UI)
- [x] `sync_all()`, `sync_contents()`, `sync_slot()` avec **two-phase sync**
- [x] `send_inventory_content_packets()` clear puis real (PMMP InventoryManager.php:542)
- [x] `send_inventory_slot_packets()` clear puis real (PMMP InventoryManager.php:511)
- [x] `on_client_open_main_inventory()` avec `on_current_window_remove()` + `open_window_deferred()`
- [x] `on_client_remove_window()` (handshake close ack)
- [x] `handle_item_stack_request()` complet (Take/Place/Swap/Drop/Destroy/CraftRecipe/CraftRecipeAuto/CraftCreative/MineBlock)
- [x] `ItemStackResponseBuilder` + envoi `ItemStackResponsePacket`

### Phase C — Brancher dans Connection ✅
- [x] Tous les handlers dans `connection/inventory.rs` appellent le manager
- [x] Au spawn : `register_inventories()` puis `sync_all()`

### Phase D — Items au sol (AddItemActor) ✅
- [x] `entity::item_metadata()` complet (flags PMMP)
- [x] `coreItemStackToNet()` correct (block_runtime_id pour items-blocs)
- [x] Casser bloc → item visible, tombe, ramassable (pickup delay 0.5s)

### Phase E — Drop depuis inventaire ✅
- [x] `ItemStackRequest::Drop` → spawn item entity avec motion direction*0.4
- [x] Legacy `InventoryTransaction::Normal` SOURCE_WORLD → spawn équivalent

### Phase F — Validation E2E
Tests E2E manuels validés avec client Bedrock 1.26.10 (test playtest 2026-04-21).

### Phase G — Block container UIs (post-Phase F) ✅
- [x] Chest UI : ContainerOpen + InventoryContent + ItemStackRequest routing via `InvKey::BlockContainer` + sync vers `ChestManager` partagé chaque tick
- [x] Crafting Table 3x3 : InvKey::Craft3x3 (UI 32..40), CraftRecipe action 12 + Auto 13, RECIPE_DB static avec 1601 recettes
- [x] Anvil UI : InvKey::AnvilInput + AnvilMaterial, ContainerOpen ANVIL=5
- [x] Enchanting Table UI : InvKey::EnchantInput + EnchantMaterial, ContainerOpen ENCHANTMENT=3
- [x] Sign edit : BlockActorData (0x38) decoder + SignManager + broadcast NBT

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
- [x] **IA des mobs** → framework générique `crate::ai::` branché dans la game loop
  (sensors → behaviors priorisés → controllers, navigation A\* au sol). Hostiles
  zombie/skeleton/creeper traquent et combattent le joueur ; passifs errent et fuient.
  Réf = Allay. **Doc dédiée : [`18-MOB-AI.md`](18-MOB-AI.md)**. Impacts sur ce catalogue :
  - `ai_states` → **supprimé** (remplacé par `ai/`).
  - `pathfinder` → **remplacé** par `ai/route.rs` (l'ancien `pathfinder.rs` n'est plus utilisé).
  - `arrow` → un **nouveau** `arrow_entity.rs` (projectile vivant) a été créé pour le tir du
    squelette ; le module dormant `arrow.rs` (modèle de données) reste non branché.
  - Mobs `zombie`/`skeleton`/`creeper` : restent des structs de données, mais leur **IA est
    désormais active** via le framework générique sur `mob_entities::MobEntity`.

### Modules dormants (créés mais non branchés) — Catalogue
> ⚠️ Les entrées **Mobs**, `ai_states`, `pathfinder` et `arrow` ci-dessous sont partiellement
> périmées : voir la note « IA des mobs » dans *Modules branchés*.
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

## Phase DATA : Import complet bedrock-samples Mojang (2026-04-21) — ✅ TERMINÉE

**Objectif :** Remplacer toutes les tables hardcodées par les **données canoniques Mojang** (bedrock-samples 1.26.10.4) et PMMP (bedrock-data). Voir [`30-VANILLA-DATA-IMPORT.md`](30-VANILLA-DATA-IMPORT.md) pour le détail complet.

### Modules data-driven créés (branchés runtime)
- [x] `creative_content` — 4 catégories + ~150 sous-groupes (PMMP bedrock-data) → envoyé dans CreativeContent au PreSpawn
- [x] `recipes_vanilla` — 1601 recettes (939 shaped + 513 shapeless + 149 furnace) → registrées au boot dans `CraftingManager`

### Modules data-driven créés (data-only, consommateurs runtime à venir)
- [x] `loot_table` — 176 tables (122 mobs + 29 chests + 25 equipment/gameplay/dispensers/pots/spawners) avec parser pools/conditions/functions
- [x] `spawn_rules_vanilla` — 56 règles de spawn mobs (brightness, surface, weight, population_control)
- [x] `biomes_vanilla` — 87 biomes vanilla (temperature, downfall, surface materials, tags)
- [x] `trading_vanilla` — 24 trades (villagers + piglins + hero_of_the_village)
- [x] `entities_vanilla` — 126 mobs (runtime_identifier, family, health, spawnable, summonable)
- [x] `items_vanilla` — 77 items Bedrock data-driven (nourriture + bundles, nutrition/saturation/tags)
- [x] `vanilla_registries` — registres de noms canoniques (37 effects + 42 enchants + 47 potions + 3 dims)

### Consommateurs runtime à brancher
- [ ] **Loot tables mobs** : quand un mob meurt, appeler `loot_table::roll_entity_loot()` au lieu des fonctions hardcodées de `entity_drops.rs` (obsolète)
- [ ] **Loot tables chests** : quand on génère un donjon / ancient_city / bastion, remplir le chest avec `loot_table::roll_chest_loot(kind)`
- [ ] **Spawn naturel** : implémenter un spawner runtime qui consulte `spawn_rules_vanilla` (biome → weight par mob → light check → surface/underground → spawn)
- [ ] **Biomes étendus** : le terrain generator utilise encore 11 biomes hardcodés, migrer vers `biomes_vanilla` (top_material/tags) pour passer aux 87
- [ ] **Villager trading** : quand un villager a un profession, utiliser `trading_vanilla::for_profession(name)` pour peupler son TradeOffersPacket
- [ ] **Validation /effect et /enchant** : tab-complete + check arg contre `vanilla_registries::is_effect/is_enchantment`
- [ ] **Entity spawn factory** : quand on `/summon X`, lire `entities_vanilla::health(x)` / `family(x)` pour initialiser le mob
- [ ] **Food system** : `items_vanilla::nutrition/saturation` pour `PlayerHungerManager::eat`

### Compteurs
- **Tests** : 998 passent (record précédent : ~985)
- **Fichiers data ajoutés** : 15 JSON consolidés dans `crates/mc-rs-server/data/{creative,loot_tables,recipes,vanilla}/`
- **Commits** : 8 feat commits (creative, loot entities, recipes, loot chests, spawn_rules, biomes, data-final)

---

## Phase 4 : Entities & Combat — ✅ TERMINÉE

**Objectif :** Entités, dégâts, mort, respawn.

### 4.1 - Système d'entités
- [x] Entity base (position, vélocité, hitbox) — `entity.rs`, `mob_entities.rs`, `item_entities.rs`
- [x] Entity spawn/despawn (AddActorPacket, RemoveActorPacket, AddItemActor)
- [x] Entity movement (MoveActorAbsolutePacket avec gravity + drag PMMP)
- [x] Entity metadata sync (SetActorData avec entity_flags)

### 4.2 - Combat
- [x] Dégâts PvP (`combat.rs::attack_entity`)
- [x] Knockback (vecteur attaquant→target + KNOCKBACK_RESISTANCE)
- [x] Invincibilité frames (`no_damage_ticks`)
- [x] Mort + respawn (death animation + Respawn packet + ready handshake)
- [x] Effets de potion (MobEffectPacket 0x1C, /effect avec parsing nom/id)

### 4.3 - Mobs passifs
- [x] ItemEntity (items droppés, gravity 0.04, drag 0.98, friction 0.6 ground, scaling 100→20 TPS)
- [x] ExperienceOrb (`passive_entities.rs`)
- [x] FallingBlock (`passive_entities.rs`)
- [x] PrimedTNT (fuse 80 ticks)

### 4.4 - Attributs & effets
- [x] AttributeMap (port `AttributeMap.php`, drain_desync, 6 floats protocol 944)
- [x] EffectManager (`effects.rs` + EffectKind 30 effects + from_name_or_id + apply via MobEffectPacket)
- [x] HungerManager (exhaustion walk=0.005, sprint=0.1, swim=0.015 PMMP-exact)
- [x] ExperienceManager (formule PMMP : <16 → 2L+7, <31 → 5L-38, ≥31 → 9L-158)

### 4.5 - Mob spawning naturel — ✅ TERMINÉE
- [x] mob_spawner.rs : tick global 20 TPS, autour de chaque joueur
- [x] Use spawn_rules_vanilla::spawn_weight pour pondérer (56 mobs vanilla)
- [x] Cap par catégorie (12 hostile + 6 passive par player)
- [x] Monster gate sur is_night (world_time 13000-23000)
- [x] Headroom + is_solid_support check pour valider la position

### 4.6 - Loot tables vanilla — ✅ TERMINÉE
- [x] mob_entities::apply_attack utilise loot_table::roll_entity_loot
- [x] 122 mob loot tables vanilla (bedrock-samples 1.26.10.4)
- [x] 29 chest loot tables (dungeon, ancient_city, bastion, etc.)
- [x] Pools + conditions (killed_by_player, is_baby, on_fire, random_chance*)
- [x] Functions (set_count, looting_enchant)

**Résultat Phase 4 :** Combat fonctionnel + mobs basiques + spawn naturel + loot vanilla.

---

## Phase 5 : Game systems — ✅ TERMINÉE

**Objectif :** Les systèmes de jeu complets.

### 5.1 - Crafting — ✅ TERMINÉE
- [x] CraftingManager (`crafting.rs` + RECIPE_DB static OnceLock)
- [x] ShapedRecipe + ShapelessRecipe + FurnaceRecipe
- [x] **1601 recettes vanilla** chargées au boot via `recipes_vanilla::register_all`
  - 939 shaped + 513 shapeless + 149 furnace
- [x] Crafting table 3x3 (InvKey::Craft3x3, UI slots 32..40, ContainerOpen WORKBENCH)
- [x] Inventory crafting 2x2 (InvKey::Craft2x2, UI slots 28..31)
- [x] CraftRecipe action (12) + CraftRecipeAuto (13) parsés et matchés contre RECIPE_DB
- [x] FurnaceRecipe (cuisson, FurnaceManager 20 TPS, register/unregister sur place/break)
- [x] 10 vanilla item tags résolus (planks, logs, wool, metal_nuggets, etc.)

### 5.2 - Enchantements — ✅ TERMINÉE
- [x] Table d'enchantement (InvKey::EnchantInput + EnchantMaterial, ContainerOpen ENCHANTMENT=3)
- [x] Anvil (InvKey::AnvilInput + AnvilMaterial, ContainerOpen ANVIL=5)
- [x] EnchantmentKind 38 enchants vanilla + from_name_or_id + max_level + incompatibilités
- [x] /enchant <target> <name|id> [level] applique via NBT compound `ench` dans extra_data
- [x] enchantments::build_extra_data_with_enchant (NBT LE write via mc-rs-nbt + format
      PMMP `ItemStackExtraData` : marker 0xFFFF + version 1 + nbt + canPlace/canDestroy)

### 5.3 - Permissions — ✅ TERMINÉE
- [x] Permission system (string permissions, defaults true/false/op, héritage via enfants)
- [x] Operator status
- [x] Ban list (player + IP)
- [x] Whitelist

### 5.4 - World generation améliorée — ✅ TERMINÉE
- [x] Normal generator (Simplex 3D noise, port PMMP)
- [x] 11 Biomes (sélection temp/rainfall, Gaussian smoothing)
- [x] **87 biomes vanilla data prêts** (`biomes_vanilla.rs` chargé depuis bedrock-samples)
- [x] biome_identifier(numeric) → "minecraft:xxx" mapping (73 IDs Bedrock)
- [x] vanilla_data_for(id) → top_material/temperature/downfall/tags accessibles
- [x] Ground cover par biome (grass, sand, snow, gravel, dirt)
- [x] Ore populator (8 types, veines courbes)
- [x] Tree populator (chêne, placement par biome)
- [x] Tall grass / short grass par biome
- [x] Bambou (jungle/bamboo_jungle)
- [x] Eau à Y=62
- [x] level.dat persistence (`level_dat.rs` save/load JSON)
- [ ] Caves
- [ ] Structures basiques (générateur structures à compléter — données loot_tables prêtes)

### 5.5 - Block entities — ✅ TERMINÉE
- [x] Furnace (FurnaceManager + tick 20 TPS, register/unregister)
- [x] Chest (chest_storage::ChestManager partagé, 27 slots,
      ContainerOpen + InventoryContent au right-click + ItemStackRequest routing
      via InvKey::BlockContainer + sync vers ChestManager partagé)
- [x] Sign (sign_storage::SignManager + parse_sign_nbt, BlockActorData 0x38
      reçu/persisté/broadcast)
- [x] Bed (spawn override au right-click)

**Résultat Phase 5 :** Serveur de survie complet avec crafting/enchant/UIs/blocks entities.

---

## Phase 6 : Plugin system — ✅ TERMINÉE

**Objectif :** Les plugins peuvent étendre le serveur.

### 6.1 - Event system — ✅ TERMINÉE
- [x] EventManager (`event/`, dispatch, priorités, Cancellable trait)
- [x] Événements joueur (join, quit, chat, move, interact, drop, item_held, xp_change)
- [x] Événements bloc (BlockBreakEvent, BlockPlaceEvent, BlockUpdateEvent, BlockGrowEvent)
- [x] Événements entité (EntityDamageEvent, EntityDeathEvent, EntitySpawnEvent, EntityDespawnEvent)
- [x] Événements serveur (ServerStartEvent, DataPacketSendEvent, DataPacketReceiveEvent)

### 6.2 - Plugin API — ✅ TERMINÉE
- [x] PluginManifest (plugin.yml)
- [x] Discover du dossier `plugins/` + chargement des manifests
- [x] Dependency resolution basique (`depend`, `softdepend`, `loadbefore`, ordre `STARTUP`/`POSTWORLD`)
- [x] Plugin data folder
- [x] Plugin lifecycle (load, enable, disable)
- [x] Plugin config

### 6.3 - Lua plugins — ✅ TERMINÉE
- [x] LuaPluginLoader
- [x] API Lua : lifecycle, commands, config
- [x] API Lua : events (`register_event(name, fn)`) + scheduler (`schedule_after(ticks, fn)`)
- [x] PluginRuntime gagne tick_counter / scheduled_tasks / event_handlers
- [x] tick_scheduler() appelé chaque server tick (100 TPS)
- [x] Exemples de plugins

### 6.4 - Commandes — ✅ TERMINÉE (62 commandes, parité PMMP + extension vanilla)

**Infrastructure**
- [x] Moteur de commandes serveur partagé (registre unique, dispatch joueur + console)
- [x] Command autocomplete (AvailableCommandsPacket filtré par permissions + soft enums dynamiques)
- [x] Target selectors (@a, @p, @r, @s, @e)
- [x] Plugin commands
- [x] Permissions via PermissionRegistry + PermissionDefault::True|Op

**Commandes implémentées (62) — vanilla Bedrock + extras**

*Parité PMMP (40)* : help, version, plugins, status, stop, save/save-on/save-off,
gc, dumpmemory, timings, list, say, me, tell, kick, op, deop, whitelist, ban,
ban-ip, banlist, pardon, pardon-ip, gamemode, tp, kill, clear, give, summon,
spawnpoint, setworldspawn, time, difficulty, defaultgamemode, seed, weather,
xp, title, transferserver.

*Extension Bedrock (4)* : /effect, /enchant, /particle, /boss, /scoreboard.

*Build/admin (18) ajoutées dans le sprint Phase 7+* :
- [x] /gamerule [<rule> [<value>]] — 33 game rules vanilla, broadcast GameRulesChangedPacket
- [x] /setblock <x y z> <block> [destroy|keep|replace]
- [x] /fill <x1 y1 z1> <x2 y2 z2> <block> [destroy|hollow|keep|outline|replace] — limite 32768
- [x] /clone <x1 y1 z1> <x2 y2 z2> <dx dy dz> [masked|replace] [force|move|normal]
- [x] /tellraw <target> <json> — TextPacket type=10 JSON_WHISPER
- [x] /playsound <sound> [target] [x y z] [volume] [pitch] — PlaySoundPacket 0x56
- [x] /stopsound <target> [sound] — StopSoundPacket 0x57
- [x] /replaceitem entity <target> <slot_type> [slot] <item> [count]
- [x] /tag <target> <add|remove|list> [<tag>] — volatile (HashSet sur Connection)
- [x] /loot spawn|give — wire vers loot_table::roll_chest_loot
- [x] /damage <target> <amount> — via combat::attack_entity DamageCause::Custom
- [x] /event entity <target> <event|id> — ActorEventPacket (hurt/death/eating…)
- [x] /testfor <target> — compte les entités matchant le selector
- [x] /testforblock <x y z> <block> — match exact par network_id
- [x] /spreadplayers <cx cz> <spreadDist> <maxRange> <target> — random dans le carré
- [x] /locate <structure> — approximation grid-based via average_separation_chunks
- [x] /reload — recharge ops/whitelist/bans + resync permissions
- [x] /ability <target> <mayfly|mute|worldbuilder|...> <true|false>
- [x] /music play|stop|volume <track> [volume] — wrapper PlaySound

### 6.5 - Commandes vanilla Bedrock non implémentées (faible valeur / gros effort)

Ces commandes nécessitent chacune un système non-trivial à implémenter. Faible
priorité car peu utilisées en pratique.

- [ ] **/execute** — parser de sub-commands (as/at/positioned/if/unless/store/run)
      Très complexe : modifie la chaîne de contexte du sender pour chaque sous-clause.
- [ ] **/function** — exécute un fichier `.mcfunction` (séquence de commandes)
      Nécessite un système de chargement de packs comportements + parser.
- [ ] **/schedule** — différer l'exécution d'une /function de N ticks
      Repose sur /function.
- [ ] **/scriptevent** — déclenche un event scripting JS/TS
      Nécessite l'API Bedrock scripting (V8/QuickJS embedding).
- [ ] **/camera** — presets de caméra cinématique (Bedrock 1.20+)
      Nécessite CameraPresetsPacket + CameraInstructionPacket.
- [ ] **/dialogue** — UI NPC dialogue
      Nécessite NpcDialoguePacket + dialogue scene JSON.
- [ ] **/playanimation** — joue une animation custom sur une entité
      Nécessite AnimateEntityPacket + animation registry.
- [ ] **/structure save|load|delete** — manipule des structures NBT
      Nécessite StructureBlockUpdatePacket + format .mcstructure.
- [ ] **/tickingarea add|remove|list** — zones de chunks toujours simulés
      Nécessite système TickingArea persisté.

**Limitations connues sur les commandes implémentées :**
- `/setblock destroy` ne fait pas drop d'item (TODO : appeler spawn_world_item_entity)
- `/fill` et `/clone` font N broadcasts UpdateBlock (peut spammer pour grosses régions)
- `/clone` : modes `filtered`/`force` non implémentés
- `/tag` : volatile (perdu à la déco), pas branché aux selectors `@e[tag=foo]`, joueurs seulement
- `/spreadplayers` : pas de respect strict du `spreadDist` minimum entre joueurs
- `/locate` : approximation grid-based, pas un scan réel des chunks générés
- `/reload` : ne recharge ni server.toml ni les plugins déjà chargés
- `/ability` : non persistant — reset au prochain changement de gamemode
- `/music` : pas de queue/fade comme vanilla, juste play/stop
- `/damage` : ignore le paramètre `cause` (toujours `DamageCause::Custom`)

**Tests** : 17 tests dans `commands::tests`, dont 11 fonctionnels end-to-end
(world blocks + gamerules + tags simulés dans TestRuntime).

**Résultat Phase 6 :** Serveur extensible via plugins Lua avec scheduler + events.

---

## Phase 7 : Polish & Performance

**Objectif :** Prêt pour la production.

### 7.1 - Performance — ✅ TERMINÉE (parité PMMP)
- [x] **Chunk caching serveur** (parité PMMP `ChunkCache.php`) — `cached_zlib_batch`
      par `ChunkColumn`, invalidé sur `set_block`. N joueurs sur même chunk = 1
      compression Zlib au lieu de N. PMMP ne fait pas le ClientCacheStatus
      protocol non plus (cacheEnabled=false partout dans ChunkRequestTask.php),
      donc on est en parité complète.
- [x] **Packet batching multi-pkt** — Connection.broadcasts: Vec<(u32, Vec<u8>)>
      coalescé en UN seul batch shared par algo dans la main loop.
- [x] **Compression / shared batch** — `encode_shared_batch()` Zlib une fois +
      `prepare_for_send` per-conn (encryption). Dump pkt_sent.log gated par
      env `MCRS_DUMP_PACKETS=1` (était I/O sync à chaque paquet).
- [x] **Entity culling par distance** — `entity_culling.rs` + `visible_entities`
      HashSet par Connection, scan périodique 5 Hz pour transitions stationnaires.
      Add/Remove envoyés uniquement sur passage de frontière de vue.
- [x] **View distance dynamique** — `handle_request_chunk_radius_ingame` route
      RequestChunkRadius en InGame state, re-queue chunks via `order_chunks`.

### 7.2 - Fonctionnalités avancées
- [x] Resource packs delivery state machine (RESOURCE_PACK_DATA_INFO 0x52 + CHUNK_DATA 0x53
      + CHUNK_REQUEST 0x54 packets, handle_resource_pack_chunk_request hooked, ClientResponse
      avec parse UUID list. resource_pack.rs : load_pack/discover_packs/chunk_pack +
      SHA256). Pas de packs côté serveur en config par défaut → fall-through HAVE_ALL_PACKS.
- [x] **Skins custom** — `Skin::from_client_data` parse 17 champs JWT (PMMP
      LoginPacketHandler), SerializedSkin wire-format dans mc-rs-proto, champ
      `skin: Option<Skin>` dans Connection, broadcast au PreSpawn + join.
- [x] Scoreboard high-level API (`/scoreboard` + `ScoreboardManager` partagé via
      ServerState.scoreboards Arc<Mutex>)
- [x] Boss bar high-level API (`/boss` + visuals::boss_show/hide/update)
- [x] Title/subtitle/actionbar
- [x] Particles (`/particle <name> [x y z]` via SpawnParticleEffect 0x76)

### 7.3 - Administration
- [x] Console interactive (locale via stdin, toujours op)
- [x] RCON Source-format complet (`rcon.rs` : encode_packet/decode_packet + serveur
      TCP threadé avec auth Login + dispatch Command via mpsc channel + 2s timeout
      par command). À wirer dans main.rs avec config.rcon.password.
- [x] Query Gamespy v4 (`query_protocol.rs` : handshake type=9 challenge token TTL 30s
      + stat type=0 basic/full selon padding). À wirer.
- [x] Server status (motd / players_online / players_max accessible via ServerState)
- [ ] Timings / profiling (module `timings.rs` existe en stub)

### 7.4 - Xbox Live auth — ✅ TERMINÉE
- [x] Validation JWT complète (`mc-rs-crypto/jwt.rs::verify_chain` ECDSA P-384 / SHA-384,
      vérifie chaque JWT signé par identityPublicKey du précédent ou x5u du header
      pour le premier — port `XboxAuthJwt::validateLoginJwt` PMMP)
- [x] MOJANG_ROOT_PUBLIC_KEY_B64 constante exposée
- [x] XUID tracking (extrait de cpk JWT au login, stocké dans Connection.xuid)

**Résultat Phase 7 :** Polish quasi-complet — reste perf optimisations + skins.

---

## Phase DATA : Import bedrock-samples Mojang (2026-04-21) — ✅ TERMINÉE

Voir [`30-VANILLA-DATA-IMPORT.md`](30-VANILLA-DATA-IMPORT.md). Tous les data consumers
listés en "à brancher" sont maintenant **branchés** :
- [x] **Loot tables mobs** : wired via mob_entities::apply_attack
- [x] **Loot tables chests** : roll_chest_loot exposé pour structures (data prête)
- [x] **Spawn naturel** : `mob_spawner.rs` consulte spawn_rules_vanilla
- [x] **Biomes vanilla** : data accessible via biomes_vanilla::for_biome (87 biomes)
- [ ] **Villager trading UI** : data prête (`trading_vanilla.rs` 24 trades), UI à wirer
- [x] **Validation /effect /enchant** : via vanilla_registries::is_effect/is_enchantment +
      EffectKind/EnchantmentKind from_name_or_id
- [ ] **Entity spawn factory /summon** : entities_vanilla::health/family/spawnable disponible,
      pas encore wiré dans /summon
- [x] **Food system** : items_vanilla::nutrition/saturation utilisé dans handle_consume_item

---

## Résumé des phases

| Phase | Description | État |
|---|---|---|
| 1 | Foundation (réseau + login + monde plat) | ✅ Terminée |
| 2 | Player basics (mouvement, chat, inventaire) | ✅ Terminée |
| 3 | World interaction (blocs, persistence) | ✅ Terminée |
| 4 | Entities & Combat (mobs, AI base, spawn naturel, loot) | ✅ Terminée |
| 5 | Game systems (crafting, enchant, worldgen, block entities) | ✅ Terminée |
| 6 | Plugin system (Lua events + scheduler) | ✅ Terminée |
| 7 | Polish & Performance | ✅ Terminée (entity culling + shared batch + chunk cache PMMP) |
| DATA | Import bedrock-samples Mojang 1.26.10.4 | ✅ Terminée |
| INV | Port intégral InventoryManager PMMP | ✅ Terminée + chest/crafting/anvil routing |

**Tests** : 1024 passent. **Build** : `cargo build --release -p mc-rs-server` clean.

**62 commandes** registered (parité PMMP + extras Bedrock). 9 commandes
vanilla restantes non implémentées (execute/function/schedule/scriptevent/
camera/dialogue/playanimation/structure/tickingarea) — chacune nécessite un
système majeur, voir §6.5 pour le détail.
