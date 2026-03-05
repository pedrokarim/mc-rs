# Format des Chunks (LevelChunkPacket) — Protocol 924

## LevelChunkPacket Structure

| Champ | Encodage |
|-------|----------|
| chunkX | SignedVarInt (zigzag) |
| chunkZ | SignedVarInt (zigzag) |
| dimensionId | SignedVarInt (zigzag) |
| subChunkCount | UnsignedVarInt |
| cacheEnabled | bool (false) |
| payload | UnsignedVarInt(len) + bytes |

## Payload Structure

```
[sub_chunk_0][sub_chunk_1]...[sub_chunk_N-1]
[biome_section_0][biome_section_1]...[biome_section_23]
[border_block_count = 0x00]
[tile_entity_nbt...]  (si applicable)
```

## Sub-Chunk Format (version 8)

```
[version: u8 = 8]
[num_layers: u8 = 1]  (on utilise 1 seule layer)
[storage_layer_0]
```

### Storage Layer (runtime palette, bit 0 = 1)

```
[header: u8]  = (bitsPerBlock << 1) | 1
[word_data: u32_le × word_count]
[palette_count: SignedVarInt (zigzag)]
[palette_entry_0: SignedVarInt (zigzag)]
[palette_entry_1: SignedVarInt (zigzag)]
...
```

- **bitsPerBlock** : MINIMUM 1 (jamais 0, même pour palette à 1 entrée)
- **word_count** : ceil(4096 / (32 / bitsPerBlock))
- **Palette entries** : runtime IDs des blocs (si blockNetworkIdsAreHashes=false, ce sont les IDs canoniques)

### Valeurs valides pour bitsPerBlock
1, 2, 3, 4, 5, 6, 8, 16

### Mapping palette_size → bitsPerBlock
| Palette size | bpb |
|-------------|-----|
| 1 | 1 (min) |
| 2 | 1 |
| 3-4 | 2 |
| 5-8 | 3 |
| 9-16 | 4 |
| 17-32 | 5 |
| 33-64 | 6 |
| 65-256 | 8 |
| 257+ | 16 |

## Biome Section Format

Même format que les storage layers mais avec 64 entrées (4×4×4) au lieu de 4096.

```
[header: u8] = (bitsPerEntry << 1) | 1
[word_data: u32_le × ceil(64 / (32/bpe))]
[palette_count: SignedVarInt (zigzag)]
[palette_entries: SignedVarInt (zigzag) × count]
```

- **bitsPerEntry** : MINIMUM 1
- **24 sections** pour Overworld (indices -4 à 19)
- Palette entries = biome legacy IDs (plains=1, ocean=0, etc.)

## Border Blocks
- 1 byte : count = 0 (toujours 0, feature non utilisée)

## Tile Entity NBT
- Données NBT concaténées (network little-endian) pour chaque block entity dans le chunk
- Pas de préfixe de longueur — juste les NBT tags les uns après les autres

## Notes Importantes

1. PocketMine utilise `blockNetworkIdsAreHashes = false` → les palette entries sont des runtime IDs canoniques, PAS des hash FNV-1a
2. Le nombre de biome sections est TOUJOURS le nombre total pour la dimension (24 pour Overworld), indépendant du subChunkCount
3. Les palette counts et entries utilisent TOUJOURS des SignedVarInt (zigzag) — même commenté "yes, this is intentionally zigzag" dans PocketMine
