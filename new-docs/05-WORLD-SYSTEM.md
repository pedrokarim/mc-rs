# 05 - World System

## PocketMine : Système de mondes

### Structure

```
WorldManager
  └─ World (1 par monde chargé)
       ├─ Chunks (HashMap<(x,z), Chunk>)
       │    └─ Chunk (16x16, 24 sub-chunks)
       │         ├─ SubChunk[24] (y=-64 à y=319)
       │         │    ├─ blockLayers: PalettedBlockArray[]
       │         │    ├─ biomes: PalettedBlockArray
       │         │    ├─ skyLight: LightArray?
       │         │    └─ blockLight: LightArray?
       │         ├─ HeightArray (16x16)
       │         └─ Entities, Tiles
       ├─ Entities (tous les entités du monde)
       ├─ ChunkLoaders (qui a besoin de quels chunks)
       └─ Generator (normal, flat, nether)
```

### WorldManager

Gère le chargement/déchargement des mondes.

**Responsabilités :**
- Charger/décharger les mondes
- Monde par défaut (spawn)
- Accès aux mondes par nom

### World

Le monde est le conteneur principal : chunks, entités, blocs, tick.

**Propriétés clés :**
- `name` : nom du monde
- `chunks` : HashMap des chunks chargés
- `entities` : liste des entités
- `time` : heure du monde (0-24000)
- `difficulty` : difficulté
- `spawnLocation` : point de spawn
- `gameRules` : règles de jeu
- `provider` : LevelDB pour la persistance

**Tick du monde :**
1. Incrémenter le temps
2. Tick les chunks actifs (random block tick)
3. Tick toutes les entités
4. Traiter les mises à jour de blocs planifiées
5. Traiter les mises à jour de lumière
6. Décharger les chunks non utilisés

### Chunk

16x16 horizontal, 384 blocs de haut (-64 à 319), 24 sub-chunks.

**Flags :**
- `DIRTY_FLAG_BLOCKS` : blocs modifiés
- `DIRTY_FLAG_BIOMES` : biomes modifiés
- `terrainPopulated` : génération terminée

### SubChunk

16x16x16 blocs.

**Stockage paletté (PalettedBlockArray) :**
- Palette : liste de block states uniques dans le sub-chunk
- Données : tableau de bits, chaque entrée est un index dans la palette
- Bits par bloc : dynamique (0, 1, 2, 3, 4, 5, 6, 8, 16)
  - 0 bits = un seul bloc (air typiquement), pas de données
  - Plus de variété = plus de bits par entrée

**Format réseau d'un sub-chunk :**
```
[u8 version]         → 8 ou 9
[u8 storage_count]   → nombre de layers (1 pour blocs, parfois 2 pour waterlogged)
[u8 y_offset]        → si version 9, offset Y du sub-chunk

Pour chaque layer :
  [u8 header]        → (bits_per_block << 1) | runtime_flag(1)

  Si bits_per_block == 0 :
    [VarInt32 single_palette_value]    → un seul bloc pour tout le sub-chunk

  Si bits_per_block > 0 :
    [u32_le words[]]  → données compactées (4096 entrées)
    [VarInt32 palette_size]
    [VarInt32 palette[palette_size]]   → runtime IDs
```

**Calcul du nombre de mots (u32) :**
```
blocks_per_word = floor(32 / bits_per_block)
word_count = ceil(4096 / blocks_per_word)
```

**Index dans le sub-chunk :**
```
index = (x << 8) | (z << 4) | y   (x, y, z de 0 à 15)
word_index = index / blocks_per_word
bit_offset = (index % blocks_per_word) * bits_per_block
palette_index = (words[word_index] >> bit_offset) & ((1 << bits_per_block) - 1)
```

### Biomes

Format identique aux blocs mais en résolution 4x4x4 (au lieu de 1x1x1) :
- 4x4x4 = 64 entrées par sub-chunk biome section
- Même format paletté avec header, palette, données

### LevelDB Storage

Format de stockage Bedrock, clés basées sur chunk position :

