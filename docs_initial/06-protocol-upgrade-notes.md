# 06 - Notes de mise à jour protocole (766 → 924)

## BLOCK_STATE_VERSION — PIEGE CRITIQUE

### Le problème

Le fichier `canonical_block_states.nbt` de PMMP/BedrockData est encodé en **Network Little-Endian NBT** :
- Les TAG_Int sont encodés en **ZigZag VarInt** (pas en raw i32 LE)
- Les longueurs de strings sont en **VarUInt32** (pas en i16 LE)

Si on lit le champ `version` comme un i32 LE brut, on obtient une valeur **complètement fausse**.

### Exemple concret

Les octets bruts dans le fichier pour le champ version sont : `c2 f0 a9 11`

**Lecture INCORRECTE** (raw i32 LE) :
```
0x11a9f0c2 = 296_349_890  ← FAUX ! (décode en "17.169.240.194")
```

**Lecture CORRECTE** (ZigZag VarInt → i32) :
```
VarInt decode: c2 f0 a9 11 → 36_337_730
ZigZag decode: (36337730 >> 1) ^ -(36337730 & 1) = 18_168_865
→ 0x01153C21 = version 1.21.60.33  ← CORRECT
```

### Source de vérité : PMMP

```php
// BedrockProtocol — WorldDataVersions.php
public const BLOCK_STATES =
    (1 << 24) |   // major  = 1
    (21 << 16) |  // minor  = 21
    (60 << 8) |   // patch  = 60
    (33);          // rev    = 33
// = 18_168_865
```

### Valeurs par version

| Protocole | Game Version | BLOCK_STATE_VERSION | Hex |
|-----------|-------------|--------------------|----|
| 766 | 1.21.50 | 18_100_737 | 0x01143201 |
| 924 | 1.26.0 | 18_168_865 | 0x01153C21 |

### Impact

Quand `block_network_ids_are_hashes = true` dans StartGame, le client et le serveur calculent indépendamment un hash FNV-1a de chaque block state. Le NBT inclut le champ `version`. Si la valeur est fausse, **TOUS** les hash sont faux et le client ne reconnaît aucun bloc → bloqué sur "Création du monde".

---

## Format NBT pour le hash FNV-1a des block states

### Format utilisé : Network Little-Endian NBT

Le hash FNV-1a est calculé sur les octets du block state sérialisé en **Network LE NBT** :
- TAG_Compound (0x0A) avec nom vide
- "name" → TAG_String (0x08) : identifiant du bloc (ex: "minecraft:stone")
- "states" → TAG_Compound (0x0A) : propriétés triées alphabétiquement
- "version" → TAG_Int (0x03) : BLOCK_STATE_VERSION en ZigZag VarInt

**Encodage des types :**
- TAG_String : VarUInt32(longueur) + octets UTF-8
- TAG_Int : ZigZag VarInt (PAS raw i32 LE)
- TAG_Byte : 1 octet signé
- TAG_End : 0x00

### Ordre des clés (déterministe, crucial pour le hash)

```
Root Compound ("")
├── "name": String      (identifiant du bloc)
├── "states": Compound  (propriétés triées par nom)
│   ├── prop_a: ...
│   ├── prop_b: ...
│   └── TAG_End
├── "version": Int      (BLOCK_STATE_VERSION)
└── TAG_End
```

---

## Sérialisation des chunks — palette VarInt

### Palette entries : signed VarInt32 (PAS ZigZag)

Les entrées de palette dans les sub-chunks utilisent un **signed VarInt32 sans ZigZag** :
```rust
// CORRECT — cast i32 → u32 (bit-preserving) puis écrire comme unsigned VarInt
fn write_signed_varint32(buf: &mut BytesMut, value: i32) {
    write_varuint32(buf, value as u32);
}
```

**PAS** ZigZag :
```rust
// FAUX — ne pas utiliser pour les palettes !
fn write_zigzag_varint(buf: &mut BytesMut, value: i32) {
    let encoded = ((value << 1) ^ (value >> 31)) as u32;
    write_varuint32(buf, encoded);
}
```

Avec ZigZag, la valeur 1 s'encode en 2, la valeur 42 en 84, etc. Toutes les IDs de blocs seraient corrompues.

### Quand utiliser quoi

| Contexte | Encodage |
|----------|----------|
| Block state NBT (pour hash FNV-1a) | Network LE NBT : TAG_Int = **ZigZag VarInt** |
| Palette entries dans sub-chunks | **Signed VarInt32 (NO zigzag)** |
| Biome palette entries | **Signed VarInt32 (NO zigzag)** |
| Palette size | **Signed VarInt32 (NO zigzag)** |

---

## Format Sub-chunk (version 9)

```
u8(9)              // version
u8(1)              // num_layers (pas de waterlogging dans Bedrock)
u8(y_index)        // i8 → u8 : sub_chunk_index - 4
[BlockStorage]     // couche de stockage

BlockStorage:
  u8(header)       // (bits_per_block << 1) | 1 (bit 0 = runtime flag)
  [u32_le words]   // données de blocs (si bpb > 0)
  VarInt32(palette_size)  // signed, no zigzag
  [VarInt32 entries]      // signed, no zigzag — runtime IDs (hash FNV-1a)
```

### Biome sections (×24)

```
Section single-biome:
  u8(0x00)         // header (0 bits)
  VarInt32(id)     // biome ID, signed no zigzag

Section multi-biome:
  u8(bpe << 1)     // header (PAS de runtime flag, bit 0 = 0)
  [u32_le words]   // 64 entries (4×4×4)
  VarInt32(palette_size)
  [VarInt32 entries]  // biome IDs
```

---

## Payload LevelChunk

```
[SubChunk × 24]    // 24 sub-chunks (Y=-64 à +319)
[BiomeSection × 24] // 24 sections biome
[0x00]             // border blocks (Education Edition, toujours vide)
```

---

## Références locales utiles

| Projet | Langage | Chemin | Usage |
|--------|---------|--------|-------|
| BedrockData (PMMP) | JSON/NBT | `.reference/BedrockData/` | Données canoniques (items, biomes, entities) |
| BedrockProtocol (PMMP) | PHP | `.reference/BedrockProtocol/` | Packets, types, encodage |
| gophertunnel | Go | `.reference/gophertunnel/` | Protocole complet, chunks, auth |
| dragonfly | Go | `.reference/dragonfly/` | Serveur complet (utilise gophertunnel) |
| CloudburstProtocol | Java | `.reference/CloudburstProtocol/` | Protocole Java, versioning |
| bedrock-rs | Rust | `.reference/bedrock-rs/` | Protocole Rust (incomplet) |
| bedrock-protocol-docs | Markdown | `.reference/bedrock-protocol-docs/` | Docs OFFICIELLES Mojang |
| prismarine-bedrock | JS | `.reference/prismarine-bedrock/` | Protocole JS avec auth |
| rak-rs | Rust | `.reference/rak-rs/` | Implémentation RakNet Rust |
| BDS | Binaire | `.reference/bds/` | Serveur officiel Mojang |
