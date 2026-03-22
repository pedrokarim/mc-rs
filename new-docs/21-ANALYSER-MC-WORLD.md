# 21 - Analyser un monde Minecraft Bedrock Edition

## Objectif

Ce document explique comment ouvrir, lire et analyser un monde Minecraft Bedrock Edition
exporté (.mcworld ou dossier de monde) pour en extraire les données de terrain, biomes,
et structures. Utile pour calibrer notre générateur de terrain.

---

## 1. Structure d'un monde Bedrock

Un monde Bedrock est un dossier contenant :

```
world/
├── db/                          ← LevelDB database (chunks, entities, etc.)
│   ├── 000003.ldb               ← Data files
│   ├── 000004.ldb
│   ├── CURRENT
│   ├── LOCK
│   └── MANIFEST-000002
├── level.dat                    ← Métadonnées du monde (NBT, little-endian)
├── level.dat_old                ← Backup
├── levelname.txt                ← Nom du monde
└── world_icon.jpeg              ← Icône
```

Un fichier `.mcworld` est simplement une **archive ZIP** renommée. Pour l'extraire :
```bash
mv monde.mcworld monde.zip
unzip monde.zip -d monde/
```

---

## 2. LevelDB — La base de données des chunks

Bedrock stocke TOUT dans LevelDB (pas de fichiers .mca comme Java Edition).

### Clés LevelDB

Chaque entrée a une clé binaire structurée :

```
[chunk_x: i32_le][chunk_z: i32_le][tag: u8][sub_chunk_y: u8 (optionnel)]
```

Pour le Nether/End, un dimension ID est inséré :
```
[chunk_x: i32_le][chunk_z: i32_le][dimension: i32_le][tag: u8][sub_chunk_y: u8]
```

### Tags importants

| Tag (hex) | Nom | Description |
|---|---|---|
| `0x2C` | CHUNK_VERSION_LEGACY | Version du chunk (ancien) |
| `0x2D` | DATA_2D_LEGACY | Heightmap + biomes 2D (ancien) |
| `0x2F` | SUBCHUNK | Données de blocs d'un sub-chunk (16x16x16) |
| `0x31` | BLOCK_ENTITIES | Tile entities (coffres, panneaux, etc.) |
| `0x32` | ENTITIES | Entités (mobs, items, etc.) |
| `0x33` | PENDING_TICKS | Block ticks en attente |
| `0x36` | FINALIZATION | État de génération du chunk |
| `0x39` | BIOME_STATE | Données de biomes 3D |
| `0x3A` | BORDER_BLOCKS | Blocs de bordure |
| `0x76` | VERSION | Version du format chunk |
| `0x2B` | DATA_3D | Heightmap + biomes 3D (nouveau format) |

### Construire une clé pour un chunk

```rust
fn make_chunk_key(cx: i32, cz: i32, tag: u8) -> Vec<u8> {
    let mut key = Vec::with_capacity(9);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    key.push(tag);
    key
}

// Pour un sub-chunk spécifique
fn make_subchunk_key(cx: i32, cz: i32, sub_y: u8) -> Vec<u8> {
    let mut key = Vec::with_capacity(10);
    key.extend_from_slice(&cx.to_le_bytes());
    key.extend_from_slice(&cz.to_le_bytes());
    key.push(0x2F); // SUBCHUNK tag
    key.push(sub_y);
    key
}
```

---

## 3. Format des Sub-Chunks (tag 0x2F)

Chaque sub-chunk stocke 16x16x16 blocs en format paletté :

```
[u8 version]          → 8 ou 9
[u8 storage_count]    → nombre de layers (1 = blocs, 2 = waterlogged)
[u8 y_offset]         → si version 9, offset Y du sub-chunk

Pour chaque layer :
  [u8 header]         → (bits_per_block << 1) | persistence_flag

  Si bits == 0 (un seul bloc) :
    [i32_le block_state_id]   ← EN DISK: little-endian i32, PAS VarInt !

  Si bits > 0 :
    [u32_le words[]]          ← données compactées
    [i32_le palette_size]     ← nombre d'entrées
    [NBT palette[]]           ← NBT compounds pour chaque block state
```

