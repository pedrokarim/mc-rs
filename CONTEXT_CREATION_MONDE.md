# Contexte Complet - Bug "Creation du monde" / Crash Connexion

Date de mise a jour: 2026-03-04 (soir)

## 1) Probleme global
- Le client Bedrock (protocol 924 / game version 1.26.2) ne termine pas correctement l'entree en jeu sur ce serveur.
- Deux comportements observes selon les builds:
1. Boucle "Creation du monde / Recherche du serveur".
2. Popup "Une erreur s'est produite", puis deconnexion immediate.

Reference fonctionnelle:
- PocketMine-MP fonctionne correctement dans le meme contexte de test.

## 2) Etat actuel (log le plus recent)
Source:
- `.reference/server.log` (LastWriteTime: 2026-03-04 18:34:23)

Observation cle:
- La session va jusqu'a l'envoi complet du burst pre-spawn.
- Le client se deconnecte juste apres `Sent pre-spawn PlayerList(local)`.
- Aucune phase spawn/chunks n'est atteinte dans ce scenario (pas de `RequestChunkRadius`).

Sequence reseau recente (resume):
1. Login OK -> ResourcePacks OK.
2. `StartGame` envoye (id `0x0B`, `sub_len=263`).
3. Pre-spawn envoye:
   - `SetTime` (`0x0A`)
   - `SetDifficulty` (`0x3C`)
   - `SetSpawnPosition(world)` (`0x2B`)
   - `ItemRegistry` (`0xA2`, ~108k)
   - `AvailableEntityIdentifiers` (`0x77`)
   - `BiomeDefinitionList` (`0x7A`)
   - `AvailableCommands` (`0x4C`)
   - `SetPlayerGameType` (`0x3E`)
   - `UpdateAbilities` (`0xBB`)
   - `UpdateAdventureSettings` (`0xBC`)
   - `SetActorData` (`0x27`)
   - `UpdateAttributes` (`0x1D`)
   - `InventoryContent` (`0x31`, 2 paquets)
   - `MobEquipment` (`0x1F`)
   - `CreativeContent` (`0x91`)
   - `CraftingData` (`0x34`)
   - `PlayerList(local)` (`0x3F`, ~33k)
4. Deconnexion immediate:
   - `Session ... sent disconnect notification`
   - `Session disconnected ...`

Conclusion:
- Le crash client survient dans ou juste apres le bloc pre-spawn (avant monde/chunks).

## 3) Correctifs deja appliques (avec resultats)

## 2026-03-03
1. Rayon de spawn reduit en Spawning (`clamp(1,2)`).
- But: eviter surcharge chunks.
- Resultat: pas suffisant.

2. Alignement pre-spawn PocketMine:
- `SetTime` (VarInt signed), `SetDifficulty`, `SetSpawnPosition(world)`.
- Resultat: pas suffisant.

3. Fallback readiness:
- Si `ServerboundLoadingScreen(type=1)`, finaliser spawn sans `SetLocalPlayerAsInitialized`.
- Resultat: amelioration partielle, mais blocage persistant selon scenario.

4. Post-ready supplementaire:
- `SetSpawnPosition(player)` + `MovePlayer(reset)`.
- Resultat: pas de resolution definitive.

5. Correctif NBT byte-array (wire format) — **ERRATUM 2026-03-04** :
- L’hypothèse « VarUInt(length)+bytes » pour ces champs était fausse. PocketMine écrit le NBT en **octets bruts** (pas de préfixe).
- Correctif appliqué : retour à `put_slice` (NBT brut) pour StartGame (block_properties.nbt, property_data), ItemRegistry (component_nbt), BlockActorData (nbt_data).
- À retester : le client devrait pouvoir parser le pre-spawn et envoyer RequestChunkRadius.

6. Erratum important:
- L'hypothese "prefixer words subchunk/biome dans LevelChunk" etait fausse pour ce format.
- Les words de palette chunk doivent rester en bytes bruts a cet endroit.

## 2026-03-04
7. Correctif `PlayerList(Add)`:
- Champ `color` ARGB (`u32 LE`) manquant dans chaque entree.
- Ajout `color_argb` + encodage `put_u32_le`.
- Valeur par defaut appliquee: `0xFFFF_FFFF`.
- Resultat actuel: le crash persiste (toujours deconnexion juste apres pre-spawn dans le log le plus recent).

## 4) Fichiers modifies importants

Protocole:
- `crates/mc-rs-proto/src/codec.rs`
- `crates/mc-rs-proto/src/packets/start_game.rs`
- `crates/mc-rs-proto/src/packets/item_registry.rs`
- `crates/mc-rs-proto/src/packets/block_actor_data.rs`
- `crates/mc-rs-proto/src/packets/player_list.rs`

Serveur login/spawn:
- `crates/mc-rs-server/src/connection/login.rs`
- `crates/mc-rs-server/src/connection/spawn.rs`
- `crates/mc-rs-server/src/connection/portal.rs`

