# 05 - Format du monde, chunks et stockage

## Vue d'ensemble

Minecraft Bedrock utilise **LevelDB** (Google) pour stocker les données de monde, contrairement à Java Edition qui utilise le format Anvil/Region.

Structure sur disque :
```
world/
├── db/                  # Base de données LevelDB
│   ├── CURRENT
│   ├── LOCK
│   ├── MANIFEST-*
│   ├── *.ldb            # Fichiers SSTable
│   └── *.log            # Write-ahead logs
├── level.dat            # Métadonnées du monde (NBT + header)
├── level.dat_old        # Backup
├── levelname.txt        # Nom du monde (texte brut)
├── world_icon.jpeg      # Icône du monde
├── world_behavior_packs.json
├── world_resource_packs.json
└── behavior_packs/      # Behavior packs appliqués
    └── resource_packs/  # Resource packs appliqués
```

## LevelDB — Format des clés

### Structure des clés

Les clés LevelDB pour les données de chunk suivent le format :

```
[X: int32_le][Z: int32_le][dimension_tag?][data_tag: byte][sub_chunk_y?: byte]
```

- `X`, `Z` : Coordonnées du chunk en little-endian signé (32 bits chacun)
- `dimension_tag` : Absent pour l'Overworld, `01 00 00 00` pour le Nether, `02 00 00 00` pour l'End
- `data_tag` : Type de données (1 octet)
- `sub_chunk_y` : Index du sub-chunk (uniquement pour le tag `0x2F`)

### Tags de données

| Tag (hex) | Tag (dec) | Nom | Contenu |
|-----------|-----------|-----|---------|
| `0x2C` | 44 | ChunkVersion | 1 octet : version du format de chunk |
| `0x2D` | 45 | Data2D | Heightmap (256 × int16_le) + Biome IDs (256 octets) |
| `0x2B` | 43 | Data3D | Données biomes 3D (palette, depuis 1.18+) |
| `0x2F` | 47 | SubChunkPrefix | Données d'un sub-chunk 16×16×16 |
| `0x31` | 49 | BlockEntity | NBT concaténés des block entities du chunk |
| `0x32` | 50 | Entity (legacy) | NBT concaténés des entités du chunk |
| `0x33` | 51 | PendingTicks | Ticks planifiés |
| `0x34` | 52 | LegacyBlockExtraData | Données legacy (ancien format) |
| `0x36` | 54 | FinalizedState | int32_le (0=needs populating, 2=done) |
| `0x39` | 57 | BorderBlocks | Blocs de bordure (éducation) |
| `0x3A` | 58 | HardcodedSpawnAreas | Zones de spawn |
| `0x3B` | 59 | Checksums | Checksums du chunk |
| `0x76` | 118 | ChunkVersion (ancien) | Même que 0x2C (ancien emplacement) |

### Exemple de clé

Pour le sub-chunk Y=3 du chunk (10, -5) dans le Nether :

```
0A 00 00 00    # X = 10 (int32_le)
FB FF FF FF    # Z = -5 (int32_le, complément à 2)
01 00 00 00    # dimension = Nether
2F             # tag = SubChunkPrefix
03             # sub_chunk_y = 3
```

Pour l'Overworld, pas de dimension_tag :
```
0A 00 00 00    # X = 10
FB FF FF FF    # Z = -5
2F             # tag = SubChunkPrefix
03             # sub_chunk_y = 3
```

### Clés spéciales (non-chunk)

| Clé | Contenu |
|-----|---------|
| `~local_player` | NBT des données du joueur local (singleplayer) |
| `player_<uuid>` | NBT des données d'un joueur serveur |
| `actorprefix<unique_id>` | Données d'entité par ID unique (moderne) |
| `digp<chunk_key>` | Liste d'IDs d'entités dans un chunk (moderne) |
| `map_<id>` | Données de cartes |
| `portals` | Données des portails du Nether |
| `scoreboard` | Données du scoreboard |
| `AutonomousEntities` | Entités autonomes |
| `BiomeData` | Données biomes globales |
| `Overworld` / `Nether` / `TheEnd` | Marqueurs de dimension |

## Sub-Chunk Format

### Version 9 (format actuel)

Un sub-chunk est un cube de **16×16×16 blocs** :

