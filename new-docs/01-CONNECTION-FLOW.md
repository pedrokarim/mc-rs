# Flux de Connexion Bedrock (Protocol 924 / 1.26.x)

Basé sur PocketMine-MP — serveur de référence fonctionnel.

## Phases

### Phase 1 — Session Start
```
Client → RequestNetworkSettings(protocolVersion=924)
Server → NetworkSettings(compressionThreshold=1, compressionAlgo=SNAPPY, clientThrottleEnabled=false, clientThrottleThreshold=0, clientThrottleScalar=0.0)
```
Après NetworkSettings : activer la compression sur la session.

### Phase 2 — Login
```
Client → LoginPacket(protocol=924, tokens=[chainData, skinData])
Server → PlayStatusPacket(LOGIN_SUCCESS = 0)
```

### Phase 3 — Resource Packs
```
Server → ResourcePacksInfoPacket(mustAccept=false, hasAddonPacks=false, hasScripts=false, worldTemplateId=UUID_NIL, worldTemplateVersion="", packs=[])
Client → ResourcePackClientResponsePacket(STATUS_HAVE_ALL_PACKS = 3)
Server → ResourcePackStackPacket(mustAccept=false, behaviorPacks=[], resourcePacks=[], gameVersion="*", experiments=[], experimentsPreviouslyUsed=false)
Client → ResourcePackClientResponsePacket(STATUS_COMPLETED = 4)
```

### Phase 4 — Pre-Spawn (paquets envoyés dans setUp)
Ordre exact (PocketMine) :
1. `StartGamePacket` — voir 02-STARTGAME.md
2. `ItemRegistryPacket` — liste complète des items
3. `AvailableActorIdentifiersPacket` — identifiants des entités (NBT)
4. `BiomeDefinitionListPacket` — définitions des biomes
5. `UpdateAttributesPacket` — attributs du joueur
6. `AvailableCommandsPacket` — commandes disponibles
7. `UpdateAbilitiesPacket` — capacités du joueur
8. `UpdateAdventureSettingsPacket` — paramètres aventure
9. `SetActorDataPacket` — metadata de l'entité joueur
10. `InventoryContentPacket` + `InventorySlotPacket` — inventaire
11. `CreativeContentPacket` — items créatifs
12. `CraftingDataPacket` — recettes
13. `PlayerListPacket` — liste des joueurs connectés

### Phase 5 — Chunk Loading
```
Client → RequestChunkRadiusPacket(radius=N)
Server → ChunkRadiusUpdatedPacket(radius=min(N, maxAllowed))
Server → NetworkChunkPublisherUpdatePacket(position, radius_blocks=viewDistance*16, savedChunks=[])
Server → LevelChunkPacket × N (chunks autour du spawn)
```
Envoyer ~50 chunks (rayon 4) avant de notifier terrain ready.

### Phase 6 — Spawn
```
Server → PlayStatusPacket(PLAYER_SPAWN = 3)
Client → SetLocalPlayerAsInitializedPacket(actorRuntimeId)
```
→ Transition vers InGame. Le joueur voit le monde.

## Notes Critiques
- `PlayStatus(PLAYER_SPAWN)` doit être envoyé APRÈS suffisamment de chunks
- Le client NE JAMAIS envoie `SetLocalPlayerAsInitialized` si les chunks ne sont pas corrects
- PocketMine utilise `blockNetworkIdsAreHashes = false` et une palette vide dans StartGame
