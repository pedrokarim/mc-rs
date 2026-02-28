# 02 - Protocole Minecraft Bedrock Edition

## Vue d'ensemble des couches

```
┌─────────────────────────────────────┐
│   Game Packets (Login, StartGame,   │
│   MovePlayer, LevelChunk, etc.)     │
├─────────────────────────────────────┤
│   Batching (0xFE wrapper)           │
│   Plusieurs paquets par frame       │
├─────────────────────────────────────┤
│   Compression (zlib/snappy/none)    │
├─────────────────────────────────────┤
│   Encryption (AES-256-CFB8)         │
├─────────────────────────────────────┤
│   RakNet Framing                    │
│   (reliability, ordering, frag)     │
├─────────────────────────────────────┤
│   UDP Transport (port 19132)        │
└─────────────────────────────────────┘
```

## Types de données de sérialisation

### Types primitifs

| Type | Taille | Encodage |
|------|--------|----------|
| `byte` / `uint8` | 1 | Octet non signé |
| `bool` | 1 | `0x00` = false, `0x01` = true |
| `int16_le` | 2 | Little-endian signé |
| `uint16_le` | 2 | Little-endian non signé |
| `int32_le` | 4 | Little-endian signé |
| `uint32_le` | 4 | Little-endian non signé |
| `int64_le` | 8 | Little-endian signé |
| `uint64_le` | 8 | Little-endian non signé |
| `float32_le` | 4 | IEEE 754 little-endian |
| `float64_le` | 8 | IEEE 754 little-endian |
| `uint24_le` | 3 | Little-endian non signé (numéros de séquence RakNet) |

### VarInt / VarLong (LEB128)

Bedrock utilise un encodage à longueur variable de type LEB128 :

**VarUInt32 (non signé) :**
```
Pour chaque octet :
  value |= (byte & 0x7F) << (7 * num_read)
  Si (byte & 0x80) == 0 : terminé
Maximum : 5 octets pour 32 bits
```

**VarInt32 (signé) — ZigZag :**
```
Encodage : encoded = (value << 1) ^ (value >> 31)
Décodage : decoded = (encoded >> 1) ^ -(encoded & 1)
Puis écriture/lecture comme VarUInt32
```

**VarUInt64 / VarInt64 :** Même principe, jusqu'à 10 octets pour 64 bits.

### String
```
Longueur : VarUInt32 (nombre d'octets, PAS de caractères)
Données  : UTF-8
```

### BlockPosition
```
X : VarInt32 (signé, ZigZag)
Y : VarUInt32
Z : VarInt32 (signé, ZigZag)
```

### Vec3 (Vector3f)
```
X : float32_le (4 octets)
Y : float32_le (4 octets)
Z : float32_le (4 octets)
Total : 12 octets
```

### Vec2 (Vector2f)
```
X : float32_le
Z : float32_le
```

### UUID (Bedrock)
```
Part1 : int64_le (8 octets) — bits les plus significatifs
Part2 : int64_le (8 octets) — bits les moins significatifs
```
**Attention :** Bedrock utilise deux int64 LE, PAS l'ordre standard big-endian des UUID.

### NBT (Named Binary Tag) — Little-Endian

Bedrock utilise du NBT **little-endian** (Java utilise big-endian).

| ID | Type | Payload |
|----|------|---------|
| 0 | TAG_End | (rien) |
| 1 | TAG_Byte | 1 octet |
| 2 | TAG_Short | 2 octets (int16_le) |
| 3 | TAG_Int | 4 octets (int32_le) |
| 4 | TAG_Long | 8 octets (int64_le) |
| 5 | TAG_Float | 4 octets (float32_le) |
| 6 | TAG_Double | 8 octets (float64_le) |
| 7 | TAG_Byte_Array | int32_le longueur + octets |
| 8 | TAG_String | uint16_le longueur + UTF-8 |
| 9 | TAG_List | byte type_id + int32_le longueur + éléments |
| 10 | TAG_Compound | Tags nommés jusqu'à TAG_End |
| 11 | TAG_Int_Array | int32_le longueur + int32_le[] |
| 12 | TAG_Long_Array | int32_le longueur + int64_le[] |

