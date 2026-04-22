# Fonctionnalités de `old_crates/` absentes de `crates/`

Inventaire de tout ce que l'ancien code implémentait et qui n'a **pas** été repris dans le nouveau. Utile pour planifier les portages futurs.

---

## 1. Crates entières disparues

| Crate ancien | Taille / contenu | Remplacement nouveau |
|---|---|---|
| `mc-rs-behavior-pack` | Loader `.mcpack`, loot tables, comportements custom | **Aucun** — loot tables hardcodées dans `crates/mc-rs-server/src/*.rs` |
| `mc-rs-game` | 18 fichiers, ~250 Ko (combat, enchanting, recipes, inventory, XP, smelting, anvil, grindstone, loom, block_entity, projectile, breeding, AI, game_world) | Éclaté dans `crates/mc-rs-server/src/` mais **incomplet** (voir ci-dessous) |
| `mc-rs-plugin-api` | API events stable (`PluginEvent`, `PluginPlayer`, `DamageCause`, `EventResult`) | **Aucun** — `event/` interne uniquement |
| `mc-rs-plugin-lua` | Runtime Lua (plugins scriptés) | **Aucun** (crate `mlua` présente dans `Cargo.toml` mais inutilisée) |
| `mc-rs-plugin-wasm` | Runtime WASM (plugins compilés) | **Aucun** |

---

## 2. Fichiers `mc-rs-world/` (ancien) disparus

Le nouveau `crates/mc-rs-server/src/world/` est bien plus léger. Modules absents :

| Fichier ancien | Taille | Ce qu'il fait | Dans le nouveau ? |
|---|---|---|---|
| `bds_compat.rs` | 28 Ko | Compatibilité Bedrock Dedicated Server (format chunks/level.dat BDS) | **Non** |
| `block_hash.rs` | 64 Ko | Hash FNV des block states pour dédup réseau | **Non** |
| `block_registry.rs` | 49 Ko | Registre complet des blocs vanilla | Partiel dans `block_registry.rs` + `block_registry_data.rs` |
| `block_state_registry.rs` | 28 Ko | Registre des block states (propriétés) | **Non** |
| `block_tick.rs` | 15 Ko | `TickScheduler` — random ticks + scheduled ticks (croissance, feu, glace, feu propagation) | **Non** |
| `chunk.rs` | 6 Ko | `ChunkColumn`, `OVERWORLD_SUB_CHUNK_COUNT`, `OVERWORLD_MIN_Y` | Partiel via `chunk_cache.rs` |
| `end_generator.rs` | 12 Ko | Génération dimension End | **Non** |
| `nether_generator.rs` | 17 Ko | Génération dimension Nether | **Non** |
| `overworld_generator.rs` | 44 Ko | Ancienne génération 2D (biome → heightmap) | Remplacé par `terrain_generator.rs` (Simplex 3D) |
| `fluid.rs` | 24 Ko | Simulation eau/lave (écoulement, source, évaporation) | **Non** |
| `gravity.rs` | 6 Ko | Chute des blocs (sand, gravel, anvil) | **Non** |
| `physics.rs` | 11 Ko | `PlayerAabb`, `ViolationTracker`, constantes anti-cheat (`BLOCK_REACH`, `MAX_AIRBORNE_KICK`, `MIN_ATTACK_INTERVAL`, `MIN_BREAK_INTERVAL`, `MIN_PLACE_INTERVAL`, `VIOLATION_DECAY_INTERVAL`) | **Non** |
| `piston.rs` | 17 Ko | Logique pistons (push/pull, sticky, slime block) | **Non** |
| `redstone.rs` | 28 Ko | Propagation redstone (wire, repeater, comparator, torch, lever, button, piston activation) | **Non** |
| `item_registry.rs` | 14 Ko | Registre items custom | **Non** |
| `network_runtime_ids.rs` | 4 Ko | Mapping block state → runtime ID réseau | **Non** |
| `serializer.rs` | 20 Ko | Sérialisation chunks vers LevelDB format BDS | Partiel via `chunk_serializer.rs` |
| `storage.rs` | 22 Ko | `LevelDbProvider` (CRUD chunks + block entities) | Partiel via `storage.rs` |
| `noise.rs` | 12 Ko | Simplex noise full (plus riche que le nouveau) | Réduit dans `noise.rs` |

---

## 3. Fichiers `mc-rs-game/` (ancien) disparus

