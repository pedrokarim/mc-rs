# Spécification Complète du Protocole — mc-rs (Protocol 924)

Basé à 100% sur PocketMine-MP + BedrockProtocol.

## Vue d'ensemble

```
Client                                    Server
  │                                          │
  ├── RequestNetworkSettings(924) ──────────>│
  │<──────────── NetworkSettings ────────────┤  (compression activée)
  │                                          │
  ├── Login(tokens) ────────────────────────>│
  │<──────────── PlayStatus(LOGIN_SUCCESS) ──┤
  │<──────────── ResourcePacksInfo ──────────┤
  │                                          │
  ├── ResourcePackClientResponse(HAVE_ALL) ─>│
  │<──────────── ResourcePackStack ──────────┤
  │                                          │
  ├── ResourcePackClientResponse(COMPLETED) >│
  │<──────────── [Pre-Spawn Burst] ──────────┤  (12-15 paquets)
  │                                          │
  ├── RequestChunkRadius(N) ────────────────>│
  │<──────────── ChunkRadiusUpdated ─────────┤
  │<──────────── NetworkChunkPublisherUpdate ─┤
  │<──────────── LevelChunk × ~50-80 ────────┤
  │<──────────── PlayStatus(PLAYER_SPAWN) ───┤
  │                                          │
  ├── SetLocalPlayerAsInitialized ──────────>│
  │                                          │  *** JOUEUR EN JEU ***
```

---

## 1. RequestNetworkSettings (0xC1) — Client → Server

**Avant compression** (batch non compressé)

| Champ | Type | Description |
|-------|------|-------------|
| protocolVersion | i32 BE | 924 |

---

## 2. NetworkSettings (0x8F) — Server → Client