**NBT Réseau :** Dans certains paquets, les `TAG_Int` et longueurs de tableaux sont encodés en **VarInt signé (ZigZag)** au lieu de int32_le fixe. Utilisé dans la plupart des paquets mais PAS dans les données de chunks.

### EntityMetadata (Synced Data)

```
Count : VarUInt32
Pour chaque entrée :
  Key   : VarUInt32 (ID de propriété)
  Type  : VarUInt32 (type de données)
  Value : (dépend du type)
```

**Types de données :**

| ID | Type | Format |
|----|------|--------|
| 0 | Byte | 1 octet |
| 1 | Short | int16_le |
| 2 | Int | VarInt signé |
| 3 | Float | float32_le |
| 4 | String | VarInt + UTF-8 |
| 5 | NBT | Compound tag (NBT réseau) |
| 6 | Vec3i | 3× VarInt signé |
| 7 | Int64 | VarLong signé |
| 8 | Vec3 | 3× float32_le |

### ItemStack (réseau)

```
Network ID : VarInt signé (0 = air/vide)
[Si network_id != 0] :
  Count     : uint16_le
  Damage    : VarUInt32
  Has NBT   : bool
  [Si has_nbt] :
    Version : byte (actuellement 1)
    NBT     : Compound tag (NBT réseau)
  CanPlaceOn  : String[] (VarInt count + strings)
  CanDestroy  : String[] (VarInt count + strings)
  [Si item est un bouclier] :
    Blocking tick : int64_le
```

---

## Séquence de connexion complète

### Phase 1 : RakNet Offline (avant connexion)

```
Client                          Serveur
  │                                │
  │── UnconnectedPing ────────────>│  (découverte serveur)
  │<── UnconnectedPong ────────────│  (MOTD, joueurs, version)
  │                                │
  │── OpenConnectionRequest1 ─────>│  (MTU discovery, MAGIC)
  │<── OpenConnectionReply1 ───────│  (MTU accepté)
  │                                │
  │── OpenConnectionRequest2 ─────>│  (port client, MTU final)
  │<── OpenConnectionReply2 ───────│  (connexion RakNet établie)
```

### Phase 2 : RakNet Online

```
  │── ConnectionRequest ──────────>│  (timestamp, challenge)
  │<── ConnectionRequestAccepted ──│
  │── NewIncomingConnection ──────>│
```

### Phase 3 : Négociation réseau

```
  │── RequestNetworkSettings ─────>│  (protocol_version: int32 BE)
  │<── NetworkSettings ────────────│  (compression algo, seuil)
  │                                │
  │   [Compression activée à partir d'ici]
```

### Phase 4 : Authentification

```
  │── LoginPacket ────────────────>│  (JWT chain + client data)
  │                                │  [Serveur valide Xbox Live JWT]
  │                                │
  │<── ServerToClientHandshake ────│  (clé publique serveur pour ECDH)
  │── ClientToServerHandshake ────>│
  │                                │
  │   [Chiffrement AES-256-CFB8 activé]
  │                                │
  │<── PlayStatus(LoginSuccess) ───│
```

### Phase 5 : Resource Packs

```
  │<── ResourcePacksInfo ──────────│  (liste des packs requis)
  │── ResourcePackClientResponse ─>│  (HaveAllPacks / SendPacks)
  │                                │
  │   [Si packs à télécharger : boucle de download]
  │                                │
  │<── ResourcePackStack ──────────│  (ordre d'application)
  │── ResourcePackClientResponse ─>│  (Completed)
```

### Phase 6 : Initialisation du monde