| Fichier ancien | Taille | Ce qu'il fait | Dans le nouveau ? |
|---|---|---|---|
| `combat.rs` | 23 Ko | Dégâts, knockback, armor, invuln frames, critical hits, shield blocking | Réduit dans `combat.rs` du server crate |
| `enchanting.rs` | 22 Ko | Table enchantement, options, coût XP, lapis, enchantements vanilla | Partiel dans `enchanting.rs` |
| `recipe.rs` | 31 Ko | `RecipeRegistry` — crafting + smelting recipes complètes | Partiel dans `crafting_recipes.rs` |
| `food.rs` | 3 Ko | Saturation, nutrition, mangeabilité | **Non** |
| `xp.rs` | 9 Ko | XP levels, bottle o' enchanting, furnace XP drops | Partiel dans `xp.rs` |
| `smelting.rs` | 11 Ko | Runtime smelting (furnace, blast furnace, smoker, campfire) | Partiel dans `smoker.rs`/`campfire.rs` |
| `anvil.rs` | 14 Ko | Réparation, renommage, combinaison enchantements, calcul XP | Partiel dans `anvil.rs` (3 Ko) |
| `grindstone.rs` | 5 Ko | Désenchantement, réparation simple | **Non** |
| `loom.rs` | 6 Ko | Patterns bannières | **Non** |
| `inventory.rs` | 39 Ko | `PlayerInventory` complet (hotbar, armor slots, offhand, crafting grid, shift-click, split, merge) | Partiel dans `inventory.rs` |
| `block_entity.rs` | 30 Ko | Chest, Furnace, Sign, Skull, Banner, Beacon, Shulker, Bed, Brewing Stand, Hopper, Dropper, Dispenser | Partiel via modules séparés |
| `projectile.rs` | 6 Ko | Tick + collision des flèches, œufs, boules de neige, tridents | **Non** |
| `breeding.rs` | 3 Ko | Reproduction mobs, items de reproduction | Partiel dans `breeding_items.rs` |
| `components.rs` | 3 Ko | Composants entités (health, velocity, rotation, AABB) | **Non** |
| `mob_registry.rs` | 5 Ko | Registre mobs | **Non** |
| `game_world.rs` | 35 Ko | `GameWorld` central, `GameEvent`, tick global | **Non** (remplacé par logique répartie dans `main.rs`) |
| `ai/` dossier | — | AI goals + states pour mobs | **Non** |

---

## 4. Fichiers `connection/` (ancien) disparus

Dans `old_crates/mc-rs-server/src/connection/` :

| Fichier | Taille | Ce qu'il fait | Dans le nouveau ? |
|---|---|---|---|
| `combat.rs` | 19 Ko | Handler attaque PvP/PvE serveur (InventoryTransaction → UseItemOnEntity → damage calculation → EntityEvent hurt → death handling) | Réduit dans `combat.rs` du server crate |
| `commands.rs` | 113 Ko | Handler de **toutes** les commandes vanilla (/tp, /give, /gamemode, /kill, /summon, /effect, /enchant, /xp, /weather, /time, /difficulty, /gamerule, /kick, /ban, /op, /deop, /pardon, /reload, /save-all, /stop, /say, /tell, /me, /list, /help, /seed, /setworldspawn, /spawnpoint, /setblock, /fill, /clone, /testfor, /execute, /scoreboard, /team, /tag, /clear, /replaceitem, /attribute, etc.) | Consolidé dans `commands.rs` (139 Ko) mais couverture différente |
| `portal.rs` | 32 Ko | Téléportation Nether/End, portails, portail allumé/éteint | **Non** |
| `projectile.rs` | 30 Ko | Spawn + tick + collision flèches, tridents, œufs, boules de neige, perles d'ender, boules de feu | **Non** |
| `survival.rs` | 19 Ko | Faim, régénération, dégâts environnementaux (feu, lave, chute, suffocation, noyade), poison, wither | **Non** |
| `world_tick.rs` | 27 Ko | Tick du monde complet (croissance cultures, météo, spawn mobs, despawn, random ticks) | **Non** (tick léger dans `tick.rs`) |
| `plugins.rs` | 18 Ko | Hook vers `plugin_manager`, `EventBus`, `PendingAction` dispatch | **Non** |

---

## 5. Fichiers `mc-rs-server/src/` (ancien) racine disparus

