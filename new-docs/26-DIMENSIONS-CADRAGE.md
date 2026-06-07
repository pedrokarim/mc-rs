# 26 — Cadrage : Dimensions (Nether / End)

> Document canonique du chantier **Phase DIM**. Consolidé à partir d'une reconnaissance
> exhaustive de l'existant mc-rs + des références. **À tenir à jour au fil du chantier.**
> Roadmap : voir [`99-ROADMAP.md`](99-ROADMAP.md) §Phase DIM.

## 0. Pourquoi ce document (et pourquoi PMMP ne suffit pas)

PocketMine-MP est notre référence canonique pour 95 % du serveur, **mais pas pour les
dimensions** :

- `ChangeDimensionPacket` existe dans le protocole PMMP mais **n'est JAMAIS envoyé** par le code.
- **Aucun générateur End.** Générateur Nether basique (`hell/Nether.php`).
- Portails = coquilles vides : `NetherPortal.php` et `EndPortalFrame.php` ont l'allumage et le
  linking en `//TODO`.
- La classe `World` PMMP **n'a aucune notion de dimension** — `SpawnSettings` est codé en dur à
  `DimensionIds::OVERWORLD` (commentaire `//TODO: implement this properly`).

On change donc de référence principale pour ce chantier :

| Sujet | Référence canonique retenue | Emplacement |
|---|---|---|
| Modèle de dimension, transfert, portails (Bedrock) | **Allay** (serveur Java Bedrock moderne) | `.reference/Allay/` |
| Séquence wire transfert + clés LevelDB par dim | **dragonfly** (serveur Go Bedrock) | `.reference/dragonfly/` |
| Wire format des paquets (cross-check) | bedrock-protocol (PMMP) + **gophertunnel** | `.reference/BedrockProtocol/`, `.reference/gophertunnel/` |
| Recette de génération Nether/End (density functions) | **mcmeta** (data vanilla Java) | `.reference/mcmeta/data/minecraft/worldgen/` |
| Données biomes/surface Bedrock | **bedrock-samples** | `.reference/bedrock-samples/behavior_pack/biomes/` |

PMMP reste la référence pour ce qui est partagé (wire des paquets communs, ChunkSerializer,
bornes de sub-chunks par dimension).

---

## 1. Constantes & wire protocol (confirmés)

### 1.1 IDs de dimension (canoniques, tous serveurs)
`OVERWORLD = 0`, `NETHER = 1`, `THE_END = 2`.
Déjà présents dans mc-rs : `dimensions::DimensionId` (`crates/mc-rs-server/src/dimensions.rs`) **et**
`visuals.rs` (`DIMENSION_OVERWORLD/NETHER/END`). ⚠️ Doublon à dédupliquer : faire de
`DimensionId` la source unique.

### 1.2 Bornes Y par dimension (sub-chunk bounds)
Réf PMMP `ChunkSerializer::getDimensionChunkBounds` + Allay `DimensionId` + dragonfly `Range()` :

| Dimension | Y min | Y max | Sub-chunks | Note |
|---|---|---|---|---|
| Overworld | -64 | 320 | 24 (index -4..19) | actuel |
| Nether | 0 | 128 | 8 (index 0..7) | toit bedrock à 127 |
| End | 0 | 256 | 16 (index 0..15) | — |

> ⚠️ `getDimensionChunkBounds` (PMMP) renvoie `[-4,19] / [0,7] / [0,15]`. Le nombre de
> sub-chunks envoyés dans `LevelChunk` **dépend de la dimension** — le `chunk_serializer` devra en
> tenir compte (aujourd'hui 24 hardcodé pour l'overworld).

### 1.3 Paquets — état mc-rs et format à reproduire

| Paquet | ID (975) | État mc-rs | Action |
|---|---|---|---|
| `ChangeDimensionPacket` | **0x3D** | ❌ absent | **créer** |
| `PlayerActionPacket` | 0x24 | struct OK, pas d'action 14 | gérer `DIMENSION_CHANGE_ACK=14` |
| `RespawnPacket` | 0x2D | ID seul | **créer struct** |
| `PlayStatusPacket` | (existe) | OK | réutiliser (`PlayerSpawn`) |
| `StopSoundPacket` | 0x57 | OK (`/stopsound`) | réutiliser (silent) |
| `StartGamePacket` | 0x0B | OK, champ `dimension` présent | passer la vraie dim |
| `LevelChunkPacket` | 0x3A | OK, champ `dimension_id` présent mais **hardcodé 0** | passer la vraie dim |
| `SetSpawnPositionPacket` | 0x2B | OK, champ `dimension` présent | utiliser pour respawn anchor / world spawn |
| `MovePlayerPacket` | 0x13 | OK | **pas** de champ dimension (normal) |