```
[version: byte]          # 9 pour le format actuel
[num_layers: byte]       # Nombre de couches (1 ou 2)
[y_index: byte]          # Position Y du sub-chunk (signé)

Pour chaque couche :
  [palette_header: byte]
    Bits 1-7 : bits_per_block (1, 2, 3, 4, 5, 6, 8, ou 16)
    Bit 0    : persistence_type (0 = persistence/disk, 1 = runtime/network)

  [block_data: uint32_le[]]  # Tableau packed d'indices dans la palette
    Nombre de mots = ceil(4096 / (32 / bits_per_block))

  [palette_size: int32_le]   # Nombre d'entrées dans la palette

  [palette_entries: NBT[]]   # Tags NBT Compound pour chaque entrée
```

### Stockage des blocs

Les 4096 blocs (16×16×16) sont stockés dans l'ordre **XZY** :
```
index = (x * 16 + z) * 16 + y
```
Où `x`, `y`, `z` vont de 0 à 15.

Les indices sont packés dans des mots de 32 bits :
- Avec 4 bits par bloc : 8 blocs par mot → `ceil(4096 / 8) = 512` mots
- Avec 1 bit par bloc : 32 blocs par mot → `ceil(4096 / 32) = 128` mots
- Avec 16 bits par bloc : 2 blocs par mot → `ceil(4096 / 2) = 2048` mots

### Valeurs de bits-per-block

| Bits | Blocs/mot | Mots nécessaires | Blocs uniques max |
|------|-----------|-------------------|-------------------|
| 1 | 32 | 128 | 2 |
| 2 | 16 | 256 | 4 |
| 3 | 10 | 410 | 8 |
| 4 | 8 | 512 | 16 |
| 5 | 6 | 683 | 32 |
| 6 | 5 | 820 | 64 |
| 8 | 4 | 1024 | 256 |
| 16 | 2 | 2048 | 65536 |

### Palette Entries (sur disque)

Chaque entrée de palette est un tag NBT Compound :

```nbt
{
    "name": "minecraft:stone",
    "states": {
        "stone_type": "stone"
    },
    "version": 18100737
}
```

Autre exemple :
```nbt
{
    "name": "minecraft:oak_stairs",
    "states": {
        "weirdo_direction": 0,
        "upside_down_bit": 1
    },
    "version": 18100737
}
```

### Palette Entries (réseau / runtime)

Sur le réseau (dans `LevelChunk` packets), les entrées de palette sont des **VarInt** représentant les runtime block state IDs, au lieu de NBT.

### Couches multiples (Waterlogging)

Bedrock utilise 2 couches par sub-chunk :
- **Couche 0** : Blocs principaux
- **Couche 1** : Blocs d'eau (waterlogging)

Si un bloc est waterlogged, la couche 1 contient `minecraft:water` à cette position.

## Biomes

### Data2D (ancien format, tag 0x2D)

```
Heightmap : int16_le[256]  # 16×16, hauteur de chaque colonne
Biomes    : byte[256]      # 16×16, ID biome par colonne (2D)
```

### Data3D (nouveau format, tag 0x2B, depuis 1.18+)

Biomes 3D encodés par sections de 4×4×4 :

```
Pour chaque section verticale de 4 blocs :
  [palette_header: byte]  # bits_per_block | (type << 1)
  [biome_data: packed]    # 64 entrées (4×4×4) packées
  [palette_size: int32_le]
  [palette_entries: String[]]  # Noms des biomes
```

## level.dat

### Format du fichier

```
[storage_version: int32_le]  # Version de stockage (ex: 10)
[data_length: int32_le]      # Longueur des données NBT
[NBT data: ...]              # Tag Compound little-endian
```

### Champs principaux du NBT