```
  │<── StartGame ──────────────────│  (ÉNORME paquet : config monde,
  │                                │   block palette, items, game rules,
  │                                │   position spawn, etc.)
  │<── CreativeContent ────────────│  (items créatifs)
  │<── BiomeDefinitionList ────────│  (définitions biomes)
  │<── AvailableEntityIdentifiers ─│  (types d'entités)
  │<── AvailableCommands ──────────│  (arbre de commandes)
  │<── CraftingData ───────────────│  (recettes)
  │<── ItemComponent ──────────────│  (items custom)
  │                                │
  │── RequestChunkRadius ─────────>│  (distance de rendu souhaitée)
  │<── ChunkRadiusUpdated ─────────│  (distance acceptée)
  │<── NetworkChunkPublisherUpdate │  (zone de chunks disponible)
  │                                │
  │<── LevelChunk (×N) ───────────│  (données de chunks)
  │<── LevelChunk (×N) ───────────│
  │       ...                      │
  │                                │
  │── SetLocalPlayerAsInitialized >│
  │<── PlayStatus(PlayerSpawn) ────│  (le joueur peut jouer !)
```

---

## Table des paquets principaux

### Paquets de login/initialisation

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x01` | Login | C→S | JWT d'authentification + données skin |
| `0x02` | PlayStatus | S→C | Résultat du login (succès/échec/spawn) |
| `0x03` | ServerToClientHandshake | S→C | Clé publique pour chiffrement |
| `0x04` | ClientToServerHandshake | C→S | Confirmation chiffrement |
| `0x05` | Disconnect | S→C | Déconnexion avec message |
| `0x06` | ResourcePacksInfo | S→C | Packs disponibles |
| `0x07` | ResourcePackStack | S→C | Ordre des packs |
| `0x08` | ResourcePackClientResponse | C→S | Réponse client (accepted/need) |
| `0x0B` | StartGame | S→C | Initialisation monde (paquet massif) |
| `0x8F` | NetworkSettings | S→C | Config compression |
| `0xC1` | RequestNetworkSettings | C→S | Version protocole |

### Paquets de mouvement

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x13` | MovePlayer | Both | Position/rotation du joueur |
| `0x12` | MoveEntityAbsolute | S→C | Position absolue d'entité |
| `0x70` | MoveEntityDelta | S→C | Delta de position d'entité |
| `0x90` | PlayerAuthInput | C→S | Input joueur (mode server-auth) |

### Paquets d'entités

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x0C` | AddPlayer | S→C | Spawn d'un joueur |
| `0x0D` | AddEntity | S→C | Spawn d'une entité |
| `0x0E` | RemoveEntity | S→C | Despawn entité |
| `0x0F` | AddItemEntity | S→C | Spawn item au sol |
| `0x11` | TakeItemEntity | S→C | Ramasser un item |
| `0x1B` | EntityEvent | Both | Événement entité |
| `0x1D` | UpdateAttributes | S→C | Attributs (vie, vitesse...) |
| `0x27` | SetEntityData | S→C | Metadata entité |
| `0x28` | SetEntityMotion | S→C | Vélocité entité |
| `0x29` | SetEntityLink | S→C | Liaison (monture, laisse) |

### Paquets de monde/blocs

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x15` | UpdateBlock | S→C | Mise à jour d'un bloc |
| `0x1A` | BlockEvent | S→C | Événement bloc (piston, coffre) |
| `0x38` | BlockEntityData | S→C | Données bloc-entité (NBT) |
| `0x3A` | LevelChunk | S→C | Données d'un chunk |
| `0x19` | LevelEvent | S→C | Événement monde (particules, son) |
| `0x7A` | NetworkChunkPublisherUpdate | S→C | Zone de chunks disponible |

