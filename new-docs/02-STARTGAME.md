# StartGamePacket — Structure Complète (Protocol 924)

Basé sur BedrockProtocol / PocketMine-MP.

## Champs dans l'ordre exact

| # | Champ | Encodage | Valeur par défaut |
|---|-------|----------|-------------------|
| 1 | actorUniqueId | SignedVarLong (zigzag i64) | 1 |
| 2 | actorRuntimeId | UnsignedVarLong | 1 |
| 3 | playerGamemode | SignedVarInt (zigzag i32) | 1 (creative) |
| 4 | playerPosition | Vec3 (3× f32 LE) | (0.0, 64.0, 0.0) |
| 5 | pitch | f32 LE | 0.0 |
| 6 | yaw | f32 LE | 0.0 |

### LevelSettings (inline)

| # | Champ | Encodage | Valeur |
|---|-------|----------|--------|
| 7 | seed | u64 LE | 0 |
| 8 | spawnSettings.biomeType | u16 LE | 0 |
| 9 | spawnSettings.biomeName | String (varuint len + utf8) | "" |
| 10 | spawnSettings.dimension | SignedVarInt | 0 (Overworld) |
| 11 | generator | SignedVarInt | 1 (OVERWORLD) |
| 12 | worldGamemode | SignedVarInt | 1 |
| 13 | hardcore | bool | false |
| 14 | difficulty | SignedVarInt | 1 (Easy) |
| 15 | spawnPosition | BlockPos (svari32 x, uvari32 y, svari32 z) | (0, 64, 0) |
| 16 | hasAchievementsDisabled | bool | true |
| 17 | editorWorldType | SignedVarInt | 0 |
| 18 | createdInEditorMode | bool | false |
| 19 | exportedFromEditorMode | bool | false |
| 20 | time | SignedVarInt | 0 |
| 21 | eduEditionOffer | SignedVarInt | 0 |
| 22 | hasEduFeaturesEnabled | bool | false |
| 23 | eduProductUUID | String | "" |
| 24 | rainLevel | f32 LE | 0.0 |
| 25 | lightningLevel | f32 LE | 0.0 |
| 26 | hasConfirmedPlatformLockedContent | bool | false |
| 27 | isMultiplayerGame | bool | true |
| 28 | hasLANBroadcast | bool | true |
| 29 | xboxLiveBroadcastMode | SignedVarInt | 0 |
| 30 | platformBroadcastMode | SignedVarInt | 0 |
| 31 | commandsEnabled | bool | true |
| 32 | isTexturePacksRequired | bool | true |
| 33 | gameRules | GameRules | voir ci-dessous |
| 34 | experiments | Experiments | count=0, previouslyUsed=false |
| 35 | hasBonusChestEnabled | bool | false |
| 36 | hasStartWithMapEnabled | bool | false |
| 37 | defaultPlayerPermission | SignedVarInt | 1 (MEMBER) |
| 38 | serverChunkTickRadius | i32 LE | 4 |
| 39 | hasLockedBehaviorPack | bool | false |
| 40 | hasLockedResourcePack | bool | false |
| 41 | isFromLockedWorldTemplate | bool | false |
| 42 | useMsaGamertagsOnly | bool | false |
| 43 | isFromWorldTemplate | bool | false |
| 44 | isWorldTemplateOptionLocked | bool | false |
| 45 | onlySpawnV1Villagers | bool | false |
| 46 | disablePersona | bool | false |
| 47 | disableCustomSkins | bool | false |
| 48 | muteEmoteAnnouncements | bool | false |
| 49 | vanillaVersion | String | "1.26.0" |
| 50 | limitedWorldWidth | i32 LE | 0 |
| 51 | limitedWorldLength | i32 LE | 0 |
| 52 | isNewNether | bool | true |
| 53 | eduSharedUriResource.buttonName | String | "" |
| 54 | eduSharedUriResource.linkUri | String | "" |
| 55 | experimentalGameplayOverride | Optional bool | None (0x00) |
| 56 | chatRestrictionLevel | u8 | 0 |
| 57 | disablePlayerInteractions | bool | false |

### Fin LevelSettings, retour StartGame

| # | Champ | Encodage | Valeur |
|---|-------|----------|--------|
| 58 | levelId | String | "" |
| 59 | worldName | String | "mc-rs" |
| 60 | premiumWorldTemplateId | String | "" |
| 61 | isTrial | bool | false |
| 62 | playerMovementSettings.rewindHistorySize | SignedVarInt | 0 |
| 63 | playerMovementSettings.serverAuthBlockBreaking | bool | true |
| 64 | currentTick | u64 LE | 0 |
| 65 | enchantmentSeed | SignedVarInt | 0 |
| 66 | blockPalette | UnsignedVarInt(count) + entries | count=0 (palette vide) |
| 67 | multiplayerCorrelationId | String | "" |
| 68 | enableNewInventorySystem | bool | true |
| 69 | serverSoftwareVersion | String | "mc-rs 0.1.0" |
| 70 | playerActorProperties | NBT Compound (network LE) | CompoundTag vide |
| 71 | blockPaletteChecksum | u64 LE | 0 |
| 72 | worldTemplateId | UUID (16 bytes) | 00000000-0000-0000-0000-000000000000 |
| 73 | enableClientSideChunkGeneration | bool | false |
| 74 | blockNetworkIdsAreHashes | bool | false |
| 75 | networkPermissions.disableClientSounds | bool | true |
| 76 | serverJoinInformation | Optional | None (0x00) |
| 77 | serverTelemetryData.serverId | String | "" |
| 78 | serverTelemetryData.scenarioId | String | "" |
| 79 | serverTelemetryData.worldId | String | "" |
| 80 | serverTelemetryData.ownerId | String | "" |

## GameRules (PocketMine envoie 2 règles)

Format par règle: String(name) + bool(isPlayerModifiable) + UnsignedVarInt(type) + value

| Règle | Modifiable | Type | Valeur |
|-------|------------|------|--------|
| naturalregeneration | false | Bool(1) | false |
| locatorbar | false | Bool(1) | false |

## Experiments

- u32 LE : count = 0
- bool : hasPreviouslyUsedExperiments = false

## NBT Compound vide (playerActorProperties)

Network Little-Endian NBT: `0x0A 0x00 0x00 0x00` (Compound, name="", end tag)
Attention: le format réseau utilise des VarInts pour les tags strings lengths.