```nbt
{
    "GameType": 0,                    // 0=Survival, 1=Creative, 2=Adventure
    "Difficulty": 2,                  // 0=Peaceful, 1=Easy, 2=Normal, 3=Hard
    "ForceGameType": 0,              // Forcer le gamemode
    "SpawnX": 0,
    "SpawnY": 64,
    "SpawnZ": 0,
    "LevelName": "My World",
    "RandomSeed": 123456789,          // Seed du monde (int64)
    "Time": 6000,                     // Tick du monde
    "currentTick": 12345,             // Tick actuel
    "LastPlayed": 1700000000,         // Timestamp UNIX
    "Generator": 1,                   // 0=legacy, 1=default, 2=flat
    "FlatWorldLayers": "...",         // JSON pour monde plat
    "StorageVersion": 10,
    "NetworkVersion": 766,            // Version protocole
    "inventoryVersion": "1.21.50",
    "commandblockoutput": 1,
    "commandsEnabled": 1,
    "dodaylightcycle": 1,
    "dofiretick": 1,
    "domobloot": 1,
    "domobspawning": 1,
    "dotiledrops": 1,
    "doweathercycle": 1,
    "drowningdamage": 1,
    "falldamage": 1,
    "firedamage": 1,
    "keepinventory": 0,
    "mobgriefing": 1,
    "naturalregeneration": 1,
    "pvp": 1,
    "sendcommandfeedback": 1,
    "showcoordinates": 0,
    "showdeathmessages": 1,
    "spawnMobs": 1,
    "tntexplodes": 1,
    "worldStartCount": 1,
    "hasBeenLoadedInCreative": 0,
    "hasLockedBehaviorPack": 0,
    "hasLockedResourcePack": 0,
    "isFromLockedTemplate": 0,
    "isFromWorldTemplate": 0,
    "isSingleUseWorld": 0,
    "useMsaGamertagsOnly": 0,
    "bonusChestEnabled": 0,
    "bonusChestSpawned": 0,
    "startWithMapEnabled": 0,
    "serverChunkTickRange": 4,
    "rainLevel": 0.0,
    "lightningLevel": 0.0,
    "rainTime": 0,
    "lightningTime": 0,
    "eduOffer": 0,
    "isEduEnabled": 0,
    "immutableWorld": 0,
    "texturePacksRequired": 0,
    "prid": ""
}
```

## Génération de monde

### Pipeline de génération (Overworld)

```
1. Noise de biomes
   └─ Déterminer le biome pour chaque colonne (4×4 résolution)
       Utilise : température, humidité, continentalness, erosion, weirdness

2. Noise de densité (terrain)
   └─ Perlin noise multi-octave pour la forme du terrain
       Inputs : biome parameters, seed
       Output : density field 3D → solid/air à chaque position

3. Surface Builder
   └─ Appliquer la surface selon le biome
       Plaines : herbe + terre + pierre
       Désert : sable + grès + pierre
       Océan : gravier/sable + eau
       etc.

4. Carvers / Caves
   └─ Creuser les grottes et ravins
       Noise Perlin 3D pour les grottes spaghetti
       Bruit Cheese/Noodle pour les grottes modernes

5. Features / Ores
   └─ Placer les minerais, arbres, fleurs, etc.
       Distribution par hauteur et biome
       Arbres selon le biome (chêne, bouleau, sapin...)

6. Structures
   └─ Placer les structures générées
       Villages, temples, donjons, bastions...
       Vérifié par chunk avec un algorithme de placement
```

### Distribution des minerais (Overworld)

| Minerai | Y min | Y max | Tentatives/chunk | Taille veine |
|---------|-------|-------|------------------|--------------|
| Charbon | 0 | 320 | 30 | 17 |
| Fer | -64 | 72 | 20 | 9 |
| Or | -64 | 32 | 4 | 9 |
| Diamant | -64 | 16 | 1-4 | 4-8 |
| Lapis | -32 | 32 | 2 | 7 |
| Redstone | -64 | 15 | 8 | 8 |
| Émeraude | -16 | 320 | 1 | 1 |
| Cuivre | -16 | 112 | 16 | 10 |

### Génération du Nether

- **Plafond et sol de bedrock** à Y=0 et Y=127
- **Lac de lave** à Y=31
- **Biomes** : Wastes, Soul Sand Valley, Crimson Forest, Warped Forest, Basalt Deltas
- **Structures** : Forteresses du Nether, Bastions, Ruined Portals
- Terrain généré par noise 3D avec cavités

### Génération de l'End

- **Île principale** autour de (0, 0) — petite île flottante
- **Îles extérieures** à partir de ~1000 blocs du centre
- **End Cities** sur les îles extérieures
- **Chorus Plants** sur les îles
- **Dragon Egg** et portail de retour au centre

### Structures et algorithme de placement

Chaque type de structure a un algorithme de placement basé sur :
- **Espacement** : Distance minimale entre deux instances
- **Séparation** : Distance maximale entre deux instances
- **Salt** : Valeur unique ajoutée au seed pour chaque type

```
Pour vérifier si un chunk contient le début d'une structure :
  hash = seed + salt
  region_x = chunk_x / spacing
  region_z = chunk_z / spacing
  start_x = region_x * spacing + random(separation, hash)
  start_z = region_z * spacing + random(separation, hash)
  Si (start_x, start_z) == (chunk_x, chunk_z) → Structure ici
```

## Block States

### Système de block states

Bedrock utilise un système de **block states** basé sur des chaînes :

