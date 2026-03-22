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

## Implémentation Rust (état actuel)

### Module : `mc-rs-server/src/world/`

L'implémentation est un port direct de l'algorithme Normal de PocketMine-MP,
sans dépendances externes pour le bruit ou le RNG.

**Fichiers :**

| Fichier | Description |
|---|---|
| `random.rs` | XorShift128 RNG (port exact de PMMP `Random`) |
| `noise.rs` | Simplex 2D/3D + multi-octave + `getFastNoise3D` trilinéaire |
| `biome.rs` | 11 biomes + BiomeSelector (temp/rainfall) + Gaussian smoothing |
| `terrain_generator.rs` | Génération terrain principale + block IDs |
| `ore.rs` | Minerais : veines courbes (8 types) |
| `vegetation.rs` | Arbres (chêne) + herbe courte par biome |
| `chunk_serializer.rs` | Sérialisation sub-chunks (format réseau paletté) |
| `flat_generator.rs` | Générateur plat (non utilisé par défaut) |

**Architecture (fonctions, pas de trait) :**
```rust
// Point d'entrée principal
pub fn generate_terrain_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>);
pub fn get_surface_height(world_x: i32, world_z: i32, seed: u64) -> i32;
```

**Pipeline de génération (dans `generate_terrain_chunk`) :**
1. Initialiser RNG avec `0xdeadbeef ^ (cx << 8) ^ cz ^ seed`
2. Créer Simplex noise (4 octaves, persistence 1/4, expansion 1/32)
3. Générer biomes + Gaussian smooth des élévations
4. Générer bruit 3D via `getFastNoise3D` (sampling 4/8/4)
5. Pré-calculer hauteurs de surface par colonne
6. Générer positions des minerais (veines courbes)
7. Générer végétation (arbres, herbe)
8. Pour chaque sub-chunk, placer les blocs :
   - Y=0 : bedrock
   - Y<0 : stone
   - Zone noise : `noiseValue - 1/smoothHeight * (y - smoothHeight - minSum)`
     - `> 0` → stone (+ ground cover + ores)
     - `≤ 0 && y ≤ 62` → water
     - sinon → air (+ végétation)
9. Sérialiser sub-chunks + biomes

**Biomes implémentés :**

| ID | Biome | Élévation | Cover |
|---|---|---|---|
| 0 | Ocean | 46-58 | Gravel |
| 1 | Plains | 63-68 | Grass + Dirt |
| 2 | Desert | 63-74 | Sand + Sandstone |
| 3 | Extreme Hills | 63-127 | Grass + Dirt |
| 4 | Forest | 63-81 | Grass + Dirt |
| 5 | Taiga | 63-81 | Snow + Grass + Dirt |
| 6 | Swampland | 62-63 | Grass + Dirt |
| 7 | River | 58-62 | Dirt |
| 12 | Ice Plains | 63-74 | Snow + Grass + Dirt |
| 20 | Small Mountains | 63-97 | Grass + Dirt |
| 27 | Birch Forest | 63-81 | Grass + Dirt |

**Minerais (paramètres PMMP) :**

| Minerai | Clusters/chunk | Taille | Y min | Y max |
|---|---|---|---|---|
| Coal | 20 | 16 | 0 | 128 |
| Iron | 20 | 8 | 0 | 64 |
| Redstone | 8 | 7 | 0 | 16 |
| Lapis | 1 | 6 | 0 | 32 |
| Gold | 2 | 8 | 0 | 32 |
| Diamond | 1 | 7 | 0 | 16 |
| Dirt (poches) | 20 | 32 | 0 | 128 |
| Gravel (poches) | 10 | 16 | 0 | 128 |

**Block IDs :** Indices séquentiels de `canonical_block_states.nbt` (copié dans `data/`).
Fichier de référence parsé via `mc-rs-nbt` pour extraire les IDs exacts.
