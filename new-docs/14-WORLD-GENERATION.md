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

> ⚠️ La section ci-dessus décrit le générateur **legacy** (`generator = "normal"`),
> un port heightmap de PMMP. Un **nouveau générateur moderne** le remplace
> progressivement — voir ci-dessous.

---

## Générateur moderne « noise » — architecture Minecraft 1.18+ (Caves & Cliffs)

Réécriture complète de la génération sur le **système Java 1.18+** (density
functions), pour un rendu fidèle à Minecraft moderne / Bedrock plutôt que le
heightmap legacy. **Opt-in** via `generator = "noise"` dans `server.toml` (le
legacy `"normal"` reste le défaut le temps de la migration).

### Principe

Le terrain 1.18 n'est plus une heightmap mais un **champ de densité 3D** évalué
en chaque point : densité > 0 → solide, < 0 → air. Ce champ est un **arbre de
density functions** (bruit, splines, opérations) que l'on évalue. Les biomes
sont placés par un **climat multi-noise 6D**, les surfaces par des **surface
rules** data-driven, puis la **décoration** est posée par biome.

**Méthodo : zéro invention.** Les données (density functions, paramètres de
bruit, noise settings, param list de biomes, surface rules) sont la **donnée
worldgen vanilla Java verbatim**, vendorée dans `crates/mc-rs-server/data/worldgen/`
et embarquée via `include_dir`. Le code Rust n'est que l'évaluateur, porté
depuis la sémantique vanilla (réf. deepslate pour le bruit/les surfaces).

### Modules : `mc-rs-server/src/world/worldgen/`

| Fichier | Rôle |
|---|---|
| `rng.rs` | RNG vanilla : Xoroshiro128++ + seeding 128-bit + `fromHashOf` (md5) |
| `perlin.rs` | Bruit : `ImprovedNoise` → `PerlinNoise` → `NormalNoise` + chargeur params |
| `blended_noise.rs` | `old_blended_noise` (bruit 3D terrain legacy, surplombs) |
| `spline.rs` | Splines cubiques d'Hermite vanilla |
| `density.rs` | Interpréteur de density functions (~30 ops) + `NoiseRouter` |
| `data.rs` | Accès aux données vendorées (`include_dir`) |
| `climate.rs` | Moteur climat multi-noise 6D + param list overworld + mapping Bedrock |
| `noise_chunk.rs` | Échantillonnage cellulaire + interpolation + pipeline de chunk |
| `surface.rs` | Interpréteur de surface rules vanilla |
| `decoration.rs` | Décoration riche par biome (arbres, lianes, aquatique, …) |

### Pipeline d'un chunk (`noise_chunk::generate_noise_chunk`)

1. **Échantillonnage du terrain** : `final_density` évalué aux **coins des
   cellules** (4×4 horizontal, 8 vertical) puis **interpolation trilinéaire** par
   bloc → grille pleine hauteur (`-64..320`) remplie en stone / water / air.
2. **Biomes** : climat 6D échantillonné à la surface de chaque colonne → biome le
   plus proche → ID Bedrock.
3. **Surface rules** : grass/dirt/sable/grès/gravier/terracotta + bedrock/deepslate
   selon le biome.
4. **Minerais** : clusters insérés dans la roche souterraine.
5. **Décoration** : arbres, lianes, herbe/fleurs, aquatique, etc. par biome.
6. **Sérialisation** : sub-chunks (paletté) + carte de biomes.

### Vue d'ensemble — couverture (✅ fait · 🟡 partiel/approx · ❌ manquant)

**Terrain & forme**

| Élément | État | Note |
|---|---|---|
| Relief 3D (density functions) | ✅ | y-64→320, montagnes/océans |
| Grottes (cheese/spaghetti/noodle) | ✅ | incluses dans `final_density` — ~70 % des colonnes en traversent une ; fines partiellement lissées par l'interpolation 4×8 (~-23 %) |
| **Aquifères** (eau/lave dans les grottes) | ✅ | port `NoiseAquifer` — grottes inondées sous terres basses/océans, sèches sous terres hautes, lave en profondeur |
| Interpolation cellulaire 4×8 | ✅ | |
| `vertical_gradient` probabiliste | 🟡 | cutoff déterministe (bedrock/deepslate) |
| Parité numérique du bruit vs vanilla | 🟡 | non cross-validée |

