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

## Équivalent Rust

### Crate : `mc-rs-world`

```rust
pub struct WorldManager {
    worlds: HashMap<String, World>,
    default_world: String,
}

pub struct World {
    pub name: String,
    pub seed: i64,
    pub spawn: BlockPos,
    pub time: i64,
    pub difficulty: Difficulty,
    pub game_rules: GameRules,
    chunks: HashMap<ChunkPos, Chunk>,
    entities: Vec<EntityId>,
    provider: Box<dyn WorldProvider>,
    generator: Box<dyn Generator>,
    // Tick
    scheduled_updates: BTreeMap<u64, Vec<BlockUpdate>>,
}

pub struct Chunk {
    pub x: i32,
    pub z: i32,
    sub_chunks: [Option<SubChunk>; 24],  // y=-4 à y=19
    height_map: HeightArray,
    dirty_flags: u8,
    entities: Vec<EntityId>,
    tiles: HashMap<BlockPos, Box<dyn BlockEntity>>,
}

pub struct SubChunk {
    block_layers: Vec<PalettedStorage>,  // généralement 1, parfois 2
    biomes: PalettedStorage,             // résolution 4x4x4
    sky_light: Option<LightArray>,
    block_light: Option<LightArray>,
}

pub struct PalettedStorage {
    palette: Vec<u32>,         // runtime block state IDs
    bits_per_block: u8,
    data: Vec<u32>,            // mots compactés
}

impl PalettedStorage {
    pub fn new_single(value: u32) -> Self {
        Self { palette: vec![value], bits_per_block: 0, data: vec![] }
    }

    pub fn get(&self, x: u8, y: u8, z: u8) -> u32 {
        if self.bits_per_block == 0 {
            return self.palette[0];
        }
        let index = ((x as usize) << 8) | ((z as usize) << 4) | (y as usize);
        let blocks_per_word = 32 / self.bits_per_block as usize;
        let word_index = index / blocks_per_word;
        let bit_offset = (index % blocks_per_word) * self.bits_per_block as usize;
        let mask = (1u32 << self.bits_per_block) - 1;
        let palette_index = (self.data[word_index] >> bit_offset) & mask;
        self.palette[palette_index as usize]
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, value: u32) {
        // Resize palette if needed, update bits_per_block, etc.
        todo!()
    }
}

pub type HeightArray = [[i16; 16]; 16];
pub type LightArray = [u8; 2048]; // 4 bits par bloc, 4096 blocs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}
```

### LevelDB Provider

```rust
pub trait WorldProvider: Send {
    fn load_chunk(&self, x: i32, z: i32) -> Result<Option<Chunk>>;
    fn save_chunk(&self, chunk: &Chunk) -> Result<()>;
    fn load_world_data(&self) -> Result<WorldData>;
    fn save_world_data(&self, data: &WorldData) -> Result<()>;
}

pub struct LevelDbProvider {
    db: Database,  // rusty-leveldb ou leveldb crate
    path: PathBuf,
}

impl LevelDbProvider {
    fn make_key(x: i32, z: i32, tag: u8, sub_y: Option<u8>) -> Vec<u8> {
        let mut key = Vec::with_capacity(10);
        key.extend_from_slice(&x.to_le_bytes());
        key.extend_from_slice(&z.to_le_bytes());
        key.push(tag);
        if let Some(y) = sub_y {
            key.push(y);
        }
        key
    }
}
```

### Dépendances

| Crate | Usage |
|---|---|
| `rusty-leveldb` ou `leveldb` | LevelDB storage |
| `mc-rs-nbt` | NBT read/write |
