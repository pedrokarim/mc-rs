# 34 – Observer BDS : captures, décompilation, comparaison avec mc-rs

Journal de travail ouvert le 2026-10-25. Depuis la fin de support de PocketMine-MP, mc-rs avançait par
déduction (schémas Mojang, gophertunnel, Cloudburst, dragonfly). Ce chantier prend le serveur officiel de
Mojang, **Bedrock Dedicated Server (BDS)**, comme référence observable : on capture ce qu'il envoie, on lit son
binaire, et on compare avec mc-rs. Chaque étape est consignée ici ; les fichiers bruts ne sont pas supprimés.

## 1. Où sont les éléments

| Élément | Emplacement | Suivi par git |
|---|---|---|
| BDS 1.26.52.3 (Windows et Linux) | `.reference/bds/bedrock-server-1.26.52.3-{win,linux}/` | non (`.reference/`) |
| Captures de paquets (binaire, JSON, journaux) | `.reference/bds/captures/<serveur>/<date>_<scénario>/` | non |
| Rapports de comparaison | `.reference/bds/comparisons/` | non |
| Projet Ghidra de BDS | `C:\tools\ghidra-projects\bds-1.26.52.3` (plusieurs Go) | non |
| Outil de capture et de décodage | `scripts/dev/bds_capture/` (Go, gophertunnel v1.62.0) | oui |
| Lecture et comparaison de captures | `scripts/dev/packet_dump.py`, `scripts/dev/compare_captures.py` | oui |
| Scripts Ghidra | `scripts/dev/ghidra/` | oui |
| REA (rétro-ingénierie par agent) | `.reference/rea`, serveur MCP `rea` de Claude Code | non |

Le jeton Microsoft de l'outil de capture est rangé dans le dossier de configuration de l'utilisateur
(`%APPDATA%\mc-rs\bds-capture-token.json`), jamais dans le dépôt.

## 2. BDS 1.26.52.3

- Téléchargé le 2026-10-25 depuis l'API officielle
  (`https://net-secondary.web.minecraft-services.net/api/v1.0/download/links`), qui annonce 1.26.52.3 en
  release et 1.26.70.21 en preview. Le BDS local précédent datait de 1.26.10.4.
- Branche `r/26_u5` (comme 1.26.50), build 51798905. Accepte un client en protocole **2193**.
- **NetherNet obligatoire** : avec `transport=raknet`, BDS refuse de démarrer (« In this release, NetherNet is
  the only supported transport type »). Il écoute en TCP (signalisation HTTP, `server-port`) et en UDP
  (WebRTC). Les clients essaient NetherNet puis retombent sur RakNet (dragonfly, `config.toml`) : mc-rs reste
  joignable en RakNet.
- Même avec `online-mode=false`, la connexion NetherNet exige une identité Xbox Live (code d'erreur 37,
  `ErrorCodeIdentityNotAllowed` de go-nethernet).
- Le binaire n'a pas de symboles de fonctions (Linux : pas de `.symtab`, 392 symboles dynamiques ; Windows :
  pas de PDB), mais garde la RTTI : `FenceBlock`, `StairBlock`, `PlayerAuthInputPacket`… Compilé avec clang
  (Ghidra : `x86:LE:64:default:clangwindows`).

Configuration de test (`server.properties` de la copie Windows) : `server-port=19150`,
`server-portv6=19151`, `online-mode=false`, `allow-list=false`, `transport=nethernet`.

## 3. Captures

`scripts/dev/bds_capture` se connecte comme un client gophertunnel et enregistre chaque paquet au format de
`MCRS_DUMP_PACKETS=full` (`[u64 ms][u32 id][u32 longueur][charge utile]`), puis `decode` le transforme en
JSON Lines avec le décodeur de gophertunnel. Le même bot sert pour BDS (NetherNet) et pour mc-rs (RakNet) :
les deux captures se comparent avec les mêmes outils.

```text
bds-capture capture -network nethernet -addr http://127.0.0.1:19150 -out <dossier>/cap -stay 20s
bds-capture capture -network raknet -addr 127.0.0.1:19132 -out <dossier>/cap -stay 20s
bds-capture decode -in <dossier>/s2c.bin -direction s2c -out <dossier>/s2c.jsonl
python scripts/dev/compare_captures.py <bds>/s2c.jsonl <mc-rs>/s2c.jsonl --out <rapport>.txt
```