**Biomes & surfaces**

| Élément | État | Note |
|---|---|---|
| Placement multi-noise 6D | ✅ | param list vanilla + mapping Geyser |
| Surface rules (grass/dirt/sable/gravier) | ✅ | interpréteur `surface_rule` |
| Neige + glace (biomes froids) | ✅ | `freeze_top_layer` |
| Pierre sur pentes raides (`steep`) | ✅ | |
| Terracotta badlands | 🟡 | uniforme (pas les bandes colorées) |
| `min_surface_level` | 🟡 | proxy (plus haut bloc solide) |
| Biomes **3D** (grottes : lush/dripstone/deep_dark) | ✅ | biomes échantillonnés par sub-chunk (depth élevé → grottes), sérialisés en 3D |

**Minerais**

| Élément | État | Note |
|---|---|---|
| Minerais en clusters (charbon→diamant, redstone, lapis) | ✅ | via `ore.rs` |
| Veines de minerai par bruit (`vein_toggle/ridged`) | ❌ | gros filons cuivre/fer 1.18 |

**Arbres** (compositions **data-fidèles** par biome)

| Élément | État | Note |
|---|---|---|
| Compositions officielles (sélecteurs `random_selector`) | ✅ | forêt = chêne+bouleau+fancy, birch=pur, etc. |
| **Fancy oak** (gros feuillage à branches) | ✅ | |
| Espèces : chêne, bouleau, super bouleau, sapin, pin, méga conifère 2×2, jungle, buisson, méga jungle 2×2, chêne noir 2×2, acacia, cerisier, palétuvier | ✅ | |
| Formes exactes (trunk/foliage placers vanilla) | 🟡 | approximations fidèles |

**Végétation au sol & déco** (densités **officielles**)

| Élément | État | Élément | État |
|---|---|---|---|
| Herbe (`noise_threshold_count`) | ✅ | Citrouilles / melons | ✅ |
| Fleurs (rarités par biome) | ✅ | Champignons brun/rouge | ✅ |
| Fougères (taïga) | ✅ | Glow lichen | ✅ |
| Lianes (127, jungle) | ✅ | Leaf litter (forêts) | ✅ |
| Bambou | ✅ | Buissons (`bush`) | ✅ |
| Canne à sucre | ✅ | Buissons de baies (taïga) | ✅ |
| Cactus / arbuste mort | ✅ | Nénuphars (marais) | ✅ |
| Fleurs hautes 2-blocs (lilas/rosier/pivoine/tournesol) | ❌ | Firefly bush | ❌ |
| Mushroom fields (mycélium + champignons géants) | ❌ | | |

**Aquatique**

| Élément | État | Note |
|---|---|---|
| Kelp / seagrass | ✅ | océans/rivières |
| Récifs de corail + sea pickles | ✅ | océan chaud |
| Icebergs (océan gelé) | ❌ | |

**Macro-features & structures** (gros manques restants)