**`ChangeDimensionPacket` (0x3D) — wire (réf `BedrockProtocol/.../ChangeDimensionPacket.php`) :**
```
dimension        VarInt32 (signed)
position         Vector3  (3× f32 LE)
respawn          bool     (1 octet)
loadingScreenId  Optional<UInt32 LE>   // flag 1 octet + valeur conditionnelle 4 octets
```

**`PlayerAction` constantes pertinentes (réf `types/PlayerAction.php`) :**
`ACTION_DIMENSION_CHANGE_ACK = 14` ("sent when spawning in a different dimension").
`ACTION_RESPAWN = 7`. (Format paquet : actorRuntimeId VarU64, action VarInt32, blockPos 3×VarInt32,
resultPos 3×VarInt32, face VarInt32.)

**`RespawnPacket` (0x2D) — wire (réf `RespawnPacket.php`) :**
```
position        Vector3 (3× f32 LE)
respawnState    u8        // 0 SEARCHING_FOR_SPAWN, 1 READY_TO_SPAWN, 2 CLIENT_READY_TO_SPAWN
actorRuntimeId  VarU64
```

### 1.4 Séquence de changement de dimension (CRITIQUE — réf dragonfly `session.go:481-501`)

**Depuis v1.19.50, c'est le SERVEUR qui envoie l'ack de fin**, pas le client. Séquence exacte :

```
SERVEUR → CLIENT (début, montre le loading screen tôt) :
  1. ChangeDimensionPacket { dimension, position, respawn=false, loadingScreenId=Some(id++) }

  (charger les chunks de la dimension cible de façon asynchrone ; vider le cache de chunks
   côté connexion ; mettre à jour time/weather/gamerules si le "monde" diffère)

SERVEUR → CLIENT (chunks de la nouvelle dimension) :
  2. N × LevelChunkPacket { dimension_id = cible }

SERVEUR → CLIENT (fin) :
  3. StopSoundPacket   { silent = true }            // coupe l'audio résiduel
  4. PlayStatusPacket  { status = PlayerSpawn }
  5. PlayerActionPacket{ action = DIMENSION_CHANGE_ACK(14) }   // le serveur s'auto-ack
```

Allay confirme le même squelette : `beginDimensionChange()` (envoie ChangeDimension tôt) → load
async → `removePlayer(source)` → `addPlayer(target)` → `completeDimensionChange()`.

> Le client envoie **aussi** `PlayerAction DIMENSION_CHANGE_ACK` quand il a fini de spawn. On
> l'accepte (no-op de tracking) mais on ne **dépend pas** de lui : le serveur pilote la séquence.

---

## 2. Architecture retenue : un monde par dimension

**Décision : 3 conteneurs de monde séparés** (overworld / nether / end), pas un seul cache
"dimension-aware". C'est le modèle Allay (`World` → `Map<DimensionType, Dimension>`, chaque
`Dimension` a son `ChunkManager` + générateur) et dragonfly (un `*World` par dimension). Plus
simple à raisonner, isolation naturelle des entités/mobs, générateur distinct par dim.

### 2.1 Forme cible

```
ServerState
└── worlds: [Dimension; 3]   (indexé par DimensionId)
      Dimension {
        dim:        DimensionId,
        chunk_cache: ChunkCache,   // déjà existant, réutilisé tel quel
        generator:  Generator,     // overworld(normal/noise/flat) | nether | end
        y_bounds:   (i32, i32),    // depuis DimensionId::min_y/max_y
      }
```

Chaque `ChunkCache` garde sa structure actuelle (`HashMap<(cx,cz), ChunkColumn>`). On NE met PAS
la dimension dans la clé mémoire — c'est le conteneur qui porte la dimension. La dimension entre en
jeu uniquement (a) dans la **clé LevelDB** et (b) dans le **wire** (`LevelChunk.dimension_id`).

### 2.2 Côté Connection (joueur)
- Nouveau champ `current_dimension: DimensionId` (défaut Overworld).
- `connection/chunks.rs:134` et `:168` : remplacer `dimension_id: 0` par `self.current_dimension as i32`.
- Toutes les lectures/écritures de chunk d'un joueur passent par `worlds[current_dimension]`.
- Le déplacement entre dimensions = state machine §1.4 + bascule du `current_dimension`.

