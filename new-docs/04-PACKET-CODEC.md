# Codec de Paquets — Protocol 924

## Couches d'encapsulation

```
RakNet Frame
  └─ Game Packet (après compression)
       └─ Batch: [packet_1][packet_2]...
            └─ Chaque packet: UnsignedVarInt(len) + [header + body]
                 └─ header: UnsignedVarInt(packetId << 2)
```

## Compression

- Après `NetworkSettingsPacket`, tous les game packets sont compressés
- Algorithme : Snappy (id=1) ou Zlib/Deflate (id=0)
- PocketMine utilise Snappy par défaut
- Format : `[compression_id: u8][compressed_data]` (pas pour les tous premiers paquets)

Note: Le tout premier paquet (`RequestNetworkSettings`) n'est PAS compressé.
Après NetworkSettings, la compression est activée.

## Types d'encodage

### Primitifs
| Type | Description |
|------|-------------|
| u8 / bool | 1 byte |
| u16 LE | 2 bytes little-endian |
| i16 LE | 2 bytes little-endian signé |
| i32 LE | 4 bytes little-endian signé |
| u32 LE | 4 bytes little-endian |
| i64 LE | 8 bytes little-endian signé |
| u64 LE | 8 bytes little-endian |
| f32 LE | 4 bytes IEEE 754 little-endian |

### VarInts
| Type | Description |
|------|-------------|
| UnsignedVarInt (u32) | Variable-length, 7 bits par byte, MSB = continuation |
| SignedVarInt (i32) | Zigzag: (n << 1) ^ (n >> 31), puis encodé comme UnsignedVarInt |
| UnsignedVarLong (u64) | Comme UnsignedVarInt mais 64 bits |
| SignedVarLong (i64) | Zigzag 64: (n << 1) ^ (n >> 63), puis UnsignedVarLong |

### Composés
| Type | Description |
|------|-------------|
| String | UnsignedVarInt(len) + UTF-8 bytes |
| Vec3 | f32 LE × 3 (x, y, z) |
| BlockPos | SignedVarInt(x) + UnsignedVarInt(y) + SignedVarInt(z) |
| UUID | 16 bytes: first 8 LE + last 8 LE (pas standard!) |
| Optional<T> | bool(present) + T si present |

## Packet IDs Importants

| ID | Nom |
|----|-----|
| 0x01 | Login |
| 0x02 | PlayStatus |
| 0x03 | ServerToClientHandshake |
| 0x04 | ClientToServerHandshake |
| 0x05 | Disconnect |
| 0x06 | ResourcePacksInfo |
| 0x07 | ResourcePackStack |
| 0x08 | ResourcePackClientResponse |
| 0x0b | StartGame |
| 0x0c | AddPlayer |
| 0x27 | SetActorData |
| 0x28 | SetActorMotion |
| 0x2b | SetTime |
| 0x31 | UpdateAttributes |
| 0x3a | ItemRegistry (anciennement CraftingData id varies) |
| 0x3e | LevelChunk |
| 0x3f | SetCommandsEnabled |
| 0x40 | SetDifficulty |
| 0x45 | RequestChunkRadius |
| 0x46 | ChunkRadiusUpdated |
| 0x4c | AvailableCommands |
| 0x77 | AvailableActorIdentifiers |
| 0x79 | NetworkChunkPublisherUpdate |
| 0x80 | BiomeDefinitionList |
| 0xbb | PlayStatus (aussi 0x02) |
| 0xc1 | UpdateAbilities |
| 0xc2 | UpdateAdventureSettings |
| 0x3f | PlayerList |
| 0x91 | CreativeContent |
| 0x34 | CraftingData |