### Paquets d'inventaire

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x1E` | InventoryTransaction | C→S | Transaction d'inventaire (legacy) |
| `0x1F` | MobEquipment | Both | Item en main |
| `0x20` | MobArmorEquipment | Both | Armure |
| `0x2E` | ContainerOpen | S→C | Ouvrir un conteneur |
| `0x2F` | ContainerClose | Both | Fermer un conteneur |
| `0x31` | InventoryContent | S→C | Contenu complet inventaire |
| `0x32` | InventorySlot | S→C | Mise à jour slot |
| `0x93` | ItemStackRequest | C→S | Requête inventaire (modern) |
| `0x94` | ItemStackResponse | S→C | Réponse inventaire (modern) |

### Paquets de gameplay

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x09` | Text | Both | Message chat |
| `0x24` | PlayerAction | C→S | Action joueur (casser, nager...) |
| `0x2C` | Animate | Both | Animation joueur |
| `0x2D` | Respawn | Both | Respawn joueur |
| `0x21` | Interact | C→S | Interaction entité |
| `0x3E` | SetPlayerGameType | S→C | Changer gamemode |
| `0x3F` | PlayerList | S→C | Liste des joueurs (tab) |
| `0x2A` | SetHealth | S→C | Vie du joueur |
| `0x2B` | SetSpawnPosition | S→C | Position de spawn |
| `0x56` | Transfer | S→C | Transfert vers autre serveur |
| `0x59` | SetTitle | S→C | Titre/sous-titre |

### Paquets de commandes

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x4D` | AvailableCommands | S→C | Arbre complet des commandes |
| `0x4E` | CommandRequest | C→S | Exécution de commande |
| `0x50` | CommandOutput | S→C | Résultat de commande |

### Paquets de formulaires (UI Bedrock)

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x65` | ModalFormRequest | S→C | Envoi d'un formulaire |
| `0x66` | ModalFormResponse | C→S | Réponse du joueur |
| `0x67` | ServerSettingsRequest | C→S | Demande settings serveur |
| `0x68` | ServerSettingsResponse | S→C | Réponse settings |

