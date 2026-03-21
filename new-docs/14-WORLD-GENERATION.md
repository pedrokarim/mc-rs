# 14 - World Generation

## PocketMine : Génération de mondes

### Architecture

```
GeneratorManager
├── register("flat", Flat)
├── register("normal", Normal)
└── register("nether", Nether)

Generator (abstract)
├── generateChunk(cx, cz)     → créer le terrain de base
├── populateChunk(cx, cz)     → ajouter la décoration
└── getSpawnLocation()         → point de spawn

Population se fait en async (PopulationTask) :
  1. Générer le chunk + 8 chunks voisins
  2. Peupler le chunk central
  3. Renvoyer au main thread
```

### Flat Generator

Le plus simple : terrain plat configurable.

```
Options : "layers" : [
  { block: "bedrock", height: 1 },
  { block: "dirt", height: 2 },
  { block: "grass_block", height: 1 },
]

Résultat : chaque chunk a exactement les mêmes couches.
```

**Implémentation :**
```php
function generateChunk(cx, cz) {
    for y in each layer :
        fill entire 16x16 slice with layer block
}
```

### Normal Generator (Overworld)

Terrain procédural avec biomes.

**Étapes :**
1. **Noise** : Perlin/Simplex noise pour la heightmap
2. **Biome selection** : Basé sur température + humidité
3. **Terrain shape** : Hauteur de base + caves
4. **Surface** : Appliquer le bloc de surface du biome (grass, sand, etc.)
5. **Population** : Arbres, minerais, herbe, fleurs, structures

**Populators :**

| Populator | Description |
|---|---|
| Tree | Arbres (Oak, Birch, Spruce, Jungle, Acacia) |
| Ore | Veines de minerai (Coal, Iron, Gold, Diamond, etc.) |
| TallGrass | Herbe haute |
| GroundCover | Couverture de surface |

**Ore Populator :**
```php
OreType(block, clusterSize, clusterCount, minY, maxY)

Ores par défaut :
  Coal:     size=17, count=20, y=0-128
  Iron:     size=9,  count=20, y=-64-72
  Gold:     size=9,  count=2,  y=-64-30
  Diamond:  size=8,  count=1,  y=-64-16
  Redstone: size=8,  count=8,  y=-64-16
  Lapis:    size=7,  count=1,  y=-64-30
```

**Tree Populator :**
```
Pour chaque chunk :
  Nombre d'arbres basé sur le biome
  Position aléatoire dans le chunk
  Type d'arbre basé sur le biome
  Vérifier l'espace disponible
  Placer tronc + feuilles
```

### Nether Generator

Terrain caverneux avec lave à Y=31.

### Noise System

**Simplex Noise :**
- Plus rapide que Perlin pour 2D/3D
- Utilisé pour la heightmap et la sélection de biomes
- Paramètres : octaves, frequency, amplitude, persistence

```php
function getNoise2D(x, z, octaves, frequency, amplitude) {
    total = 0
    for i in 0..octaves :
        total += simplex2D(x * frequency, z * frequency) * amplitude
        frequency *= 2
        amplitude *= persistence
    return total
}
```

### Population Task (async)

```
1. Main thread demande la génération d'un chunk
2. PopulationTask envoyée à l'AsyncPool
3. Worker thread :
   a. Charger/générer chunk + 8 voisins
   b. Appliquer les populators sur le chunk central
   c. Sérialiser le résultat
4. Main thread récupère et applique le chunk
```

### Fichiers PocketMine de référence

```
src/world/generator/Generator.php
src/world/generator/GeneratorManager.php
src/world/generator/Flat.php
src/world/generator/FlatGeneratorOptions.php
src/world/generator/normal/Normal.php
src/world/generator/hell/Nether.php
src/world/generator/noise/Noise.php
src/world/generator/noise/Simplex.php
src/world/generator/object/Tree.php
src/world/generator/object/Ore.php
src/world/generator/object/TallGrass.php
src/world/generator/populator/Populator.php
src/world/generator/PopulationTask.php
src/world/generator/LightPopulationTask.php
```

---

## Équivalent Rust

### Crate : `mc-rs-world` (module `generator`)