| Élément | État | Note |
|---|---|---|
| Lacs de lave souterrains (`lake_lava`) | ✅ | 1/9 ; lacs d'eau retirés en 1.18 ; sources/springs ❌ |
| **Géodes d'améthyste** | ✅ | 1/24, coquille smooth_basalt/calcite/amethyst + budding + clusters |
| **Dripstone** (clusters + pointed) | ✅ | stalactites/stalagmites (192-256) + blocs (48-96) dans les grottes |
| **Lush caves** (mousse, azalée, lianes à baies, spore blossom, racines) | ✅ | placées dans le biome `lush_caves` |
| **Deep dark / sculk** (sculk + sensor/shrieker/catalyst/vein) | ✅ | placés dans le biome `deep_dark` |
| Disques sable/gravier/argile (bord de l'eau) | ❌ | |
| **Structures** | 🟡 | **donjons** (salle cobble + spawner + coffres) & **puits du désert** ✅ ; villages/mines/temples/strongholds/portails ❌ (jigsaw + templates NBT) |
| Position de spawn | 🟡 | calcul legacy (désync possible) |

> En clair : **terrain + biomes + surfaces + minerais + toute la végétation/déco de
> surface** sont là et fidèles. Ce qui manque relève surtout des **systèmes macro**
> (aquifères, structures, lacs/springs, features de grottes spécialisées) et des
> **biomes 3D** de grottes.

### ✅ Fait

**Terrain (phases A1→A4)**
- **A1** RNG vanilla (Xoroshiro128++, `fromHashOf`).
- **A2** Bruit Perlin/Normal + chargement des 60 paramètres de bruit vendorés.
- **A3** Interpréteur de density functions (~30 ops : add/mul/min/max, splines,
  `shifted_noise`, `weird_scaled_sampler`, `range_choice`, `y_clamped_gradient`,
  caches, `old_blended_noise`…) — parse l'arbre `final_density` overworld complet.
- **A4** Échantillonnage NoiseChunk (cellules 4×8) + interpolation trilinéaire +
  remplissage stone/eau/air. **Validé : relief vanilla y31→y208** (montagnes +
  océans), grottes présentes (déjà dans `final_density`).

**Biomes (phase B)** — placement multi-noise 6D
- Moteur climat 6D porté de deepslate (`ParamPoint`/`TargetPoint`, métrique de
  fittness, recherche du plus proche).
- **Param list overworld résolu** vendoré (sortie `OverworldBiomeBuilder`, 7593
  points / 54 biomes ; source mcmeta `1.21.4-data`).
- Mapping noms Java → **IDs Bedrock via Geyser** (autoritaire, validé vs
  `biomes.json` : 0 mismatch). Teintes herbe/eau/feuillage correctes par biome.

**Surfaces (phase C-full)** — interpréteur `surface_rule` vanilla complet
- Moteur par colonne (suivi `stoneDepth`/`waterHeight`) + arbre de règles évalué
  verbatim. Conditions : `above_preliminary_surface`, `biome`, `not`,
  `stone_depth` (+ `surface_secondary`), `vertical_gradient`, `water`, `y_above`,
  `noise_threshold`, `hole`, `temperature` (neige), `steep` (pentes).
- Résultats par biome : grass/dirt, **sable** (désert/plage), **gravier** (océans),
  **neige** (biomes froids), **terracotta** (badlands), mud (mangroves), etc.

**Minerais** — réutilise `ore.rs` : clusters (charbon/fer/or/diamant/redstone/
lapis + poches dirt/gravel) insérés dans stone/deepslate.

**Décoration (phase E)** — module `decoration.rs`, par biome :
- **Arbres data-fidèles** : chaque biome reçoit sa **composition vanilla
  officielle** (sélecteur `random_selector` = espèce par défaut + alternatives à
  chances exactes). Espèces : chêne, **fancy oak** (gros chêne touffu à branches
  + grappes de feuillage), bouleau / super bouleau, sapin, **pin**, **méga
  conifère 2×2**, jungle / **buisson** / **méga jungle 2×2**, chêne noir 2×2,
  acacia, **cerisier** (`cherry_*`), **palétuvier** (`mangrove_*` + racines).
  Sélection par la sémantique vanilla `random_selector` (test ordonné des chances).
- **Lianes** sur les arbres de jungle/palétuvier.
- **Aquatique** : récifs de **corail** (5 couleurs) + **sea pickles** (océan
  chaud), **kelp** (colonnes) + **seagrass** (océans/rivières).
- Herbe / fougères (taïga) / fleurs ; cactus & arbustes morts (désert),
  **bambou** (jungle), **canne à sucre** (bord de l'eau), **nénuphars** (marais).

#### Densités de décoration — **toutes officielles** (donnée vanilla)

Chaque biome (`worldgen/biome/<nom>.json`) a un champ `features` = 11 étapes de
génération ; l'étape **`vegetal_decoration`** liste *chaque* feature avec son
modificateur `count` / `rarity_filter`. Donc le taux d'**herbe, fleurs, lianes,
citrouilles, melons, champignons, bambou, canne, buissons…** est défini
exactement, par biome. Exemples extraits :

| Feature | Jungle | Plaines |
|---|---|---|
| Arbres (`trees_*` count) | 50 | 0.05 |
| Herbe (`patch_grass_*`) | 25 | bruit |
| **Lianes** (`vines` count) | **127** | — |
| Fleurs (`flower_*` rarity) | 1/16 | 1/32 |
| Bambou | 1/4 | — |
| Citrouille / Melon | 1/300 / 1/6 | 1/300 / — |
| Canne à sucre | 1/6 | 1/6 |
| Champignons brun/rouge | 1/256 / 1/512 | 1/256 / 1/512 |
| Glow lichen | 104–157 | 104–157 |

**Densités d'arbres** déjà appliquées aux valeurs officielles (moyenne du `count`
des `placed_feature` : plaines 0.05, jungle 50, mangrove 25…). L'herbe utilise
les `patch_grass_*` (jungle 25, forêt 2) ; plaines/savanes = `noise_threshold_count`
(approximé). Les autres densités (vines 127, citrouilles, champignons, bushes,
glow lichen…) restent à aligner exactement (voir « Reste à faire »).

**Données vendorées** (`data/worldgen/`) : 35 density functions, 60 params de
bruit, noise_settings overworld (avec `surface_rule`), param list de biomes
(`biome_parameters/overworld.json`), mapping `java_to_bedrock.json`.

### ⏳ Reste à faire (pour coller 100 % à Bedrock)

**Fidélité fine du terrain/surface**
- `vertical_gradient` en cutoff déterministe (pas la bande probabiliste vanilla,
  faute de RNG positionnel `at(x,y,z)`).
- `min_surface_level` approximé par le plus haut bloc solide (vs vrai
  `preliminary_surface_level`).
- `bandlands` : terracotta **uniforme** au lieu des bandes colorées par Y.
- Terme aléatoire ±0.25 de `surfaceDepth` omis.
- Parité numérique exacte du bruit vs vanilla **non cross-validée** (à confirmer
  contre deepslate).

**Biomes**
- Carte de biomes **2D** (répétée verticalement) → passer en **3D** (biomes de
  grottes : lush_caves, dripstone_caves, deep_dark).

**Décoration — alignée sur l'officiel (✅) + reste**
- ✅ Densités officielles appliquées : **lianes jungle 127** (y64-100),
  champignons 1/256·1/512, citrouilles 1/300, melons 1/6, **glow_lichen** 104-157,
  **leaf litter** (forêts), **bushes** 1/4, buissons de baies (taïga),
  **neige/glace** (freeze_top_layer), **fleurs** (plaines 1/32, jungle 1/16,
  flower_forest dense) et **herbe par bruit** (`noise_threshold_count`, 10/5).
- Reste : **mushroom fields** (mycélium + champignons géants), **firefly bush**,
  fleurs hautes 2-blocs (lilas/rosier/pivoine de flower_forest), nuances exactes
  des providers de fleurs (`noise_threshold_provider`).
- Formes d'arbres = approximations fidèles des trunk/foliage placers (pas le
  portage exact des placers vanilla).
- Portage **100 % data-driven** du système `placed_feature`/`configured_feature`
  (l'objectif final : lire les `features` du biome au lieu de les coder en dur).

**Phase D — aquifères**
- Aquifères par bruit (niveaux de fluide eau/lave dans les grottes). Les grottes
  elles-mêmes sont déjà présentes via `final_density`.

**Divers**
- Structures (villages, donjons, etc.).
- Spawn `find_spawn_position` utilise encore le calcul legacy (désync possible
  avec le terrain noise).

### Validation

`cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` — 35 tests
worldgen verts (RNG, bruit, density, climat, surface, décoration, dont fancy oak
= plus de feuillage qu'un chêne, cerisier/palétuvier). Validation visuelle en jeu
via `generator = "noise"`.