| Capture | Paquets serveur → client | Décodage |
|---|---|---|
| `1.26.52.3/2026-10-25_bot-nethernet-20s` | 1 946 (3,8 Mo) | 1 946 sans erreur ni octet restant |
| `mc-rs-749087b/2026-10-25_bot-raknet-20s` | 788 (11,2 Mo) | 788 sans erreur ni octet restant |

Rapports : `.reference/bds/comparisons/2026-10-25_bds-1.26.52.3_vs_mc-rs-749087b.txt` (tous les paquets
communs) et `2026-10-25_detail_startgame-attributes-playerlist.txt`.

## 4. Constats du 2026-10-25 (bot, 20 s, spawn sans action)

### 4.1 Validé par BDS

- **Format 2193** : BDS et mc-rs se décodent intégralement avec gophertunnel v1.62.0. Les paquets
  convertis en 2193 (optionnels simples, BossEvent, PlaySound…) sont donc cohérents avec le vrai serveur ;
  ceux que le bot ne voit pas (InventoryTransaction, ItemStackRequest/Response) restent à capturer avec un
  vrai client.
- **Blocs data-driven** : BDS envoie exactement 98 entrées dans `Block Properties` de StartGame, et les
  définitions de mc-rs (dragonfly `data_driven_blocks.nbt`) sont **identiques** champ par champ.
- **ItemRegistry** : identique à celui de BDS (aucun écart).
- **Ingrédients de CraftingData** (point douteux n° 1 de 2169) : les octets de BDS se lisent avec la forme
  gophertunnel / Cloudburst (variante, nom ou tag, quantité), celle que mc-rs a retenue.
- NetworkSettings, PlayStatus, ChunkRadiusUpdated : identiques.

### 4.2 Écarts à corriger dans mc-rs

| Paquet | BDS | mc-rs |
|---|---|---|
| StartGame | 39 règles de jeu | aucune (**corrigé**, § 5) |
| StartGame | `ServerAuthoritativeInventory = true` | `false` |
| StartGame | `PlayerMovementSettings.RewindHistorySize = 40` | 0 (**corrigé**) |
| StartGame | `PlayerGameMode = 5` (mode du monde), `BaseGameVersion = "*"` | 0, `1.26.51` (`*` **corrigé**) |
| PlayerList | UUID du joueur | UUID nul (**corrigé**, § 5) |
| PlayerList | `BuildPlatform`, version moteur de la géométrie : valeurs du client relayées | `-1`, `MINECRAFT_VERSION` forcés (géométrie **corrigée**) |
| PlayerList | `Trusted = false`, `PrimaryUser = false`, couleur (237, 237, 237) | `true`, `true`, blanc |
| UpdateAttributes | saturation par défaut 5, épuisement max 20, dégâts d'attaque 1–1, absorption max 16, résistance au recul min −2 ; `air_drag_modifier`, `bounciness`, `friction_modifier` | saturation 20, épuisement max 5, bornes 0–3,4e38 ; `fall_damage` en plus (**corrigé**) |
| SetActorData (joueur) | environ 80 clés de métadonnées | quelques-unes ; drapeaux différents |
| CraftingData | 1 217 façonnées, 2 068 sans forme, 1 456 à données, 210 potions, 2 changements de contenant, 14 multi, 13 forge ; identifiant texte (`minecraft:barrel`), UUID, priorités 0 à 50, bloc de sortie renseigné | 1 064 façonnées, 778 sans forme, rien d'autre ; identifiant numérique, UUID nul, priorité 50 partout, bloc de sortie 0 |
| ResourcePacksInfo | `WorldTemplateVersion = "0.0.0"` | chaîne vide |

Paquets envoyés par BDS et absents de mc-rs : BiomeDefinitionList (définitions complètes), CameraPresets,
CameraAimAssistPresets, GameRulesChanged, TrimData, UnlockedRecipes, VoxelShapes, JigsawStructureData,
SyncWorldClocks, PlayerFog, entre autres.