Memoire des fixes:
- `FIX_LOG.md`

## 5) Verification et build (etat)
- `cargo test -p mc-rs-proto start_game` -> OK
- `cargo test -p mc-rs-proto item_registry` -> OK
- `cargo test -p mc-rs-proto block_actor_data` -> OK
- `cargo test -p mc-rs-proto player_list` -> OK
- `cargo test -p mc-rs-proto` -> OK (suite complete passee)
- `cargo check -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK

Attention runtime:
- Si `target/release/mc-rs-server.exe` est deja lance, le rebuild peut echouer (fichier locke / os error 5).
- Si port 19132 occupe, lancement echoue (`AddrInUse` / os error 10048).

## 6) Ce qu'on sait avec haute confiance
- Le client deconnecte volontairement (disconnect notification), ce n'est pas un panic serveur.
- La rupture intervient avant `RequestChunkRadius` dans le log recent.
- Le probleme est donc tres probablement un des paquets pre-spawn (format ou ordre), pas le streaming de chunks dans ce scenario.

## 7) Suspects prioritaires restants
Ordre de priorite recommande:
1. `PlayerList` skin payload exact (taille/champs) vs PocketMine, malgre ajout du champ color.
2. `UpdateAbilities` (`0xBB`) structure exacte des layers/flags/speeds.
3. `SetActorData` metadata et `PropertySyncData`.
4. `InventoryContent` / `CreativeContent` / `CraftingData` (coherence IDs/format).
5. Ordre strict de la sequence pre-spawn (diff binaire avec PMMP).

## 8) Plan de debug recommande (prochaine etape)
Objectif:
- Isoler le premier paquet qui fait crash client, sans hypothese floue.

Strategie:
1. Ajouter un mode "gating pre-spawn" (env var) pour envoyer les paquets un par un (ou en blocs) et tester.
2. Commencer minimal:
   - StartGame
   - SetTime/SetDifficulty/SetSpawnPosition
   - puis ajouter 1 packet a la fois.
3. Quand le crash reapparait, comparer ce paquet octet-a-octet avec PocketMine.
4. Corriger, rebuild release, retester.
5. Une fois stable, retirer instrumentation.

## 9) Commandes de run fiables
Lancer serveur release depuis:
- `crates/mc-rs-server`

Commande:
```powershell
../../target/release/mc-rs-server.exe 2>&1 | tee ../../.reference/server.log
```

Liberer port 19132 si besoin:
```powershell
Get-NetUDPEndpoint -LocalPort 19132 | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }
```

## 10) Regles anti-boucle
- Toujours verifier d'abord `FIX_LOG.md` et ce fichier avant de retenter un "fix deja fait".
- Ne pas reintroduire des changements deja infirmes (ex: faux prefixage chunk words).
- Chaque hypothese doit etre validee par:
1. preuve log avant/apres,
2. test unitaire si possible,
3. rebuild release confirme.

## 2026-03-05 - Etat actuel `crates_new` (PocketMine-first)

Contexte:
- Le developpement actif est dans `crates_new/*`.
- `old_crates/*` est a eviter sauf blocage majeur.

Changements appliques aujourd'hui:
1. `UpdateAbilities` aligne PMMP:
- 2 layers (BASE + CACHE)
- format layer complet: `layerType + abilitiesSet + abilityValues + flySpeed + verticalFlySpeed + walkSpeed`.
- fichier: `crates_new/mc-rs-proto/src/packets/update_abilities.rs`

2. `PlayerList(Add)` aligne PMMP:
- ajout `color` ARGB (`u32 LE`) par entree.
- ajout des bool `skin_verified` en fin de packet.
- fichier: `crates_new/mc-rs-proto/src/packets/player_list.rs`

3. `UpdateAdventureSettings`:
- `showNameTags=true`
- `autoJump=true`
- fichier: `crates_new/mc-rs-proto/src/packets/update_adventure_settings.rs`

4. Politique spawn stricte PMMP:
- suppression du fallback `ServerboundLoadingScreen => InGame`.
- succes spawn = reception `SetLocalPlayerAsInitialized` uniquement.
- fichier: `crates_new/mc-rs-server/src/connection.rs`

5. Chunk column PMMP (runtime):
- payload reconstruit en v8 (subchunks + biomes + border count 0, sans tiles).
- subchunks single-valued en bpb=0 (pas de words, pas de longueur words).
- `subChunkCount` derive des bornes overworld `[-4..19]` avec top non-empty.
- runtime IDs `air/bedrock` resolves depuis `.reference/BedrockData/canonical_block_states.nbt`.
- fallback logge si parsing NBT echoue.
- fichier: `crates_new/mc-rs-server/src/connection.rs`

6. Instrumentation temporaire:
- `MC_RS_TRACE_SPAWN=1` -> traces RX/TX (StartGame, UpdateAbilities, PlayerList, NCPU, 1er LevelChunk, RequestChunkRadius, LoadingScreen, SetLocalPlayerAsInitialized, ClientCacheStatus).
- fichier: `crates_new/mc-rs-server/src/connection.rs`

7. Signal client cache:
- prise en charge de `ClientCacheStatus` (`0x81`) en pre-game avec log.
- constante ajoutee: `ID_CLIENT_CACHE_STATUS=0x81`.
- fichiers:
  - `crates_new/mc-rs-proto/src/packets/mod.rs`
  - `crates_new/mc-rs-server/src/connection.rs`

Validation:
- `cargo test -p mc-rs-proto` -> OK
- `cargo test -p mc-rs-server` -> OK
- `cargo build --release -p mc-rs-server` -> OK
- binaire: `target/release/mc-rs-server.exe` mis a jour le `2026-03-05 15:16:31`.

## 2026-03-05 - Mise a jour 16:37 (PocketMine-first)

Corrections ajoutees depuis l'etat ci-dessus:
1. `UpdateAbilities` corrige apres diff PMMP:
- PMMP utilise 1 seule layer BASE dans ce cas (pas BASE+CACHE).
- taille packet observee: `len=33`.
- fichiers:
  - `crates_new/mc-rs-proto/src/packets/update_abilities.rs`
  - `crates_new/mc-rs-server/src/connection.rs`

2. `ItemRegistry` non vide (alignement required_item_list):
- abandon de `encode_empty()`.
- chargement de `crates_new/mc-rs-proto/data/item_list.json`.
- envoi de toutes les entrees (`name`, `runtime_id`, `component_based`, `version`, NBT compound vide).
- fichiers:
  - `crates_new/mc-rs-proto/src/packets/item_registry.rs`
  - `crates_new/mc-rs-server/src/connection.rs`

3. Spawn sync renforce:
- envoi `SetSpawnPosition(player)` + `SetSpawnPosition(world)`.
- envoi `MovePlayer(mode=normal)` juste avant `PlayStatus(PlayerSpawn)`.
- fichiers:
  - `crates_new/mc-rs-server/src/connection.rs`
  - `crates_new/mc-rs-proto/src/packets/move_player.rs`

4. Payload chunk rapproche PMMP:
- subchunks "vides" encodes en `version=8, layerCount=0`.
- seul subchunk top non-vide encode avec palette runtime single-valued.
- but: eviter l'ecart "air layer forcee" sur subchunks vides.
- fichier:
  - `crates_new/mc-rs-server/src/connection.rs`

5. Build/Tests valides apres patch:
- `cargo test -p mc-rs-proto` -> OK (3 tests)
- `cargo build --release -p mc-rs-server` -> OK
- binaire actuel:
  - `C:\\Users\\karim\\Desktop\\programming-laboratory\\mc-rs\\target\\release\\mc-rs-server.exe`
  - LastWriteTime: `2026-03-05 16:41:23`

## 2026-03-05 - Instrumentation diff binaire PMMP

Objectif:
- comparer PMMP vs mc-rs en binaire complet (payload packet, non tronque) pour isoler le premier octet divergent.

Changements:
1. `mc-rs` dump binaire TX sous `MC_RS_TRACE_SPAWN=1`:
- paquets: `StartGame`, `UpdateAbilities`, `PlayerList`, `NetworkChunkPublisherUpdate`, `1er LevelChunk`.
- sortie: `.reference/dumps/mc_rs/*.bin`
- fichier: `crates_new/mc-rs-server/src/connection.rs`

2. PMMP dump binaire TX sous `MC_RS_TRACE_SPAWN=1`:
- hook temporaire dans `NetworkSession::sendDataPacketInternal()`.
- extraction `packet_id` + payload binaire (sans header varint packet).
- paquets: meme liste que ci-dessus (avec `1er LevelChunk` seulement).
- sortie: `.reference/dumps/pmmp/*.bin`
- fichier: `.reference/PocketMine-MP/src/network/mcpe/NetworkSession.php`

3. Outil de comparaison:
- script: `.reference/tools/compare_spawn_dumps.ps1`
- compare `pmmp` vs `mc_rs` par type de paquet.
- affiche: tailles + premier offset divergent + contexte hex.

Validation technique:
- `cargo test -p mc-rs-proto` -> OK
- `cargo build --release -p mc-rs-server` -> OK
- `php -l .reference/PocketMine-MP/src/network/mcpe/NetworkSession.php` -> OK

Prerequis PMMP pour dumps:
- installation d'un runtime `pmmp/PHP-Binaries` compatible dans:
  - `.reference/PocketMine-MP/bin/php/php.exe`
- extensions critiques verifiees (`chunkutils2`, `crypto`, `encoding`, `pmmpthread`, etc.) via `php -m`.
- pour eviter le setup wizard interactif, lancer PMMP avec:
  - `./start.ps1 --no-wizard`