| Fichier | Taille | Ce qu'il fait | Dans le nouveau ? |
|---|---|---|---|
| `permissions.rs` | 7 Ko | `PermissionManager`, `BanEntry`, whitelist, ops, groupes de permissions | **Non** |
| `persistence.rs` | 19 Ko | `LevelDat` (world metadata format BDS), `PlayerData` (inventaire + position + XP + potions persistés en JSON/NBT) | Partiel : seulement position JSON dans le nouveau |
| `plugin_manager.rs` | 24 Ko | Gestionnaire plugins (chargement dynamique, dispatch events, `ServerSnapshot`, `PendingAction` queue) | **Non** |
| `query.rs` | 8 Ko | Protocole UDP Query (GameSpy) pour trackers externes | **Non** |
| `rcon.rs` | 5 Ko | Remote console TCP (protocole Source RCON) | **Non** (remplacé partiellement par `mc-rs-webui`) |

---

## 6. Packets `mc-rs-proto/src/packets/` (ancien) disparus ou simplifiés

L'ancien a **70 fichiers** séparés pour les packets. Le nouveau regroupe dans 5 fichiers (`login.rs`, `player.rs`, `world.rs`, `chunks.rs`, `forms.rs`). État des packets manquants côté nouveau :

### Totalement absents du nouveau

| Packet ancien | ID | Rôle |
|---|---|---|
| `add_item_entity.rs` | 0x0F | Spawn d'un item drop au sol |
| `take_item_entity.rs` | 0x11 | Ramassage d'un item |
| `mob_effect.rs` | 0x1C | Potions/effets (add/remove/modify) |
| `change_dimension.rs` | 0x3D | Passage Overworld ↔ Nether ↔ End |
| `set_display_objective.rs` | 0x6B | Scoreboard sidebar |
| `set_score.rs` | 0x6C | Scoreboard scores |
| `spawn_particle_effect.rs` | 0x76 | Particules custom |
| `player_enchant_options.rs` | 0x92 | Options table enchantement |
| `player_skin.rs` | 0x5D | Changement skin runtime |
| `respawn.rs` | 0x2D | Séquence mort → respawn |
| `update_adventure_settings.rs` | 0xBC | Flags adventure mode (no-break, no-place…) |
| `container_set_data.rs` | 0x33 | Progression fournaise, barre brewing |
| `resource_pack_data_info.rs` | 0x52 | Métadata pack ressources |
| `resource_pack_chunk_data.rs` | 0x53 | Transfert pack ressources |
| `resource_pack_chunk_request.rs` | 0x54 | Demande chunk pack |
| `serverbound_loading_screen.rs` | 0x138 | État écran de chargement |
| `set_spawn_position.rs` | 0x2B | Position de spawn monde |
| `set_time.rs` | 0x0A | Heure du jour (fonction existe côté nouveau mais standalone packet nope) |
| `set_title.rs` | 0x58 | Titles / subtitles / actionbar |
| `play_sound.rs` | 0x56 | Sons custom avec coords |
| `boss_event.rs` | 0x4A | Boss bar (show/hide/update) |
| `transfer.rs` | 0x55 | Transfert serveur (cross-server) |

### Présents mais simplifiés

| Packet | Différence |
|---|---|
| `inventory_transaction.rs` | Ancien : 23 Ko avec `UseItemAction`, `UseItemData`, `UseItemOnEntityAction`, `UseItemOnEntityData`. Nouveau : handler seulement pour `NormalTransaction` + `UseItem` basique |
| `item_stack_request.rs` / `item_stack_response.rs` | Ancien : support complet des actions (Take, Place, Swap, Drop, Destroy, CraftCreative, CraftRecipe, CraftRecipeAuto, ConsumeStack…). Nouveau : CraftCreative + Take/Place |
| `crafting_data.rs` | Ancien : recettes shaped, shapeless, furnace, brewing, smithing, stonecutter, multi-recipe. Nouveau : shapeless + shaped basique |
| `entity_event.rs` (0x1B) | Nouveau : constante ID seulement, pas d'encoder complet |
| `add_actor.rs` | Ancien : `AddActor` avec `ActorAttribute` complet + metadata. Nouveau : struct allégée |
| `update_attributes.rs` | Ancien : `AttributeEntry` avec min/max/default/modifiers. Nouveau : version courte |

---

## 7. Systèmes gameplay entiers absents

Synthèse de ce qui n'existe plus **fonctionnellement** dans le nouveau (au-delà des fichiers) :