### 4.3 Écarts attendus

Position, graine, identifiants d'entité, nom du monde, heure, chunks (BDS en mode de requête de
sous-chunks, `SubChunkCount = 0`), IDs de blocs (BDS en hash, mc-rs en séquentiel, les deux valides), skin
(joueurs différents), JWT de la poignée de main.

### 4.4 Questions ouvertes

- ~~Recette `minecraft:barrel` de BDS avec des bâtons~~ : **résolu**. Cette recette est rattachée au bloc
  `deprecated` (priorité −1) : une ancienne recette que BDS envoie encore, pas une erreur de lecture.
- `UpdateAdventureSettings.ShowNameTags` à `false` et `InventoryContent.Container.ContainerID` à 0 chez BDS :
  à confirmer avec un vrai client.

## 5. Corrections du 2026-10-25

Vérifiées par une nouvelle capture (`captures/mc-rs-wip-bds-fixes-2/`, rapport
`comparisons/2026-10-25_bds-1.26.52.3_vs_mc-rs-correctifs-2.txt`) : UpdateAttributes sans écart, UUID et
règles de jeu identiques à BDS ; simulateur 3/3 en jeu, mode probe sans incident.

| Correction | Constat | Source |
|---|---|---|
| UUID des joueurs Xbox | `uuid_str` restait vide en authentification Xbox : **tous les vrais joueurs avaient l'UUID nul** dans PlayerList (collisions en multijoueur). Calculé désormais depuis le XUID : MD5 de `pocket-auth-1-xuid:<xuid>`, version 3, variante RFC 4122 | PMMP `LoginPacketHandler::calculateUuidFromXuid`, valeur identique à BDS |
| Ordre des octets d'UUID | PlayerList et AddPlayer écrivaient les 16 octets bruts ; le protocole les écrit en deux `u64` little-endian (chaque moitié inversée). Masqué jusqu'ici par l'UUID nul. `ProtoWriter::write_uuid` | PMMP `CommonTypes::putUUID`, gophertunnel, BDS |
| Règles de jeu | Aucune dans StartGame ; noms en minuscules ; défauts Java (`randomTickSpeed` 3, `spawnRadius` 5) ; règles récentes absentes. Désormais les 39 règles de BDS, dans son ordre, en camelCase, insensibles à la casse pour `/gamerule` | BDS 1.26.52.3 |
| Attributs du joueur | Saturation initiale 5, épuisement borné à 20, absorption à 16, dégâts d'attaque 1–1, recul dès −2, `friction_modifier`, `bounciness`, `air_drag_modifier` ; `fall_damage` retiré (lu nulle part) ; ordre de BDS | BDS 1.26.52.3 |
| Seuil d'épuisement | Saturation et faim baissaient tous les **5,0** points d'épuisement, le commentaire disait « port PMMP » : PMMP (et le jeu) utilisent **4,0** | PMMP `HungerManager::exhaust` |
| Version de géométrie du skin | Jetée au login, remplacée par `MINECRAFT_VERSION` ; désormais relayée (repli inchangé si absente) | PMMP `ClientDataToSkinDataHelper`, BDS |
| StartGame, ResourcePackStack | `BaseGameVersion = "*"`, historique de rembobinage 40 | BDS |
| Métadonnées du joueur | Drapeau `CAN_CLIMB` absent (le client ne grimpe pas sans lui) ; air et air max, propriétaire, cible, couleur, `ALWAYS_SHOW_NAMETAG`, `SCORE_TAG`, effets visibles absents ; **les autres joueurs (AddPlayer) partaient sans aucun drapeau**. Un seul constructeur, `player_metadata`, pour SetActorData et AddPlayer | Clés communes à PMMP (`Entity`, `Living`, `Player::syncNetworkData`) et BDS ; capture `mc-rs-wip-bds-fixes-3` |

Volontairement non repris :

- **Plateforme dans PlayerList** : BDS relaie celle du client, mais le schéma Mojang n'admet que 11 valeurs
  sans donner leurs nombres, et une valeur hors énumération provoque l'écran « Block » du vrai client
  (montée 2169). `-1` (Unknown) reste envoyé, comme PMMP et dragonfly ; seule l'icône de plateforme manque.
