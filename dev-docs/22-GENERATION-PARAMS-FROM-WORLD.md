# 22 - Paramètres de Génération extraits d'un monde Bedrock réel

Résultats de l'analyse de "Mon monde" (seed: `-1320394628052872703`, protocol 924, 1.26.3).
264 chunks analysés, 2332 sub-chunks, 9.5M blocs.

---

## 1. Couches souterraines

| Zone | Y min | Y max | Bloc principal | Composition |
|---|---|---|---|---|
| **Bedrock** | -64 | -60 | `bedrock` | 100% à Y=-64/-63, mélange graduel : 75% à Y=-62, 50% à Y=-61, 25% à Y=-60 |
| **Deepslate** | -60 | +4 | `deepslate` 65-88% | + tuff 5-9%, gravel ~1%, grottes (air 10-25%) |
| **Transition** | 0 | +7 | mélange | Y=0: 80% deepslate / Y=5: croisement 30%/37% / Y=8: 0% deepslate |
| **Stone** | +5 | surface | `stone` 57-65% | + granite/diorite/andesite ~6-7% chacun |

### Détail de la transition deepslate → stone

```
Y=0 : deepslate 79.8%, stone  0.0%
Y=1 : deepslate 70.2%, stone  7.7%
Y=2 : deepslate 60.9%, stone 15.0%
Y=3 : deepslate 50.3%, stone 22.6%
Y=4 : deepslate 40.1%, stone 30.0%
Y=5 : deepslate 29.9%, stone 37.5%  ← croisement
Y=6 : deepslate 19.9%, stone 45.0%
Y=7 : deepslate 10.2%, stone 52.4%
Y=8 : deepslate  0.0%, stone 59.4%
```

La transition est **linéaire sur 8 niveaux** (Y=0 à Y=8), ~10% de shift par niveau.

---

## 2. Bedrock layer (détail)

```
Y=-64 : bedrock 100%
Y=-63 : bedrock 100%
Y=-62 : bedrock  75%, deepslate 22%, tuff 2%
Y=-61 : bedrock  50%, deepslate 44%, tuff 4%
Y=-60 : bedrock  25%, deepslate 66%, tuff 6%
Y=-59 : bedrock   0%, deepslate 88%
```

Règle : bedrock garanti à Y=-64/-63, puis dégradé probabiliste (~25% par niveau) de Y=-62 à Y=-60.

---

## 3. Minerais intrinsèques (stone variants)

Dans la zone stone (Y=8 à surface), les proportions sont stables :

| Bloc | Proportion dans la zone stone |
|---|---|
| `stone` | 57-65% |
| `granite` | 5.7-8.0% |
| `diorite` | 5.6-7.5% |
| `andesite` | 5.9-8.0% |
| `coal_ore` | ~0.6% (en surface) |
| `copper_ore` | ~0.4% |
| `iron_ore` | ~0.2% |
| `lapis_ore` | trace |
| `gold_ore` | trace |
| `redstone_ore` | trace |

---

## 4. Surface et niveau de la mer

### Niveau de la mer
- **Y=62** : dernier niveau avec de l'eau significative
- L'eau apparaît à partir de Y=36 (~6%), augmente jusqu'à Y=62 (~30%)
- À Y=63 : l'air remplace l'eau (33% air), c'est le premier niveau "aérien"

### Composition de surface (bloc le plus haut non-air)

| Surface | % des colonnes | Biome associé |
|---|---|---|
| `sand` | 54.1% | desert, beach, ocean floor |
| `grass_block` | 10.8% | plains, savanna |
| `gravel` | 7.3% | ocean floor, river |
| `stone` (exposé) | 6.1% | falaises, windswept |
| `seagrass` | 3.5% | ocean |
| `short_grass` | 3.2% | plains, savanna |
| `hardened_clay` | 2.0% | badlands |
| `dirt` | 1.7% | divers |
| `red_sand` | 1.6% | badlands |
| `sandstone` | 1.4% | desert (exposé) |
| coraux divers | ~2.5% | warm_ocean |

### Épaisseur de la couche de surface

```
Min: 1, Max: 68, Médiane: 3, Moyenne: 3.4
```

La plupart des colonnes ont **3 blocs** de matériau de surface avant de toucher la stone/deepslate.

### Structure typique d'une colonne désert