### Monde / dimensions
- **Dimension Nether** (génération + portails)
- **Dimension End** (génération + dragon + portails)
- **Redstone** complet (wire, torch, repeater, comparator, piston activation, lever, button, pressure plate, tripwire, observer)
- **Pistons** (push/pull, sticky, slime/honey blocks)
- **Fluid simulation** (eau qui s'écoule, lave qui brûle, évaporation, source)
- **Gravity blocks** (sand, gravel, anvil, concrete powder)
- **Block ticks aléatoires** (croissance blé/patates/betteraves, propagation feu, fonte neige/glace, cactus/bambou/canne à sucre, leaf decay)
- **Scheduled block ticks** (redstone delays, tree growth triggered)

### Entités / mobs
- **Projectiles** (flèches, tridents, œufs, boules de neige, perles d'ender, boules de feu)
- **AI mobs** (goals, states, pathfinding)
- **Breeding complet** (avec cooldown, bébés)
- **Mob spawning dynamique** (selon biome + lumière + heure)

### Combat / survie
- **PvP complet serveur** (damage calc, armor, enchantements, crits, shield, invuln frames)
- **Faim / saturation / nutrition**
- **Dégâts environnementaux** (feu, lave, chute, suffocation, noyade, wither, poison, starvation)
- **Régénération**
- **Bouclier / bloquer attaque**

### Inventaire / UIs
- **Enchanting table** (3 options + coût XP + lapis)
- **Grindstone** (désenchant + réparation)
- **Loom** (patterns bannières)
- **Brewing stand runtime** (potions avec effets)
- **Anvil** complet (combinaison enchant, renommage, coût XP exponentiel)

### Plugins / extensibilité
- **Plugin API stable** (events, hooks, pending actions)
- **Plugin Lua** (scripts)
- **Plugin WASM** (compiled)
- **Behavior packs** (.mcpack loader)

### Admin / ops
- **RCON** (protocole TCP Source RCON pour console distante)
- **Query** (protocole UDP GameSpy pour trackers MCPE)
- **Permissions** (groupes, héritage, wildcards)
- **Ban list** persistée
- **level.dat** standard BDS-compatible

### Anti-cheat
- **`ViolationTracker`** (compteur violations par type, decay automatique)
- **`PlayerAabb`** (hitbox physique pour collision checks)
- Constantes : `BLOCK_REACH`, `MAX_FALL_PER_TICK`, `MAX_AIRBORNE_TICKS`, `MAX_AIRBORNE_KICK`, `MAX_ACTIONS_PER_SECOND`, `MIN_ATTACK_INTERVAL`, `MIN_BREAK_INTERVAL`, `MIN_COMMAND_INTERVAL`, `MIN_PLACE_INTERVAL`

---

## 8. Priorités suggérées pour portage

Classement par rapport utilité/complexité :

### Rapide et haute valeur
1. **`rcon.rs`** — 5 Ko, isolé, utile pour admin à distance
2. **`gravity.rs`** — 6 Ko, indépendant, important pour gameplay (sable/gravier)
3. **`permissions.rs`** — 7 Ko, ban list persistée (critique pour serveur public)
4. **`survival.rs` (partiel)** — juste faim + régénération + chute

### Moyenne complexité, gros impact
5. **`fluid.rs`** (24 Ko) — eau/lave qui s'écoulent
6. **`block_tick.rs`** (15 Ko) — croissance cultures, feu
7. **`combat.rs` (ancien game)** — PvP serveur complet
8. **`projectile.rs`** — flèches + tridents

### Gros morceaux
9. **`redstone.rs`** (28 Ko) — redstone complet
10. **`piston.rs`** (17 Ko) — pistons
11. **Dimensions Nether + End** (`nether_generator.rs` + `end_generator.rs` + `portal.rs`)
12. **Plugin API + Lua runtime**

### Très spécifique
13. **`query.rs`** — protocole Query
14. **AI mobs** (`ai/` complet)
15. **`grindstone.rs` + `loom.rs`** — UIs secondaires

---

## 9. Ce que le nouveau apporte en plus

Pour contexte, le nouveau introduit :

- **`mc-rs-webui`** — interface web HTTP (remplace en partie rcon + query)
- **`mc-rs-nbt`** avec 3 variantes séparées (`be.rs`, `le.rs`, `network.rs`)
- **`bin/generate_block_registry.rs`** — génération auto du registre depuis `canonical_block_states.nbt`
- **`block_registry_data.rs`** (55 Ko de données canoniques embarquées)
- **Content vanilla encodé en dur** — chaque mob, item, biome, structure a son fichier dans `crates/mc-rs-server/src/*.rs` (~400 fichiers)
- **State machine login explicite** (`ConnectionState`) respectant l'ordre PMMP
- **`event/` interne** (mini-remplacement du plugin system)
- **Connection découpée** : `chat.rs`, `chunks.rs`, `forms.rs`, `spawn.rs`, `movement.rs`, `inventory.rs`, `login.rs` (au lieu d'un mega-`mod.rs`)