- **Inventaire géré par le serveur** (`ServerAuthoritativeInventory`) : change le comportement du client,
  à essayer avec un vrai client.
- **Couleur du joueur dans PlayerList** : `0xFFEDEDED` chez BDS pour un seul joueur ; fixe ou attribuée par
  joueur, à observer avec plusieurs joueurs.

Métadonnées du joueur encore différentes de BDS : 62 clés secondaires de BDS absentes (positions de lit et
de mort, commerce, sièges, visée assistée…), `SCALE` et boîte englobante (38, 53, 54) envoyées comme PMMP mais
plus par BDS, qui utilise `CollisionBox` (130). Les drapeaux de BDS (`Climb` seul, sans `Breathing` ni
`ShowName`) ont été relevés pendant que son bot était encore placé à y = 32 769, avant le vrai spawn : état
transitoire, non repris.

Documentation corrigée au passage : bits de drapeaux de `07-ENTITY-SYSTEM.md` (`CAN_CLIMB` 19 et non 16,
`NO_AI` 16 et non 15, `BABY` 11 et non 8), faim et règles de jeu de `99-ROADMAP.md`, points ouverts de
`31-PROTOCOL-2169.md` § 4, bugs connus de `CLAUDE.md`.

Écarts trouvés en chemin, non traités ici : les règles de jeu ne sont **appliquées nulle part** en jeu
(`keepInventory`, `naturalRegeneration`, `fallDamage`… sans effet) ; le `SkinResourcePatch` renvoyé par mc-rs
n'est pas l'octet près celui du client (fins de ligne), BDS le relaie tel quel.

## 6. Recettes importées de BDS (2026-10-25)

Toutes les recettes de mc-rs viennent désormais du CraftingData de BDS
(`scripts/import_bds_recipes.py` → `data/recipes/bds.json`, chargé par `recipes_bds.rs`), au lieu de
bedrock-samples. Les 1 796 recettes de bedrock-samples existent toutes chez BDS sous le même nom ; BDS en
ajoute 2 856 que le jeu génère par code. Détail du format et du comportement serveur :
[`30-VANILLA-DATA-IMPORT.md`](30-VANILLA-DATA-IMPORT.md).

Comparaison après import (`comparisons/2026-10-25_craftingdata_bds_vs_mc-rs-recettes-bds-2.txt`) : identifiants,
UUID, blocs, priorités, ingrédients, NBT, potions, forge, recettes multiples **identiques**. Restent deux écarts
assumés : la condition de déblocage (« toujours débloquée ») et le runtime ID de bloc des sorties (hash chez BDS,
séquentiel chez mc-rs). BDS ne met un bloc qu'aux sorties qui sont des items-blocs (pas aux lits, portes,
panneaux suspendus, feux de camp…) : la présence est reprise sortie par sortie.

Défauts corrigés au passage : les recettes du tailleur de pierre partaient avec le bloc `crafting_table` (elles
apparaissaient à l'établi) ; `CraftRecipe` ignorait l'identifiant de recette envoyé par le client.

**Mode typé du décodeur** (`bds-capture decode -typed`) : le JSON standard de gophertunnel perd les types NBT et
affiche `{}` pour tout champ optionnel. Une première lecture de la condition de déblocage (« absente chez BDS »)
venait de là et était **fausse** : BDS envoie une condition par recette (contexte « None » et les ingrédients
dont la possession débloque la recette ; aucune pour les recettes à données). Les comparaisons de champs
optionnels se font désormais sur des captures décodées en mode typé.

## 7. Décompilation (en cours)

Import et analyse automatique de `bedrock_server.exe` lancés le 2026-10-25 à 16:12 dans le projet Ghidra
(16 Go de tas, 12 cœurs, priorité basse ; journal `C:\tools\ghidra-projects\bds-1.26.52.3-analyze.log`).
Cibles : calcul des connexions (`minecraft:connection_*`) et des coins d'escaliers (`minecraft:corner`),
forme des faces de blocs, sérialisation des paquets restés douteux. Scripts prêts dans `scripts/dev/ghidra/`,
pas encore exécutés.