```
Identifiant : "minecraft:oak_stairs"
Propriétés  : {
    "weirdo_direction": 0,    // 0-3 (direction)
    "upside_down_bit": false  // true/false
}
```

Chaque combinaison unique (identifiant + propriétés) = **un block state unique** avec son propre runtime ID.

### Runtime Block IDs

- Assignés au démarrage du serveur pour chaque version de protocole
- Envoyés dans le `StartGame` packet comme palette
- Depuis 1.19.80+ avec `BlockNetworkIDsAreHashes = true` :
  ```
  runtime_id = fnv1_32(block_state_nbt)
  ```
- Le dépôt `pmmp/BedrockData` contient les palettes canoniques

### Registre de blocs

Le serveur doit maintenir un registre de tous les block states connus :

```rust
pub struct BlockRegistry {
    // Nom → définition du bloc
    blocks: HashMap<String, BlockDefinition>,
    // Runtime ID → block state
    runtime_to_state: Vec<BlockState>,
    // Block state → Runtime ID
    state_to_runtime: HashMap<BlockState, u32>,
}

pub struct BlockState {
    pub name: String,              // "minecraft:stone"
    pub properties: BTreeMap<String, PropertyValue>,
}

pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    String(String),
}
```

## Entités dans LevelDB

### Ancien format (tag 0x32)

Les entités étaient stockées par chunk :
- Clé : `[chunk_x][chunk_z][dimension?][0x32]`
- Valeur : NBT Compound tags concaténés (une par entité)

### Nouveau format (actorprefix / digp)

- `digp<chunk_key>` : Liste d'entité Unique IDs (int64_le) dans le chunk
- `actorprefix<unique_id>` : Données NBT de l'entité

Avantage : Les entités peuvent être accédées individuellement sans charger tout le chunk.

## Block Entities (Tile Entities)

Stockées avec le tag `0x31` :
- Clé : `[chunk_x][chunk_z][dimension?][0x31]`
- Valeur : NBT Compound tags concaténés

Exemples de block entities :
- Coffres (inventaire)
- Fours (recettes en cours)
- Panneaux (texte)
- Spawners (config de mob)
- Bannières (patterns)
- Enchantement tables
- Beacon

Chaque block entity contient au minimum :
```nbt
{
    "id": "Chest",       // Type
    "x": 100,            // Position
    "y": 64,
    "z": -50,
    "isMovable": 1       // Peut être déplacé par piston
}
```

## Implémentation Rust

### Provider LevelDB

```rust
use rusty_leveldb::DB;

pub struct LevelDbProvider {
    db: DB,
}

impl LevelDbProvider {
    pub fn open(path: &Path) -> Result<Self> {
        let options = Options {
            // Bedrock utilise Snappy compression dans LevelDB
            compressor: Some(Box::new(SnappyCompressor)),
            ..Default::default()
        };
        let db = DB::open(path, options)?;
        Ok(Self { db })
    }

    pub fn load_sub_chunk(&self, x: i32, z: i32, y: i8, dim: Dimension) -> Option<SubChunk> {
        let key = Self::make_sub_chunk_key(x, z, y, dim);
        let data = self.db.get(&key)?;
        SubChunk::deserialize(&data)
    }

    fn make_sub_chunk_key(x: i32, z: i32, y: i8, dim: Dimension) -> Vec<u8> {
        let mut key = Vec::new();
        key.extend_from_slice(&x.to_le_bytes());
        key.extend_from_slice(&z.to_le_bytes());
        if dim != Dimension::Overworld {
            key.extend_from_slice(&(dim as i32).to_le_bytes());
        }
        key.push(0x2F); // SubChunkPrefix tag
        key.push(y as u8);
        key
    }
}
```

### Chunk en mémoire

```rust
pub struct ChunkColumn {
    pub x: i32,
    pub z: i32,
    pub sub_chunks: Vec<Option<SubChunk>>,  // -4 à +19 (24 sub-chunks pour -64 à 319)
    pub biomes: BiomeData,
    pub block_entities: Vec<NbtCompound>,
    pub heightmap: [i16; 256],
    pub finalized_state: u32,
    pub dirty: bool,
}

pub struct SubChunk {
    pub layers: Vec<BlockLayer>,
}

pub struct BlockLayer {
    pub palette: Vec<BlockState>,
    pub blocks: PackedArray,  // Indices dans la palette, bits_per_block variable
}

pub struct PackedArray {
    pub bits_per_block: u8,
    pub data: Vec<u32>,
}
```
