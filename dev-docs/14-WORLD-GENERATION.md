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
5. **Arbres — population CROSS-CHUNK** : on rejoue le placement déterministe des
   arbres du chunk **et de ses 8 voisins**, en n'écrivant que les blocs qui
   tombent dans ce chunk. Une canopée qui déborde d'un chunk à l'autre est ainsi
   posée à l'identique des deux côtés → **plus de coupures aux frontières**. Les
   surfaces des voisins sont calculées de façon déterministe (mêmes coins
   interpolés) et mises en cache global (`SURF_CACHE`), donc le surcoût est
   amorti (~79 ms/chunk, inchangé).
6. **Décoration** : lianes, herbe/fleurs, aquatique, etc. par biome.
7. **Sérialisation** : sub-chunks (paletté) + carte de biomes.

### Vue d'ensemble — couverture (✅ fait · 🟡 partiel/approx · ❌ manquant)

**Terrain & forme**

| Élément | État | Note |
|---|---|---|
| Relief 3D (density functions) | ✅ | y-64→320, montagnes/océans |
| Grottes (cheese/spaghetti/noodle) | ✅ | incluses dans `final_density` — ~70 % des colonnes en traversent une ; fines partiellement lissées par l'interpolation 4×8 (~-23 %) |
| **Aquifères** (eau/lave dans les grottes) | ✅ | port de vanilla `NoiseBasedAquifer` (barrières de pierre entre niveaux, aquifères perchés, deep dark sec) ; 99,6 % d'accord eau/air avec BDS (voir « Parité avec BDS ») |
| Carvers (grottes et canyons creusés après le bruit) | ❌ | absents : chez BDS, 3 % des blocs sous y = 80 sont de l'air ou de l'eau creusés par eux |
| Écoulement des fluides | ❌ | aucune simulation : l'eau posée par la génération ou par un joueur ne coule pas |
| Waterlogging (eau en seconde couche) | ✅ | plantes aquatiques générées dans l'eau, blocs posés dans une source, bloc cassé qui rend son eau ; liste des blocs de BDS (`block_liquid.rs`) |
| Disques (sable, argile, gravier au fond de l'eau ; herbe sur boue ; glace sur neige) | ✅ | `disks.rs`, lus dans les données vanilla ; aléa propre à mc-rs |
| Interpolation cellulaire 4×8 | ✅ | |
| `vertical_gradient` probabiliste | 🟡 | cutoff déterministe (bedrock/deepslate) |
| Parité numérique du bruit vs vanilla | ✅ | validée contre deepslate et BDS (voir « Parité avec BDS ») |

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
| Position de spawn | ✅ | premier sol au-dessus de y = 62 en spirale autour de (0, 0), lu sur le terrain chargé (`connection/spawn.rs`) ; le calcul legacy ne sert que de repli |

> En clair : **terrain + biomes + surfaces + minerais + toute la végétation/déco de
> surface** sont là et fidèles. Ce qui manque relève surtout des **systèmes macro**
> (carvers, structures, lacs/springs, features de grottes spécialisées) et des
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
  **Posés en passe CROSS-CHUNK** (voisinage 3×3) → canopées continues, plus de
  troncs/feuillages coupés aux frontières de chunk.
- **Lianes** sur les arbres de jungle/palétuvier.
- **Aquatique** : récifs de **corail** (5 couleurs) + **sea pickles** (océan
  chaud), **kelp** (colonnes clairsemées, océans seuls) + **seagrass**
  (océans/rivières).
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
- Parité du bruit et du climat : ✅ validée contre deepslate et BDS le 2026-10-26
  (voir « Parité avec BDS » ci-dessous).

**Biomes**
- Biomes 3D envoyés au client avec **une valeur par sous-chunk** (climat lu au
  centre de chaque tranche de 16 blocs) ; vanilla résout le climat tous les 4 blocs
  en hauteur.
- Frontières propres à Bedrock (3 % des colonnes sur la seed de référence), voir
  ci-dessous.

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
- ✅ **Population cross-chunk des arbres** (voisinage 3×3, surfaces voisines
  mises en cache) → canopées non coupées aux frontières. La validité d'emplacement
  ne lit plus la grille (déterminisme inter-chunks) : repose sur surface > mer +
  composition du biome.
- **Dripstone** : pointes courtes (1-2 blocs) faute de support des **états de
  bloc** (`dripstone_thickness` tip/frustum/base) ; le vrai effilage demande
  d'exposer les states dans le registre de blocs.
- Portage **100 % data-driven** du système `placed_feature`/`configured_feature`
  (l'objectif final : lire les `features` du biome au lieu de les coder en dur).

**Phase D — aquifères** : faite (2026-10-26), voir « Parité avec BDS ». Restent les
carvers et l'écoulement des fluides.

**Divers**
- Structures (villages, donjons, etc.).

### Parité avec BDS (placement des biomes)

Mesure du 2026-10-26, seed `4817747595384169343`, BDS 1.26.52.3. Fichiers :
`.reference/bds/comparisons/biomes-seed-4817747595384169343/` (captures, cartes
`{"cx,cz": [256 biomes]}` à y = 64).

| Carte | Colonnes identiques |
|---|---|
| deepslate 0.26.0 (Java), mêmes données que mc-rs, contre BDS (921 chunks) | 96,96 % |
| mc-rs avant correction, contre BDS (297 chunks) | 0 % |
| mc-rs corrigé, contre deepslate (716 chunks) | 100 % |
| mc-rs corrigé, contre BDS (536 chunks communs) | 99,41 % |

**Le placement des biomes de Bedrock est celui de Java** : mêmes données
(`data/worldgen`), même algorithme. Les données vanilla Java vendorées sont donc
les bonnes.

**Cause du 0 %** : `XoroshiroRandom::next_int` (`rng.rs`) prenait les 32 bits de
poids fort de `next_long`, alors que vanilla (`XoroshiroRandomSource.nextInt()` =
`(int) nextLong()`) et deepslate prennent ceux de poids faible. Les tables de
permutation de chaque `ImprovedNoise` étaient fausses, donc tous les bruits, le
climat, le relief et l'aquifère. Tests : `rng::next_int_takes_low_bits`,
`climate::climate_matches_deepslate_reference` (les six valeurs de climat en quatre
points). `Mth.lerp3` suit aussi l'ordre de vanilla depuis (même arrondi).

**Les 3 % restants** sont aux frontières de biomes, surtout le long des rivières :
BDS y trace des courbes au bloc près, et 5 % de ses cases 4 × 4 ne sont pas
uniformes, alors que Java avance par cases de 4 blocs. Hypothèses écartées, avec
deepslate contre BDS :

| Variante | Colonnes identiques |
|---|---|
| climat par case 4 × 4 (vanilla) | 96,96 % |
| climat au centre de la case | 96,89 % |
| climat exact à chaque bloc | 96,96 % |
| climat interpolé entre les coins des cases | 97,25 % |
| « fuzzy zoom » de `BiomeManager` (Java) | 96,61 % |
| quantification de `Climate.target` (× 10 000, entiers) | 96,98 % |
| autre altitude d'échantillonnage (y = 48 à 160) | ≤ 96,96 % |

Il reste à trouver le traitement de Bedrock dans BDS (désassemblage). Les écarts
se trouvent tous à moins de 14 blocs d'une frontière Java ; aucune translation
globale ne les explique.

**Outils** : `scripts/dev/chunk_biomes.py` (carte des biomes d'une capture
décodée, BDS ou mc-rs) et `scripts/dev/deepslate_biomes.mjs` (carte de deepslate
et comparaison ; procédure de compilation de deepslate dans son en-tête). Chaîne :
`bds-capture capture` (`-stay 120s` contre mc-rs, qui envoie ses chunks plus
lentement), `bds-capture decode`, `chunk_biomes.py --json`, puis
`deepslate_biomes.mjs --map`.

### Parité avec BDS (eau et aquifères)

Mesure du 2026-10-26, même seed, sur les 54 chunks que BDS a enregistrés autour du
spawn (`worlds/Bedrock level/db`, lu avec `amulet-leveldb`), de y = -64 à 79. On
compare des classes de blocs (`#` solide, `~` eau, `L` lave, `.` air).

| Carte | Accord eau/air dans les vides communs | Faces eau/air horizontales |
|---|---|---|
| BDS | — | 858 (eau qui a coulé après la génération) |
| mc-rs avant correction | 4 299 blocs d'eau communs sur 25 296 entre y = 16 et 79 | plusieurs milliers |
| deepslate 0.26.0 (aquifère désactivé : tout vide sous y = 63 est noyé) | 93,81 % | — |
| mc-rs corrigé | 99,59 % (aucune eau là où BDS a de l'air) | 9 |

**Causes**, toutes héritées de l'aquifère de deepslate, que deepslate garde lui-même
désactivé (`// WIP: Noise aquifers don't work yet`) :
- une barrière (pression trop forte entre deux aquifères) donnait de l'**air** au lieu
  de la **pierre** : parois d'eau verticales à nu, eau « coupée » ;
- surface préliminaire approchée par le premier bloc plein de `final_density` (qui
  contient les grottes) au lieu de `preliminary_surface_level` (`find_top_surface`,
  ajoutée à `density.rs` avec `invert`) : niveaux d'eau trop bas sous les grottes
  proches de la surface ;
- `computeFluid` rendait le statut d'une source voisine dans le mauvais cas, pressions
  combinées autrement que vanilla, bruits de barrière et d'inondation ré-échelonnés en y
  alors que leur `y_scale` est déjà dans les données, `spread` sans son facteur 10,
  région du deep dark non exclue ;
- aquifère consulté seulement sous le niveau de la mer (vanilla : partout).

`aquifer.rs` suit maintenant vanilla `Aquifer.NoiseBasedAquifer`. `Aquifer::dry_above`
saute le calcul au-dessus de la hauteur où aucun aquifère ne peut agir, sans changer le
résultat (test `dry_above_is_exact`, et sortie identique au bloc près sur les 54 chunks).

Le reste des écarts de classes (95,4 % d'accord au total) vient des carvers, absents
de mc-rs : air ou eau chez BDS là où mc-rs a de la pierre.

**Outils** : test ignoré `noise_chunk::tests::dump_chunk_classes` (commande dans sa
documentation) pour mc-rs, et `scripts/dev/world_block_classes.py extract` pour le monde
de BDS (sur une copie de la base), puis `world_block_classes.py compare`. Cartes de la
mesure : `bds-block-classes.json` et `mcrs-block-classes-aquifer-vanilla.json` dans
`.reference/bds/comparisons/biomes-seed-4817747595384169343/`.

### Parité avec BDS (fonds d'eau et waterlogging)

Zone de rivière et de plage (chunks x -8 à 1, z 9 à 18) que BDS a générée et enregistrée
(`scripts/dev/bds_load_area.py` : `tickingarea` puis `save hold`), 29 chunks lus dans sa
base, comparés à mc-rs (test ignoré `dump_chunk_blocks`) de y = 50 à 79.

| Bloc | BDS | mc-rs avant | mc-rs après |
|---|---|---|---|
| sable | 1 511 | 0 | 1 198 |
| argile | 29 | 0 | 93 |
| herbe marine (toute avec eau en seconde couche chez BDS) | 141 | 91, sans eau | 107, avec eau |

- **Waterlogging** : Bedrock range l'eau d'un bloc immergé dans une seconde couche du
  sous-chunk ; mc-rs n'en avait qu'une, d'où une bulle d'air autour de chaque plante
  aquatique. `SubChunk::liquid` (sérialisée, enregistrée, relue), liste des 19 840 états
  qui peuvent contenir une source d'eau (`canContainLiquidSource`, dump de BDS d'Allay,
  `scripts/dev/build_block_liquid.py`). L'air y figure selon BDS : `ChunkCache::set_block`
  vide la seconde couche quand on pose de l'air ou un bloc qui ne peut pas contenir d'eau,
  `break_block` rend l'eau d'un bloc cassé.
- **Disques** : le fond des rivières est de la terre selon les règles de surface (comme
  vanilla) ; le sable, l'argile et le gravier viennent des features `disk_*`, absentes de
  mc-rs jusqu'ici.
- Restent : écoulement, carvers (l'eau des grottes creusées par eux manque toujours),
  `spring_water`, `underwater_magma`, seau sur un bloc gorgé d'eau (à observer sur BDS).

### Validation

`cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` — 35 tests
worldgen verts (RNG, bruit, density, climat, surface, décoration, dont fancy oak
= plus de feuillage qu'un chêne, cerisier/palétuvier). Validation visuelle en jeu
via `generator = "noise"`.