**ATTENTION — Différences Disk vs Network :**

| Aspect | Disk (LevelDB) | Network (packets) |
|---|---|---|
| Palette format | **NBT compound** (nom + states) | **VarInt runtime ID** |
| Int encoding | **i32 little-endian** | **VarInt ZigZag** |
| Persistence flag | bit 0 = 0 (persistent) | bit 0 = 1 (runtime) |

### Palette NBT (format disk)

Chaque entrée de palette est un NBT Compound :
```nbt
{
  "name": "minecraft:stone",    ← nom du bloc
  "states": {                   ← propriétés du block state
    "stone_type": "stone"
  },
  "version": 18100736           ← version du block state
}
```

### Décoder les blocs

```
index = (x << 8) | (z << 4) | y     (x, y, z de 0 à 15)
word_index = index / blocks_per_word
bit_offset = (index % blocks_per_word) * bits_per_block
palette_index = (words[word_index] >> bit_offset) & ((1 << bits_per_block) - 1)
block = palette[palette_index]
```

Bits per block possibles : 1, 2, 3, 4, 5, 6, 8, 16

---

## 4. Biomes (tag 0x2B ou 0x2D)

### Format 3D (tag 0x2B — DATA_3D)

Biomes sont stockés en résolution 4x4x4 (64 entrées par section de 16 blocs) :
```
[u16_le heightmap[256]]       ← 16x16 heightmap
[biome_sections[24]]          ← 24 sections (Y=-64 à Y=319)

Chaque biome section :
  [u8 header]                 ← même format paletté que les blocs
  Si bits == 0 : [i32_le biome_id]    ← un seul biome
  Si bits > 0 : [words + palette]     ← palette de biome IDs
```

### Format 2D (tag 0x2D — legacy)
```
[u16_le heightmap[256]]       ← 16x16 heightmap
[u8 biome_ids[256]]           ← 16x16 biome ID par colonne
```

---

## 5. level.dat — Métadonnées du monde

Format : 8 octets de header + NBT little-endian standard.

```
[u32_le version]     ← format version (ex: 10)
[u32_le data_length] ← taille des données NBT
[NBT LE data]        ← compound tag avec les métadonnées
```

### Champs importants dans level.dat

| Champ | Type | Description |
|---|---|---|
| `LevelName` | String | Nom du monde |
| `RandomSeed` | Long | Seed du monde |
| `SpawnX/Y/Z` | Int | Position de spawn |
| `GameType` | Int | 0=survie, 1=créatif |
| `Difficulty` | Int | 0-3 |
| `currentTick` | Long | Tick actuel |
| `Time` | Long | Heure du monde |
| `Generator` | Int | 1=flat, 2=normal |
| `WorldVersion` | Int | Version du monde |
| `StorageVersion` | Int | Version du format |

---

## 6. Comment analyser un monde en Rust

### Dépendances nécessaires
```toml
rusty-leveldb = "3"
mc-rs-nbt = { path = "../mc-rs-nbt" }
```

### Ouvrir la base LevelDB
```rust
use rusty_leveldb::{DB, Options};

let opts = Options::default();
let mut db = DB::open("monde/db", opts).expect("Failed to open LevelDB");
```

### Lire level.dat
```rust
use mc_rs_nbt::read_nbt_le;

let data = std::fs::read("monde/level.dat").unwrap();
// Skip 8-byte header (version + length)
let mut buf = &data[8..];
let root = read_nbt_le(&mut buf).unwrap();
let seed = root.compound.get("RandomSeed"); // Long
let spawn_x = root.compound.get("SpawnX");  // Int
```