**Structure des clés :**
```
Key = chunk_x (i32_le) + chunk_z (i32_le) [+ dimension (i32_le) si pas overworld] + tag (u8) [+ sub_chunk_y (u8) si SUBCHUNK]
```

**Tags :**

| Tag | Nom | Description |
|---|---|---|
| 0x2b | HEIGHTMAP_AND_3D_BIOMES | Heightmap + biomes 3D |
| 0x2f | SUBCHUNK | Données de blocs d'un sub-chunk |
| 0x31 | BLOCK_ENTITIES | Entités de bloc (NBT) |
| 0x32 | ENTITIES | Entités (NBT) |
| 0x33 | PENDING_TICKS | Block ticks en attente |
| 0x36 | FINALIZATION | État de génération |
| 0x39 | BIOME_STATE | État des biomes |
| 0x3a | BORDER_BLOCKS | Blocs de bordure |
| 0x76 | VERSION | Version du chunk format |

**level.dat :**
- NBT avec les métadonnées du monde
- Contient : nom, seed, spawn, game rules, temps, etc.

### Fichiers PocketMine de référence

```
src/world/World.php
src/world/WorldManager.php
src/world/format/Chunk.php
src/world/format/SubChunk.php
src/world/format/HeightArray.php
src/world/format/io/leveldb/LevelDB.php
src/world/format/io/leveldb/ChunkDataKey.php
src/world/format/io/leveldb/ChunkVersion.php
src/world/format/io/leveldb/SubChunkVersion.php
src/world/format/io/data/BedrockWorldData.php
src/world/light/
```

---

## Implémentation Rust (état actuel)

### Module : `mc-rs-server/src/world/`

Le système de monde est intégré directement dans `mc-rs-server` (pas de crate séparé).

**Fichiers :**

| Fichier | Description |
|---|---|
| `terrain_generator.rs` | Génération de terrain (Simplex 3D, biomes, ores, végétation) |
| `flat_generator.rs` | Génération plate (4 couches fixes) |
| `chunk_serializer.rs` | Sérialisation sub-chunks en format réseau paletté |
| `chunk_cache.rs` | Cache mémoire avec LevelDB persistence |
| `storage.rs` | Interface LevelDB (rusty-leveldb) |
| `tick.rs` | Temps du monde (jour/nuit) + météo |
| `biome.rs` | 11 biomes, sélection par bruit, lissage Gaussien |
| `noise.rs` | Simplex noise 2D/3D (port PMMP) |
| `random.rs` | XorShift128 RNG (port PMMP) |
| `ore.rs` | Génération de minerais (veines courbes) |
| `vegetation.rs` | Arbres (chêne) et herbe courte |

**Architecture actuelle :**
- Pas de trait `Generator` — fonctions standalone dans `terrain_generator.rs`
- Chunks générés à la demande quand un joueur s'en approche
- `ChunkCache` garde les chunks en mémoire et les sauvegarde dans LevelDB
- `WorldState` gère le cycle jour/nuit et la météo (tick séparé)

**Génération de chunks :**
```rust
// Génère un chunk complet (terrain + biomes + ores + arbres)
pub fn generate_terrain_chunk(cx: i32, cz: i32, seed: u64) -> (u32, Vec<u8>);

// Hauteur de surface pour le spawn
pub fn get_surface_height(world_x: i32, world_z: i32, seed: u64) -> i32;
```

**Sérialisation sub-chunk :**
```rust
pub fn serialize_sub_chunk(blocks: &[u32; 4096], palette: &[u32]) -> Vec<u8>;
pub fn serialize_biome_section_single(biome_id: u32) -> Vec<u8>;
```

**Cache + Persistance :**
```rust
pub struct ChunkCache {
    chunks: HashMap<(i32, i32), ChunkColumn>,
    dirty: HashSet<(i32, i32)>,
    storage: Option<WorldStorage>,  // LevelDB
    seed: u64,
}
```

### Dépendances

| Crate | Usage |
|---|---|
| `rusty-leveldb` | LevelDB storage |
| `mc-rs-nbt` | NBT read/write (canonical_block_states.nbt) |