### Paquets de scoreboard

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x6B` | RemoveObjective | S→C | Supprimer objectif |
| `0x6C` | SetDisplayObjective | S→C | Afficher objectif |
| `0x6D` | SetScore | S→C | Mettre à jour scores |
| `0x71` | SetScoreboardIdentity | S→C | Identité scoreboard |

### Paquets divers importants

| ID | Nom | Direction | Description |
|----|-----|-----------|-------------|
| `0x34` | CraftingData | S→C | Toutes les recettes |
| `0x45` | RequestChunkRadius | C→S | Distance de rendu |
| `0x46` | ChunkRadiusUpdated | S→C | Distance acceptée |
| `0x49` | GameRulesChanged | S→C | Règles du jeu |
| `0x4B` | BossEvent | S→C | Barre de boss |
| `0x57` | PlaySound | S→C | Jouer un son |
| `0x5E` | PlayerSkin | Both | Changement de skin |
| `0x74` | NetworkStackLatency | Both | Mesure de ping |
| `0x78` | AvailableEntityIdentifiers | S→C | Types d'entités |
| `0x7B` | BiomeDefinitionList | S→C | Définitions biomes |
| `0x91` | CreativeContent | S→C | Items mode créatif |
| `0x9C` | PacketViolationWarning | C→S | Erreur de protocole |
| `0xA3` | ItemComponent | S→C | Composants items custom |
| `0xB9` | ToastRequest | S→C | Notification toast |
| `0xBB` | UpdateAbilities | S→C | Capacités joueur |

---

## Détail des paquets clés

### StartGame (0x0B)

C'est le paquet le plus gros et le plus complexe. Il contient toute la configuration du monde :

```
EntityUniqueID           : VarLong signé
EntityRuntimeID          : VarULong
PlayerGamemode           : VarInt signé
PlayerPosition           : Vec3
Rotation                 : Vec2 (pitch, yaw)
Seed                     : uint64_le
BiomeType                : int16_le
BiomeName                : String
Dimension                : VarInt signé (0=Overworld, 1=Nether, 2=End)
Generator                : VarInt signé (0=legacy, 1=overworld, 2=flat, 3=nether, 4=end, 5=void)
WorldGamemode            : VarInt signé
Difficulty               : VarInt signé (0=Peaceful, 1=Easy, 2=Normal, 3=Hard)
SpawnPosition            : BlockPosition
AchievementsDisabled     : bool
EditorWorldType          : VarInt signé
CreatedInEditor          : bool
ExportedFromEditor       : bool
DayCycleStopTime         : VarInt signé
EduOffer                 : VarInt signé
EduFeaturesEnabled       : bool
EduProductUUID           : String
RainLevel                : float32_le
LightningLevel           : float32_le
HasConfirmedPlatformLockedContent : bool
IsMultiplayer            : bool
BroadcastToLAN           : bool
XboxLiveBroadcastMode    : VarUInt
PlatformBroadcastMode    : VarUInt
EnableCommands           : bool
AreTexturePacksRequired  : bool
GameRules                : GameRule[]
Experiments              : Experiment[]
ExperimentsPreviouslyUsed : bool
BonusChest               : bool
MapEnabled               : bool
PermissionLevel          : VarInt signé
ServerChunkTickRange     : int32_le
HasLockedBehaviorPack    : bool
HasLockedResourcePack    : bool
IsFromLockedWorldTemplate : bool
MsaGamertagsOnly        : bool
IsFromWorldTemplate      : bool
IsWorldTemplateOptionLocked : bool
OnlySpawnV1Villagers     : bool
PersonaDisabled          : bool
CustomSkinsDisabled      : bool
EmoteChatMuted           : bool
GameVersion              : String
LimitedWorldWidth        : int32_le
LimitedWorldLength       : int32_le
IsNewNether              : bool
EduResourceURI           : { ButtonName: String, LinkURI: String }
ExperimentalGameplayOverride : bool
ChatRestrictionLevel     : byte
DisablePlayerInteractions : bool
ServerIdentifier         : String
WorldIdentifier          : String
ScenarioIdentifier       : String
LevelID                  : String
WorldName                : String
PremiumWorldTemplateID   : String
IsTrial                  : bool
MovementSettings         : { AuthType: VarInt, RewindHistorySize: VarInt, ServerAuthBlockBreaking: bool }
CurrentTick              : int64_le
EnchantmentSeed          : VarInt signé
BlockProperties          : NBT entries[] (palette de blocs custom)
ItemTable                : { StringID, NumericID, IsComponentBased }[]
MultiplayerCorrelationID : String
ServerAuthoritativeInventory : bool
GameEngine               : String ("vanilla")
PropertyData             : NBT Compound
BlockPaletteChecksum     : uint64_le
WorldTemplateID          : UUID
ClientSideGeneration     : bool
BlockNetworkIDsAreHashes : bool
ServerControlledSounds   : bool
```

### PlayerAuthInput (0x90) — Mouvement server-authoritative

```
Rotation         : Vec2 (pitch, yaw)
Position         : Vec3
MoveVector       : Vec2 (input analogique)
HeadYaw          : float32_le
InputData        : VarULong (bitflags, voir ci-dessous)
InputMode        : VarUInt (0=mouse, 1=touch, 2=gamepad, 3=motion_controller)
PlayMode         : VarUInt (0=normal, 1=teaser, 2=screen, 3=viewer, 4=VR, 5=placement, 6=living_room, 7=exit_level, 8=exit_level_living_room)
InteractionModel : VarUInt
InteractRotation : Vec2
Tick             : VarULong
PositionDelta    : Vec3
[+ données conditionnelles selon les bitflags]
```

**InputData Bitflags principaux :**

| Bit | Nom | Description |
|-----|-----|-------------|
| 0 | Ascend | Monter |
| 1 | Descend | Descendre |
| 3 | JumpDown | Saut appuyé |
| 4 | SprintDown | Sprint appuyé |
| 6 | Jumping | En train de sauter |
| 8 | Sneaking | Accroupi |
| 10-13 | Up/Down/Left/Right | Direction mouvement |
| 20 | Sprinting | En sprint |
| 25 | StartSprinting | Début sprint |
| 26 | StopSprinting | Fin sprint |
| 27 | StartSneaking | Début accroupi |
| 28 | StopSneaking | Fin accroupi |
| 29 | StartSwimming | Début nage |
| 32 | StartGliding | Début élytra |
| 34 | PerformItemInteraction | Interaction item |
| 35 | PerformBlockActions | Actions bloc |
| 36 | PerformItemStackRequest | Requête inventaire |

### LevelChunk (0x3A)

```
ChunkX          : VarInt signé
ChunkZ          : VarInt signé
DimensionID     : VarInt signé
SubChunkCount   : VarUInt
CacheEnabled    : bool
[Si CacheEnabled] :
  BlobCount     : VarUInt
  BlobHashes    : uint64_le[]