### Lire un sub-chunk
```rust
fn read_subchunk(db: &mut DB, cx: i32, cz: i32, sub_y: u8) -> Option<Vec<u8>> {
    let key = make_subchunk_key(cx, cz, sub_y);
    db.get(&key)
}
```

### Extraire les blocs d'un sub-chunk
```rust
fn parse_subchunk(data: &[u8]) -> Vec<String> {
    let version = data[0]; // 8 ou 9
    let storage_count = data[1];
    let mut pos = if version == 9 { 3 } else { 2 }; // skip y_offset for v9

    let mut blocks = Vec::new();

    for _ in 0..storage_count {
        let header = data[pos]; pos += 1;
        let bits = header >> 1;
        let _persistent = (header & 1) == 0; // disk = 0, network = 1

        if bits == 0 {
            // Single block — read NBT
            // Parse NBT compound from data[pos..]
            // ... (use mc_rs_nbt::read_nbt_le)
        } else {
            // Multi-block paletted storage
            let blocks_per_word = 32 / bits as usize;
            let word_count = 4096usize.div_ceil(blocks_per_word);

            // Skip word array
            pos += word_count * 4;

            // Read palette size (i32 LE)
            let palette_size = i32::from_le_bytes(data[pos..pos+4].try_into().unwrap());
            pos += 4;

            // Read palette entries (NBT compounds)
            for _ in 0..palette_size {
                // Parse NBT compound from data[pos..]
                // Each entry has "name" and "states"
            }
        }
    }

    blocks
}
```

### Analyser la heightmap et les biomes
```rust
fn read_heightmap_biomes(db: &mut DB, cx: i32, cz: i32) -> Option<([u16; 256], Vec<u8>)> {
    let key = make_chunk_key(cx, cz, 0x2B); // DATA_3D
    let data = db.get(&key)?;

    // First 512 bytes = heightmap (256 x u16_le)
    let mut heightmap = [0u16; 256];
    for i in 0..256 {
        heightmap[i] = u16::from_le_bytes([data[i*2], data[i*2+1]]);
    }

    // Rest = biome sections (paletted, 24 sections)
    let biome_data = data[512..].to_vec();

    Some((heightmap, biome_data))
}
```

---

## 7. Analyses possibles

### 7.1 Distribution des hauteurs
Pour calibrer notre générateur :
1. Lire la heightmap de chaque chunk (tag 0x2B ou 0x2D)
2. Construire un histogramme des hauteurs
3. Comparer avec notre générateur

### 7.2 Distribution des biomes
1. Lire les biome IDs de chaque chunk
2. Compter la fréquence de chaque biome
3. Vérifier qu'on a la même distribution

### 7.3 Composition du terrain
1. Lire les sub-chunks et compter les blocs
2. Ratio stone/dirt/sand/gravel par biome
3. Profondeur moyenne du ground cover

### 7.4 Profil de terrain
1. Pour une ligne de chunks (ex: Z=0, X=-100 à 100)
2. Extraire la hauteur à chaque X
3. Tracer le profil pour voir la forme du terrain

---

## 8. Limitations

- Les sub-chunks non générés n'existent pas dans LevelDB
- Le format de palette (NBT compounds) est plus complexe que les runtime IDs réseau
- Les biomes 3D (post-1.18) utilisent 24 sections au lieu d'un tableau 2D
- `rusty-leveldb` ne supporte pas la compression zstd (certains mondes récents)

---

## 9. Fichiers de référence dans le projet

- Notre LevelDB storage : `crates/mc-rs-server/src/world/storage.rs`
- Notre chunk cache : `crates/mc-rs-server/src/world/chunk_cache.rs`
- Notre NBT parser : `crates/mc-rs-nbt/src/`
- Données BDS biomes : `.reference/bds/server/behavior_packs/vanilla/biomes/`
- canonical_block_states.nbt : `crates/mc-rs-server/data/canonical_block_states.nbt`