> **Décision actée (2026-06-07)** : option **« 3 ChunkCache + 1 seule LevelDB »**. Les 3 caches
> mémoire partagent un unique `WorldStorage` (`Arc<Mutex<WorldStorage>>`), la dimension étant dans
> la clé (DIM-B). Format on-disk = vanilla Bedrock (monde importable/exportable). Un seul lock file
> LevelDB possible → le partage par `Arc<Mutex>` est obligatoire (pas 3 DB séparées).

### 2.3 Storage LevelDB (réf dragonfly `mcdb/db.go:523-535`)
Format canonique Bedrock — le **dimension-id n'est ajouté que pour nether/end** (rétrocompat
overworld) :
```
clé index :
  Overworld : [cx i32 LE][cz i32 LE]              (8 octets)   ← inchangé, pas de migration
  Nether/End: [cx i32 LE][cz i32 LE][dim u32 LE]  (12 octets)
puis : [tag u8]  (+ [y u8] pour les sub-chunks)
```
`world/storage.rs::chunk_index/chunk_key/sub_chunk_key` prennent un paramètre `dim`. Les mondes
overworld existants restent lisibles **sans migration** (clés à 8 octets préservées). Brancher au
passage le module dormant `level_db_keys.rs` (constantes de tags) comme source unique.

---

## 3. Génération Nether (réf mcmeta `noise_settings/nether.json` + PMMP `hell/Nether.php`)

Paramètres vanilla exacts :
- Hauteur 128 (Y 0..127). `sea_level` (lave) = **32**. `default_block` = netherrack,
  `default_fluid` = lava. `aquifers_enabled=false`, `ore_veins_enabled=false`.
- Bruit : `old_blended_noise { xz_factor 80, xz_scale 0.25, y_factor 60, y_scale 0.375,
  smear_scale_multiplier 8 }` (`density_function/nether/base_3d_noise.json`).
- Density finale : base 3D noise + gradient Y bas (y=-8→0.0, y=24→1.0) + gradient Y haut
  (y=104→1.0, y=128→0.0), offset +2.5, ×0.64.
- **Bedrock** : sol y0-4 (gradient vertical), toit y123-127 (gradient inversé).
- **Lave** : remplir les "trous" sous y≤32.
- Biomes nether (brancher `nether_biomes.rs` dormant) : nether_wastes, soul_sand_valley,
  crimson_forest, warped_forest, basalt_deltas. Surface builders depuis
  `bedrock-samples/.../biomes/*.biome.json` (floor/ceiling/sea/foundation materials).
- Ores : quartz (veines 10/chunk, Y 10-117), ancient_debris.

**Approche pragmatique v1** : reproduire l'algorithme `Nether.php` (Simplex 4 octaves
`emptyHeight=64`, `emptyAmplitude=1`, `density=0.5`, lave ≤32, bedrock cap) qui est simple et
fonctionnel, puis raffiner vers les density functions vanilla si besoin (le générateur "noise"
moderne de mc-rs partage déjà cette philosophie).

## 4. Génération End (réf mcmeta `noise_settings/end.json` + deepslate `TheEndBiomeSource`)

- Hauteur Bedrock 0..255. `default_block` = end_stone, pas de fluide. `sea_level=0`.
- Île centrale : `sectionX² + sectionZ² ≤ 4096` → biome `the_end` (deepslate). Forme via
  `end_islands` erosion (density), gradient Y (y=4→0, y=32→1 ; y=56→1, y haut→0), offset
  -0.234375, ×23.4375 → îles distinctes.
- **Île centrale (spawn)** : disque d'end_stone centré sur (0,0), + **10 obsidian pillars**
  (module dormant `obsidian_pillar.rs` : angles TAU/10, hauteurs [43,50,41,47,55,45,57,51,49,53],
  cages de bedrock/iron bars en sommet) + **end crystals** au sommet + **exit portal/fountain**
  (bedrock + portal end + dragon egg au kill).
- **Plateforme d'arrivée** des joueurs : obsidienne 5×5 à `[100, 49, 0]`, joueur tp à
  `[100.5, 50, 0.5]` (Allay `createEndPlatform`).
- **Void** entre île centrale (rayon ~128) et îles externes (>1024 blocs).
- **Îles externes** (`end_islands.rs` dormant) + chorus + **end_city** (`end_city.rs`) +
  **end_gateway** (`end_gateway.rs` : 20 gateways en spirale, exit positions).