[Sinon] :
  RawPayload    : ByteArray (SubChunks + biomes + border blocks)
```

### Text (0x09)

```
Type             : byte (0=RAW, 1=CHAT, 2=TRANSLATION, 3=POPUP, 4=JUKEBOX, 5=TIP, 6=SYSTEM, 7=WHISPER, 8=ANNOUNCEMENT)
NeedsTranslation : bool
[Selon type] :
  SourceName     : String (types 0,1,7,8)
  Message        : String
  Parameters     : String[] (types 2,3,4)
XUID             : String
PlatformChatID   : String
FilteredMessage  : String
```

---

## PlayStatus Values

| Valeur | Nom | Signification |
|--------|-----|---------------|
| 0 | LoginSuccess | Login accepté |
| 1 | LoginFailedClient | Client trop ancien |
| 2 | LoginFailedServer | Serveur trop ancien |
| 3 | PlayerSpawn | Signal pour spawn le joueur |
| 4 | LoginFailedInvalidTenant | |
| 7 | LoginFailedServerFull | Serveur plein |

---

## Versioning du protocole

### Fonctionnement

Deux numéros de version coexistent :
1. **Version du jeu** : Lisible (ex : `1.26.0`), visible par les joueurs
2. **Version du protocole** : Entier incrémental, utilisé par le réseau

Le client envoie sa version protocole dans `RequestNetworkSettings`. Le serveur compare et rejette les incompatibilités.

### Historique des versions récentes

| Version Jeu | Protocole |
|-------------|-----------|
| 1.18.0 | 503 |
| 1.19.0 | 544 |
| 1.19.50 | 575 |
| 1.20.0 | 589 |
| 1.20.30 | 618 |
| 1.20.50 | 630 |
| 1.20.70 | 662 |
| 1.20.80 | 671 |
| 1.21.0 | 685 |
| 1.21.20 | 712 |
| 1.21.30 | 729 |
| 1.21.40 | 748 |
| 1.21.50 | 766 |
| 1.26.0 | 924 |

### Compatibilité

- Les serveurs Bedrock ne sont généralement **PAS** rétro-compatibles entre versions protocole
- Le paquet `StartGame` grandit à chaque version (nouveaux champs ajoutés à la fin)
- Les serveurs tiers supportent typiquement 1-2 versions protocole maximum
- **Stratégie MC-RS :** Cibler la dernière version stable, avec abstraction pour faciliter les mises à jour

---

## Block Runtime IDs et Palette

Bedrock utilise des **Runtime Block State IDs** au lieu d'IDs numériques fixes :

- Chaque état de bloc unique (ex : `minecraft:oak_stairs[facing=north,half=top]`) a un runtime ID unique
- Le mapping complet est envoyé dans `StartGame` comme **block palette**
- La palette contient typiquement **15 000+ entrées**
- Les runtime IDs changent entre versions
- Depuis 1.19.80+ : `BlockNetworkIDsAreHashes = true` → Les IDs sont des hash FNV-1 32-bit de l'état NBT du bloc

**Source de données :** Le dépôt `pmmp/BedrockData` contient les palettes canoniques extraites de BDS.

---

## Système de cache client (Blob Cache)

Optimisation optionnelle pour réduire la bande passante :

1. Client envoie `ClientCacheStatus` (`supported: bool`)
2. Si activé, le serveur envoie des hash de blobs dans `LevelChunk` au lieu des données complètes
3. Le client répond avec `ClientCacheBlobStatus` (blobs qu'il a / qu'il manque)
4. Le serveur envoie uniquement les blobs manquants via `ClientCacheMissResponse`

Réduit significativement la bande passante pour les joueurs qui reviennent.

---

## Mouvement Server-Authoritative

Depuis ~1.16.100, trois modes :

| Mode | Nom | Description |
|------|-----|-------------|
| 0 | ClientAuthoritative | Client envoie sa position, serveur fait confiance |
| 1 | ServerAuthoritative | Client envoie ses inputs, serveur simule |
| 2 | ServerAuthWithRewind | Server-auth avec correction côté client |

Configuré via `StartGame.MovementSettings.AuthType` et `SetMovementAuthority` (0x148).

En mode server-authoritative :
- Le client envoie `PlayerAuthInput` (0x90) **chaque tick** (20× par seconde)
- Le serveur valide et peut envoyer `CorrectPlayerMovePrediction` pour corriger
- `MovePlayer` avec `mode=Reset` force la position

---

## Système ItemStackRequest (inventaire moderne)

| ID Action | Nom | Description |
|-----------|-----|-------------|
| 0 | Take | Prendre des items |
| 1 | Place | Placer des items |
| 2 | Swap | Échanger des items |
| 3 | Drop | Jeter des items |
| 4 | Destroy | Détruire des items |
| 5 | Consume | Consommer |
| 6 | Create | Créer (créatif) |
| 10 | BeaconPayment | Paiement beacon |
| 11 | MineBlock | Minage de bloc |
| 12 | CraftRecipe | Craft normal |
| 13 | CraftRecipeAuto | Craft automatique |
| 14 | CraftCreative | Craft créatif |
| 15 | CraftRecipeOptional | Craft optionnel |
| 16 | CraftGrindstone | Meule |
| 17 | CraftLoom | Métier à tisser |

Chaque item a un `StackNetworkId` assigné par le serveur pour le tracking.

---

## Resource Packs Protocol

### ResourcePacksInfo (0x06)
```
MustAcceptPacksForSkins : bool
ScriptingEnabled        : bool (deprecated)
ForcingServerPacks      : bool
BehaviorPacks           : [{ UUID, Version, Size, ContentKey, SubPackName, ContentIdentity, HasScripts }]
ResourcePacks           : [{ UUID, Version, Size, ContentKey, SubPackName, ContentIdentity, HasScripts, RTXEnabled }]
CDNUrls                 : [{ PackID, RemoteURL }]
```

### ResourcePackClientResponse (0x08) Status values
| Valeur | Signification |
|--------|---------------|
| 0 | None |
| 1 | Refused |
| 2 | SendPacks (client veut télécharger) |
| 3 | HaveAllPacks |
| 4 | Completed |

---

## AvailableCommands (0x4D)

L'un des paquets les plus complexes — contient l'arbre complet des commandes :

```
Values[]          : String[] (pool de valeurs enum)
Suffixes[]        : String[]
Enums[]           : { Name: String, ValueIndices: VarInt[] }
Commands[]        :
  Name            : String
  Description     : String
  Flags           : uint16_le
  PermissionLevel : byte
  AliasEnum       : int32_le (-1 si aucun)
  Overloads[]     :
    Chained       : bool
    Params[]      :
      Name        : String
      Type        : uint32_le (type + flags)
      Optional    : bool
      Options     : byte
DynamicEnums[]    : { Name, Values[] }
EnumConstraints[] : { EnumValueIndex, EnumIndex, Constraints[] }
```
