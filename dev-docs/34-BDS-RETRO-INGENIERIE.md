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
| StartGame | 39 règles de jeu | aucune |
| StartGame | `ServerAuthoritativeInventory = true` | `false` |
| StartGame | `PlayerMovementSettings.RewindHistorySize = 40` | 0 |
| StartGame | `PlayerGameMode = 5` (mode du monde), `BaseGameVersion = "*"` | 0, `1.26.51` |
| PlayerList | UUID du joueur | UUID nul |
| PlayerList | `BuildPlatform`, version moteur de la géométrie : valeurs du client relayées | `-1`, `MINECRAFT_VERSION` forcés |
| PlayerList | `Trusted = false`, `PrimaryUser = false`, couleur (237, 237, 237) | `true`, `true`, blanc |
| UpdateAttributes | saturation par défaut 5, épuisement max 20, dégâts d'attaque 1–1, absorption max 16, résistance au recul min −2 ; `air_drag_modifier`, `bounciness`, `friction_modifier` | saturation 20, épuisement max 5, bornes 0–3,4e38 ; `fall_damage` en plus |
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

- Recette `minecraft:barrel` de BDS : gophertunnel lit `minecraft:stick` là où la recette vanilla utilise des
  planches. Lecture du descripteur par nom à vérifier avant d'en tirer une conclusion.
- `UpdateAdventureSettings.ShowNameTags` à `false` et `InventoryContent.Container.ContainerID` à 0 chez BDS :
  à confirmer avec un vrai client.

## 5. Décompilation (en cours)

Import et analyse automatique de `bedrock_server.exe` lancés le 2026-10-25 à 16:12 dans le projet Ghidra
(16 Go de tas, 12 cœurs, priorité basse ; journal `C:\tools\ghidra-projects\bds-1.26.52.3-analyze.log`).
Cibles : calcul des connexions (`minecraft:connection_*`) et des coins d'escaliers (`minecraft:corner`),
forme des faces de blocs, sérialisation des paquets restés douteux. Scripts prêts dans `scripts/dev/ghidra/`,
pas encore exécutés.