```
Y=68 : sand          (1-3 blocs)
Y=65 : sandstone     (8-12 blocs)
Y=57 : stone         (zone stone)
Y=5  : transition    (deepslate/stone mix)
Y=0  : deepslate     (zone deepslate)
Y=-60: bedrock mix
Y=-64: bedrock solid
```

---

## 5. Heightmap (profil du terrain)

```
Min:      82
Max:     168
Médiane: 134
Moyenne: 135.3
Écart-type: 8.43
```

### Distribution des hauteurs

```
  80-83   [    1]
 112-115  [    4]
 116-119  [   89]
 120-123  [  297]
 124-127  [20320] ██████████████████████████████████████████████████  ← pic principal
 128-131  [ 4470] ██████████
 132-135  [14327] ███████████████████████████████████
 136-139  [10829] ██████████████████████████
 140-143  [ 6412] ███████████████
 144-147  [ 3677] █████████
 148-151  [ 3258] ████████
 152-155  [ 1390] ███
 156-159  [ 1151] ██
 160-163  [  800] █
 164-167  [  292]
 168-171  [    9]
```

Le terrain a **deux pics** :
- Y=124-127 : terrain plat océan/désert bas (~30%)
- Y=132-143 : collines/plateaux (~47%)
- Queue jusqu'à Y=168 : badlands/montagnes

---

## 6. Grottes (densité d'air souterrain)

### Profil de densité air par Y

```
Y=-64 à -55 :  0% air (solide complet, bedrock+deepslate)
Y=-54       :  7% air (début des grottes)
Y=-40       : 20% air (grottes profondes, pic)
Y=-32       : 26% air (maximum de grottes deepslate)
Y=-17       : 13% air (grottes se referment)
Y=  0       : 13% air (transition zone)
Y= 14       : 16% air (deuxième zone de grottes, dans stone)
Y= 25       : 14% air
Y= 40       :  7% air (grottes rares)
Y= 62       : 30% air (mais c'est l'eau → surface)
```

Deux couches de grottes distinctes :
1. **Grottes profondes** (Y=-54 à Y=-17) : pic à 26% air, dans la deepslate
2. **Grottes normales** (Y=8 à Y=30) : ~15% air, dans la stone

---

## 7. Biomes détectés

| Biome ID | Nom | Proportion |
|---|---|---|
| 0 | ocean | 37.4% |
| 2 | desert | 35.7% |
| 40 | warm_ocean | 8.9% |
| 35 | savanna | 4.5% |
| 1 | plains | 4.4% |
| 25 | deep_ocean | 2.2% |
| 7 | river | 0.7% |
| 42 | cold_ocean | 0.3% |
| 16 | beach | trace |
| 21 | jungle | trace |

---

## 8. Lava souterraine

```
Y=-59 : lava 1.4%
Y=-58 : lava 2.4%
Y=-57 : lava 3.7%
Y=-56 : lava 5.4%
Y=-55 : lava 5.8%  ← pic
Y=-54 : lava disparaît (remplacée par air/water dans les grottes)
```

La lave remplit les grottes les plus profondes (Y=-59 à Y=-55), au-dessus c'est de l'air.

---

## 9. Résumé des paramètres pour le générateur

```
BEDROCK_FLOOR     = -64
BEDROCK_CEILING   = -60  (dégradé 100%→0% sur 5 niveaux)
DEEPSLATE_FLOOR   = -60
DEEPSLATE_CEILING = +4   (transition linéaire sur 8 niveaux)
STONE_FLOOR       = +5
SEA_LEVEL         = 62
SURFACE_DEPTH     = 3    (médiane)
TERRAIN_HEIGHT_AVG = 135
TERRAIN_HEIGHT_STD = 8.4
TERRAIN_HEIGHT_MIN = 82
TERRAIN_HEIGHT_MAX = 168

CAVE_LAYER_1_Y    = -54 à -17  (deepslate caves, max 26% air)
CAVE_LAYER_2_Y    = +8 à +30   (stone caves, max 16% air)
LAVA_LEVEL        = -55         (lave dans les grottes < Y=-54)

GRANITE_PCT  = 6-7%  (dans zone stone)
DIORITE_PCT  = 6-7%
ANDESITE_PCT = 6-8%
TUFF_PCT     = 5-9%  (dans zone deepslate)
```