```rust
use noise::{NoiseFn, Perlin, Fbm};

/// Trait pour les générateurs de monde
pub trait Generator: Send + Sync {
    fn generate_chunk(&self, cx: i32, cz: i32) -> Chunk;
    fn populate_chunk(&self, cx: i32, cz: i32, chunks: &mut ChunkAccess);
    fn spawn_location(&self) -> BlockPos;
}

/// Générateur plat
pub struct FlatGenerator {
    layers: Vec<FlatLayer>,
    biome: u32,
}

pub struct FlatLayer {
    pub block: BlockState,
    pub height: u32,
}

impl Generator for FlatGenerator {
    fn generate_chunk(&self, cx: i32, cz: i32) -> Chunk {
        let mut chunk = Chunk::new(cx, cz);
        let mut y = -64i32;
        for layer in &self.layers {
            for dy in 0..layer.height {
                let block_y = y + dy as i32;
                for x in 0..16 {
                    for z in 0..16 {
                        chunk.set_block(x, block_y, z, layer.block);
                    }
                }
            }
            y += layer.height as i32;
        }
        chunk
    }

    fn populate_chunk(&self, _cx: i32, _cz: i32, _chunks: &mut ChunkAccess) {
        // Pas de population pour flat
    }

    fn spawn_location(&self) -> BlockPos {
        let height: i32 = self.layers.iter().map(|l| l.height as i32).sum();
        BlockPos { x: 0, y: -64 + height, z: 0 }
    }
}

/// Générateur normal (overworld)
pub struct NormalGenerator {
    seed: i64,
    noise: Fbm<Perlin>,
    biome_noise: Fbm<Perlin>,
}

impl Generator for NormalGenerator {
    fn generate_chunk(&self, cx: i32, cz: i32) -> Chunk {
        let mut chunk = Chunk::new(cx, cz);

        for x in 0u8..16 {
            for z in 0u8..16 {
                let world_x = (cx * 16 + x as i32) as f64;
                let world_z = (cz * 16 + z as i32) as f64;

                // Heightmap via noise
                let height = self.get_height(world_x, world_z);

                // Bedrock
                chunk.set_block(x, -64, z, BlockState::BEDROCK);

                // Stone
                for y in -63..height - 3 {
                    chunk.set_block(x, y, z, BlockState::STONE);
                }

                // Dirt + surface
                let biome = self.get_biome(world_x, world_z);
                let (surface, sub_surface) = biome.surface_blocks();
                for y in (height - 3)..height {
                    chunk.set_block(x, y, z, sub_surface);
                }
                chunk.set_block(x, height, z, surface);

                // Water fill
                if height < 62 {
                    for y in (height + 1)..=62 {
                        chunk.set_block(x, y, z, BlockState::WATER);
                    }
                }
            }
        }

        chunk
    }

    fn populate_chunk(&self, cx: i32, cz: i32, chunks: &mut ChunkAccess) {
        let mut rng = ChunkRng::new(self.seed, cx, cz);

        // Ores
        self.populate_ores(&mut rng, cx, cz, chunks);

        // Trees
        self.populate_trees(&mut rng, cx, cz, chunks);

        // Tall grass
        self.populate_grass(&mut rng, cx, cz, chunks);
    }

    fn spawn_location(&self) -> BlockPos {
        // Trouver un bon spawn sur la surface
        let height = self.get_height(0.0, 0.0);
        BlockPos { x: 0, y: height + 1, z: 0 }
    }
}

/// Populator de minerai
pub struct OrePopulator {
    ores: Vec<OreConfig>,
}

pub struct OreConfig {
    pub block: BlockState,
    pub cluster_size: u32,
    pub count_per_chunk: u32,
    pub min_y: i32,
    pub max_y: i32,
}

impl OrePopulator {
    pub fn default_ores() -> Self {
        Self {
            ores: vec![
                OreConfig { block: BlockState::COAL_ORE, cluster_size: 17, count_per_chunk: 20, min_y: 0, max_y: 128 },
                OreConfig { block: BlockState::IRON_ORE, cluster_size: 9, count_per_chunk: 20, min_y: -64, max_y: 72 },
                OreConfig { block: BlockState::GOLD_ORE, cluster_size: 9, count_per_chunk: 2, min_y: -64, max_y: 30 },
                OreConfig { block: BlockState::DIAMOND_ORE, cluster_size: 8, count_per_chunk: 1, min_y: -64, max_y: 16 },
                OreConfig { block: BlockState::REDSTONE_ORE, cluster_size: 8, count_per_chunk: 8, min_y: -64, max_y: 16 },
                OreConfig { block: BlockState::LAPIS_ORE, cluster_size: 7, count_per_chunk: 1, min_y: -64, max_y: 30 },
            ],
        }
    }
}

/// Dépendances recommandées
// noise = "0.9"  → Perlin, Simplex, Fbm
// rand = "0.8"   → RNG seedé par chunk
```