## 5. Portail Nether (réf Allay `NetherPortalHelper.java`)

1. **Détection frame** : tester axes X et Z ; scanner vers le bas jusqu'au sol obsidienne ;
   largeur intérieure 2-21, hauteur intérieure 3-21 ; valider sol/plafond/côtés obsidienne,
   intérieur = air/feu.
2. **Allumage** : flint&steel / fire_charge sur une face intérieure → `fill()` place les blocs
   `portal` avec propriété `portal_axis` (X/Z), `update=false`. (Bedrock : pas de bloc de feu
   requis.) Brancher `nether_portal_spawn.rs` (validation taille) + émettre l'équivalent
   `PortalCreateEvent`.
3. **Linking 1:8** : `scaleCoordinate` — OW→Nether ÷8, Nether→OW ×8 (déjà dans
   `dimensions::nether_portal_coordinate`). Clamp aux bornes.
4. **Recherche/création destination** : charger les chunks dans un rayon (1 chunk côté Nether,
   8 chunks côté OW) ; chercher un portail existant (POI) ; sinon `findSafeY` + `createPortal`
   (frame obsidienne 4×5, intérieur 2×3, plateformes netherrack perpendiculaires).
5. **Téléportation** : `nether_portal.rs` (dormant, 85 %) — `PortalState` : 80 ticks dans le
   portail (survie) / 20 (créatif) avant tp ; cooldown 80 ticks après. Puis state machine §1.4.

## 6. Portail End + Gateway (réf Allay `BlockEndPortal*` / `BlockEndGateway*`)

- **End portal frame** : 12 blocs en 3×4 autour d'un centre, insertion eye of ender
  (`END_PORTAL_EYE_BIT`), comparator=15 si eye ; quand les 12 ont un eye → remplir 3×3 de blocs
  `end_portal` + son `END_PORTAL_SPAWN`. Brancher `end_portal.rs` (frames) +
  `end_features.rs`.
- **Eye of ender signal** : projectile vers le stronghold le plus proche (modules `stronghold.rs`
  rings + `EntityEyeOfEnderSignal` côté Allay). v1 : approximation grid (comme `/locate`).