**Avant compression** (envoyé en clair, compression s'active APRÈS ce paquet)

| Champ | Type | Valeur PocketMine |
|-------|------|-------------------|
| compressionThreshold | u16 BE | 1 |
| compressionAlgorithm | u16 BE | 1 (Snappy) |
| clientThrottleEnabled | bool | false |
| clientThrottleThreshold | u8 | 0 |
| clientThrottleScalar | f32 BE | 0.0 |

**IMPORTANT**: Après l'envoi, activer Snappy pour TOUS les paquets suivants.

---

## 3. Login (0x01) — Client → Server

| Champ | Type |
|-------|------|
| protocolVersion | i32 BE |
| chainData | ByteArray (VarUInt32 len + data) |

Le serveur doit :
- Parser les JWT tokens (chain + client data)
- Extraire le nom du joueur et les infos de skin
- Pour le MVP : ignorer la validation, accepter tout

---

## 4. PlayStatus (0x02) — Server → Client

| Champ | Type | Valeurs |
|-------|------|---------|
| status | i32 BE | 0=LOGIN_SUCCESS, 3=PLAYER_SPAWN |

---

## 5. ResourcePacksInfo (0x06) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| mustAccept | bool | false |
| hasAddons | bool | false |
| hasScripts | bool | false |
| forceDisableVibrantVisuals | bool | false |
| worldTemplateUUID | 16 bytes | UUID_NIL |
| worldTemplateVersion | String | "" |
| packCount | u16 LE | 0 |

---

## 6. ResourcePackClientResponse (0x08) — Client → Server

| Champ | Type |
|-------|------|
| status | u8 | 3=HAVE_ALL_PACKS, 4=COMPLETED |
| packIds | VarUInt32(count) + String[] |

---

## 7. ResourcePackStack (0x07) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| mustAccept | bool | false |
| stackCount | VarUInt32 | 0 |
| gameVersion | String | "*" |
| experimentCount | u32 LE | 0 |
| hasPreviouslyUsedExperiments | bool | false |
| useVanillaEditorPacks | bool | false |

---

## 8. StartGame (0x0B) — Server → Client

Voir `02-STARTGAME.md` pour la structure complète (80 champs).

Points critiques :
- `blockNetworkIdsAreHashes = false` (PocketMine)
- `blockPalette = []` (vide, count=0)
- `blockPaletteChecksum = 0`
- `enableClientSideChunkGeneration = false`
- `currentTick = 0` (u64 LE)

---

## 9. ItemRegistry (0xA2) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| count | VarUInt32 | 0 (vide pour MVP) |

Pour un serveur complet : chaque entrée = String(name) + i16 LE(runtime_id) + bool(component_based).

---

## 10. AvailableActorIdentifiers (0x77) — Server → Client

Blob NBT réseau brut (fichier `entity_identifiers.nbt` de BedrockData).

---

## 11. BiomeDefinitionList (0x7A) — Server → Client

Format binaire structuré (protocol 924, PAS NBT) :
1. VarUInt32(count)
2. Pour chaque biome: u16 LE(nameIdx), u16 LE(id), f32 LE(temp), f32 LE(downfall), f32 LE(foliageSnow), f32 LE(depth), f32 LE(scale), u32 LE(waterColorARGB), bool(rain), Optional<tags>, Optional<chunkGenData>
3. VarUInt32(stringCount) + string table

---

## 12. UpdateAttributes (0x1D) — Server → Client

| Champ | Type |
|-------|------|
| actorRuntimeId | UnsignedVarLong |
| attributeCount | VarUInt32 |
| (par attribut) min | f32 LE |
| (par attribut) max | f32 LE |
| (par attribut) current | f32 LE |
| (par attribut) default | f32 LE |
| (par attribut) name | String |
| (par attribut) modifierCount | VarUInt32 |
| tick | UnsignedVarLong |

Attributs PocketMine par défaut : health, hunger, movement, follow_range, saturation, exhaustion, level, experience, absorption, luck.

---

## 13. AvailableCommands (0x4C) — Server → Client

Pour le MVP : 8 VarUInt32 à 0 (tout vide).

---

## 14. UpdateAbilities (0xBB) — Server → Client

| Champ | Type |
|-------|------|
| targetActorUniqueId | u64 LE |
| playerPermission | u8 (1=MEMBER) |
| commandPermission | u8 (0=NORMAL) |
| layerCount | u8 |
| (par layer) layerType | u16 LE |
| (par layer) abilitiesSet | u32 LE |
| (par layer) abilityValues | u32 LE |
| (par layer) flySpeed | f32 LE |
| (par layer) walkSpeed | f32 LE |

PocketMine envoie 2 layers : BASE + CUSTOM_CACHE.

---

## 15. UpdateAdventureSettings (0xBC) — Server → Client

5 booleans : noPvM, noMvP, immutableWorld, showNameTags, autoJump. Tous false.

---

## 16. SetActorData (0x27) — Server → Client

| Champ | Type |
|-------|------|
| actorRuntimeId | UnsignedVarLong |
| metadataCount | VarUInt32 |
| (par entry) key | VarUInt32 |
| (par entry) type | VarUInt32 |
| (par entry) value | selon type |
| intPropertiesCount | VarUInt32 |
| floatPropertiesCount | VarUInt32 |
| tick | UnsignedVarLong |

Metadata types : 0=Byte, 1=Short, 2=Int, 3=Float, 4=String, 5=CompoundTag, 6=BlockPos, 7=Long, 8=Vec3.

---

## 17. CreativeContent (0x91) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| count | VarUInt32 | 0 |

---

## 18. CraftingData (0x34) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| recipeCount | VarUInt32 | 0 |
| potionTypeCount | VarUInt32 | 0 |
| potionContainerCount | VarUInt32 | 0 |
| materialReducerCount | VarUInt32 | 0 |
| isClean | bool | true |

---

## 19. PlayerList (0x3F) — Server → Client

| Champ | Type |
|-------|------|
| type | u8 (0=ADD) |
| entryCount | VarUInt32 |
| (par entry) uuid | 16 bytes |
| (par entry) actorUniqueId | SignedVarLong |
| (par entry) username | String |
| (par entry) xuid | String |
| (par entry) platformChatId | String |
| (par entry) buildPlatform | i32 LE |
| (par entry) skinData | SkinData (complexe) |
| (par entry) isTeacher | bool |
| (par entry) isHost | bool |
| (par entry) isSubClient | bool |

---

## 20. RequestChunkRadius (0x45) — Client → Server

| Champ | Type |
|-------|------|
| radius | SignedVarInt |

---

## 21. ChunkRadiusUpdated (0x46) — Server → Client

| Champ | Type |
|-------|------|
| radius | SignedVarInt |

---

## 22. NetworkChunkPublisherUpdate (0x79) — Server → Client

| Champ | Type |
|-------|------|
| position.x | SignedVarInt |
| position.y | SignedVarInt |
| position.z | SignedVarInt |
| radius | VarUInt32 (en blocs = viewDistance × 16) |
| savedChunksCount | u32 LE (0) |

---

## 23. LevelChunk (0x3A) — Server → Client

| Champ | Type |
|-------|------|
| chunkX | SignedVarInt |
| chunkZ | SignedVarInt |
| dimensionId | SignedVarInt |
| subChunkCount | VarUInt32 |
| cacheEnabled | bool (false) |
| payloadLen | VarUInt32 |
| payload | bytes |

Voir `03-CHUNK-FORMAT.md` pour le détail du payload.

---

## 24. PlayStatus(PLAYER_SPAWN) (0x02) — Server → Client

| Champ | Type | Valeur |
|-------|------|--------|
| status | i32 BE | 3 |

Envoyé APRÈS suffisamment de chunks (PocketMine: ~50 chunks).

---

## 25. SetLocalPlayerAsInitialized (0x71) — Client → Server

| Champ | Type |
|-------|------|
| actorRuntimeId | UnsignedVarLong |

Le client envoie ce paquet UNIQUEMENT quand il a fini de charger le monde.
→ Transition vers InGame.

---

## Résumé — Checklist d'implémentation

- [x] Packet framing : `VarUInt32(packet_id)` (PAS `<< 2`)
- [x] Compression Snappy après NetworkSettings
- [x] RequestNetworkSettings → NetworkSettings
- [x] Login → PlayStatus(LOGIN_SUCCESS) + ResourcePacksInfo
- [x] ResourcePackClientResponse(3) → ResourcePackStack
- [x] ResourcePackClientResponse(4) → Pre-Spawn burst
- [x] StartGame (blockNetworkIdsAreHashes=false, palette vide)
- [x] 11 paquets pre-spawn dans l'ordre exact PocketMine
- [x] RequestChunkRadius → ChunkRadiusUpdated + NetworkChunkPublisherUpdate
- [x] LevelChunk × N (bpb >= 1, bpe >= 1, zigzag palette counts)
- [x] PlayStatus(PLAYER_SPAWN)
- [ ] Validation complète des chunks (runtime IDs corrects)
- [ ] ItemRegistry avec les items réels
- [ ] Skin data complète dans PlayerList