- **Voyage** : collision avec `end_portal` → §1.4 vers End, plateforme `[100,49,0]`. Retour
  (depuis l'End, au portail de sortie central) → world spawn overworld.
- **End gateway** (`end_gateway.rs`, 75 %) : généré à la mort du dragon ; tp vers îles externes
  (>1024 blocs), 20 positions en spirale.

## 7. Spawn des mobs gaté par dimension (réf `mob_spawner.rs` + `spawn_rules_vanilla`)

- Mobs nether (blaze, ghast, piglin, piglin_brute, zombified_piglin, hoglin, magma_cube,
  wither_skeleton via skeleton, strider) → **uniquement dimension Nether**, par biome nether.
- Mobs end (enderman en masse, shulker via end_city, endermite) → **dimension End**.
- Guardian/elder_guardian → overworld dans `ocean_monument` (structure, hors scope DIM strict).
- **Empêcher** les mobs nether/end de spawn en overworld (filtrage par `current_dimension` du
  spawner + tags de dimension dans `spawn_rules_vanilla`).
- Tous les structs mobs nether/end existent (dormants, 60-85 %) — voir §9.

## 8. Combat ender dragon (réf modules dormants + Allay entités)

Allay a `EntityEnderDragon`/`EntityEnderCrystal` mais **pas de fight controller**. On a déjà
côté mc-rs (dormants, quasi complets) :
- `dragon_fight.rs` (80 %) : state machine PreDragon→Spawning→Alive→Dying→Dead→Respawn, drop egg
  au 1er kill seulement.
- `ender_dragon.rs` (85 %) : phases (Holding/Strafing/Landing/Sitting/Roaring/Breath/TakingOff/
  Charge/Dying), HP 200, dégâts tête ×1 / corps ×0.25, soin par crystal.
- `end_crystal.rs` (80 %) : explosion power 6, détruit en 1 coup, `heals_dragon()`.

À faire : spawn du dragon à l'entrée de l'End, **invulnérabilité tant que des crystals existent**,
soin +1/20 ticks par crystal connecté, phases pilotées par l'IA existante, à la mort →
exit portal + dragon egg + gateway. L'IA générique (`crate::ai::`) accueille déjà des phases.

## 9. Modules dormants à brancher (inventaire)

Tous compilent, tests unitaires OK, **non branchés**. Complétude entre parenthèses.

- **Core** : `dimensions` (80, seul `nether_portal` l'utilise), `level_db_keys` (40).
- **Nether struct/gen** : `nether_biomes` (60), `nether_fortress` (50), `bastion_remnant` (40),
  `nether_portal` (85), `nether_portal_spawn` (70).
- **End struct/gen** : `end_islands` (50), `end_city` (30), `end_portal` (65), `end_gateway` (75),
  `end_crystal` (80), `end_features` (60, ⚠️ **doublonne** `end_portal`/`end_gateway` → consolider),
  `obsidian_pillar` (70), `dragon_egg` (20).
- **Boss** : `ender_dragon` (85), `dragon_fight` (80), `wither` (85).
- **Structures** : `stronghold` (60), `ocean_monument` (40).
- **Mobs nether** : `blaze` (75), `ghast` (80), `piglin` (75), `piglin_brute` (60),
  `zombified_piglin` (80), `hoglin` (85), `zoglin` (65), `strider` (85). Loot :
  `blaze_loot` (70), `wither_skeleton_loot` (80), `piglin_bartering` (90).
- **Mobs end** : `enderman` (85), `endermite` (70), `shulker` (80), `shulker_box` (75).
- **Autre** : `guardian` (80, overworld), `happy_ghast` (80).

> ⚠️ Ces structs sont des **modèles de données + state machines**, pas des entités IA branchées.
> Le framework `crate::ai::` (déjà actif pour zombie/skeleton/creeper) est le point d'ancrage :
> chaque mob = `MobEntity` + behaviors. Le travail = brancher comportements + projectiles
> (fireball ghast/blaze, bullet shulker, etc.), pas réécrire les structs.

---

## 10. Plan par phases (ordre d'exécution)

Chaque phase est **buildable + testable** avant la suivante (philosophie roadmap).

- **DIM-A — Protocole** : `ChangeDimensionPacket`, `RespawnPacket`, `PlayerAction` action 14.
  Tests d'encodage round-trip. *(aucune dépendance)*
- **DIM-B — Storage** : dimension dans les clés LevelDB (nether/end = +4 octets), overworld
  intact. Tests de clés.
- **DIM-C — Multi-monde** : `ServerState.worlds[3]`, générateur + bornes Y par dim, accès par
  dimension dans Connection. Encore mono-dim côté gameplay (overworld), mais l'infra est là.
- **DIM-D — State machine transfert** : `current_dimension` sur Connection, séquence §1.4,
  commande de debug `/dimension <ow|nether|end>` pour tester le voyage **avant** les portails.
- **DIM-E — Générateur Nether** : terrain + bedrock + lave + biomes + ores. Tester via `/dimension nether`.
- **DIM-F — Portail Nether** : frame/allumage/link 1:8/tp. Tester gameplay réel.
- **DIM-G — Générateur End** : île centrale + pillars + crystals + plateforme + void.
- **DIM-H — Portail End + gateway** : frame/eyes/voyage/retour.
- **DIM-I — Spawn mobs par dimension** : gating nether/end, anti-spawn overworld.
- **DIM-J — Ender dragon fight** : spawn + crystals + invuln + phases + mort/egg/gateway.
- **DIM-K — Structures (optionnel)** : nether_fortress, bastion, end_city (loot data déjà prête).

`/dimension <id>` (DIM-D) est la clé : il permet de valider transfert + générateurs (E,G) **sans**
attendre les portails (F,H), donc on dérisque tôt.

---

## 11. Pièges connus / décisions actées

- **Doublon `DimensionId` vs `visuals::DIMENSION_*`** → `DimensionId` source unique, virer les
  consts de `visuals`.
- **Doublon `end_features` vs `end_portal`/`end_gateway`** → consolider sur ces derniers, supprimer
  `end_features`.
- **Nombre de sub-chunks par dimension** → `chunk_serializer` doit produire 8 (nether) / 16 (end)
  et pas 24. Vérifier `LevelChunk.sub_chunk_count` cohérent avec les bornes.
- **Pas de migration overworld** : la clé reste 8 octets pour dim 0 (compat dragonfly/vanilla).
- **Server-sent dimension ack** (1.19.50+) : ne PAS attendre le client ; le serveur envoie
  `PlayStatus(PlayerSpawn)` + `PlayerAction(DimensionChangeDone)`.
- **Allay/dragonfly ne génèrent PAS le terrain nether/end** → la génération est 100 % à notre
  charge, source = mcmeta density functions + PMMP `Nether.php`. Pas de copie possible, on
  implémente.
