# 20 - Bedrock Edition World Generation — Recherche

## Objectif

Remplacer la génération PocketMine (simplifiée) par la vraie génération Bedrock Edition.
Données extraites des fichiers BDS (`.reference/bds/`), du protocole BedrockProtocol,
du projet BetterVanillaGenerator, et de la documentation officielle.

---

## Architecture de la génération Bedrock

```
1. Sélection de biome (multi-noise 4D ou climate-based)
2. Terrain noise (3 couches : low, high, selector)
3. Density → blocks (stone si density > 0, air/water sinon)
4. Surface builder (ground cover par biome)
5. Features/populators (arbres, minerais, végétation, caves)
```

---

## 1. Les 3 Noise Maps du Terrain

Bedrock utilise 3 couches de bruit Perlin (pas Simplex) pour sculpter le terrain :

### Low Noise (terrain lisse, grandes collines)
| Paramètre | Valeur |
|---|---|
| Octaves | 16 |
| Fréquence X/Z | 0.005221649... |
| Fréquence Y | 0.002610824... |
| Période X/Z | ~191.5 blocs |
| Période Y | ~383 blocs |

### High Noise (terrain rugueux, détails)
| Paramètre | Valeur |
|---|---|
| Octaves | 16 |
| Fréquence X/Z | 0.016709... |
| Fréquence Y | 0.008354... |
| Période X/Z | ~59.8 blocs |
| Période Y | ~119.7 blocs |

### Selector Noise (interpolation low↔high)
| Paramètre | Valeur |
|---|---|
| Octaves | 8 |
| Fréquence X/Z/Y | 0.001527... |
| Période | ~655 blocs |

### Formule de combinaison
```
selector = clamp(selector_noise, 0.0, 1.0)

if selector == 0.0:
    density = low_noise
elif selector == 1.0:
    density = high_noise
else:
    density = low_noise + (high_noise - low_noise) * selector

// Ajustement de hauteur basé sur le biome
height_adjustment = (y_index - base_height) * stretch_y * 128.0 / 256.0 / avg_scale
density -= height_adjustment

// Résultat
if density > 0 → STONE
elif y <= 63    → WATER (sea level)
else            → AIR
```

---

## 2. Constantes Globales du Terrain

| Constante | Valeur |
|---|---|
| Coordinate Scale | 684.412 |
| Height Scale | 684.412 |
| Height Noise Scale X/Z | 200.0 |
| Detail Noise Scale X | 80.0 |
| Detail Noise Scale Y | 160.0 |
| Detail Noise Scale Z | 80.0 |
| Surface Scale | 0.0625 |
| Base Size | 8.5 |
| Stretch Y | 12.0 |
| Sea Level | 64 |
| Biome Depth Weight | 1.0 |
| Biome Scale Weight | 1.0 |

---

## 3. Biome Noise Types → depth/scale

Chaque biome a un `noise_type` qui donne ses paramètres de hauteur :

| noise_type | depth | scale | Description |
|---|---|---|---|
| lowlands | 0.125 | 0.05 | Plaines plates |
| default | 0.1 | 0.2 | Collines normales |
| default_mutated | 0.2 | 0.2 | Collines variantes |
| taiga | 0.2 | 0.2 | Taïga |
| mountains / hills | 0.45 | 0.3 | Collines hautes |
| extreme | 1.0 | 0.5 | Montagnes extrêmes |
| less_extreme | 0.2 | 0.4 | Collines modérées |
| highlands | 1.5 | 0.025 | Plateaux hauts |
| beach | 0.0 | 0.025 | Plage plate |
| stone_beach | 0.1 | 0.8 | Côte rocheuse |
| ocean | -1.0 | 0.1 | Océan |
| deep_ocean | -1.8 | 0.1 | Océan profond |
| river | -0.5 | 0.0 | Rivière |
| swamp | -0.2 | 0.1 | Marais |
| mushroom | 0.2 | 0.3 | Île champi |

---

## 4. Sampling du Terrain

- Grille de densité : **5x33x5** par chunk (tous les 4 blocs en X/Z, tous les 8 blocs en Y)
- **Interpolation trilinéaire** pour remplir les 16x256x16 blocs
- Pour chaque colonne de densité, les biomes voisins (grille 5x5) sont moyennés avec un kernel Gaussien pour lisser les transitions

### Algorithme de moyennage biome
```
Pour chaque point de sampling (5x5 biome grid) :
    weight = ELEVATION_WEIGHT[dx][dz] / (biome_depth + 2.0)
    avgDepth += depth * weight
    avgScale += scale * weight
    totalWeight += weight

avgDepth /= totalWeight
avgScale /= totalWeight
```

---

## 5. Surface Builder (par biome)

Chaque biome définit ses matériaux de surface dans les fichiers BDS JSON :

| Biome | Top | Mid | Foundation | Sea Floor |
|---|---|---|---|---|
| Plains | grass_block | dirt | stone | gravel |
| Desert | sand | sand | stone | gravel |
| Forest | grass_block | dirt | stone | gravel |
| Taiga | grass_block | dirt | stone | gravel |
| Jungle | grass_block | dirt | stone | gravel |
| Swamp | grass_block | dirt | stone | gravel |
| Ocean | grass_block | dirt | stone | gravel |
| Extreme Hills | grass_block | dirt | stone | gravel |
| Mesa | red_sand | hardened_clay | stone | gravel |
| Mushroom | mycelium | dirt | stone | gravel |

### Surface Material Adjustments
Certains biomes ont des ajustements conditionnels :
- **Extreme Hills** : pierre exposée quand noise > 0.121 (fréquence 0.0625)
- **Beach** : sable en surface basé sur le bruit
- **Mesa** : couches de terracotta colorée

---

## 6. Multi-Noise Biome Selection (4D)

4 paramètres de bruit + poids pour chaque biome :

| Biome | temp | humidity | altitude | weirdness | weight |
|---|---|---|---|---|---|
| plains | 0.0 | 0.0 | 0.0 | 0.0 | 0.4 |
| forest | 0.0 | 0.5 | 0.0 | 0.0 | 0.375 |
| forest_hills | 0.0 | 0.5 | 0.2 | 0.0 | 0.375 |
| desert | 0.4 | -0.4 | 0.0 | 0.0 | 0.1 |
| desert_hills | 0.4 | -0.4 | 0.2 | 0.0 | 0.1 |
| ocean | 0.0 | 0.1 | -0.1 | 0.0 | 0.3 |
| savanna | 0.1 | -0.1 | 0.0 | 0.1 | 0.2 |
| taiga | 0.0 | -0.4 | 0.0 | 0.0 | 0.1 |
| taiga_hills | 0.0 | -0.5 | 0.2 | 0.0 | 0.0 |
| jungle | 0.4 | 0.0 | 0.0 | 0.0 | 0.0 |
| jungle_hills | 0.4 | 0.0 | 0.2 | 0.0 | 0.0 |
| ice_plains | -1.0 | 1.0 | 0.2 | 1.0 | 0.0 |
| extreme_hills | — | — | — | — | — |

L'algorithme cherche le biome le plus proche dans l'espace 4D du point de bruit.

---

## 7. Minerais (Feature Rules BDS)

Depuis `.reference/bds/server/definitions/feature_rules/` :

| Minerai | Iterations/chunk | Y min | Y max |
|---|---|---|---|
| Coal | 20 | 0 | 128 |
| Iron | 20 | 0 | 64 |
| Gold | 2 | 0 | 32 |
| Diamond | 1 | 0 | 16 |
| Redstone | 8 | 0 | 16 |
| Lapis | 1 | 0 | 32 |
| Emerald | biome-specific | 4 | 32 |
| Copper | 10 | 0 | 96 |
| Dirt (poches) | 20 | 0 | 128 |
| Gravel (poches) | 10 | 0 | 128 |
| Granite | 10 | 0 | 80 |
| Diorite | 10 | 0 | 80 |
| Andesite | 10 | 0 | 80 |

---

## 8. Différences clés vs PocketMine

| Aspect | PocketMine | Bedrock réel |
|---|---|---|
| **Noise** | 1 Simplex 3D (4 oct.) | 3 Perlin (16+16+8 oct.) + selector |
| **Sampling** | 16x16 + interpolation 4/8/4 | 5x33x5 + trilinéaire |
| **Biomes** | 2D (temp+rain lookup) | 4D multi-noise |
| **Hauteur** | fixe par biome (min/max) | depth/scale + densité 3D |
| **Caves** | aucune | noise caves 3D (spaghetti, noodle, cheese) |
| **Eau** | Y=62 fixe | Aquifères dynamiques |
| **Surface** | 1 cover array | Builder avec adjustments conditionnels |
| **Sea level** | 62 | 64 |

---

## 9. Sources et Références

### Fichiers locaux
- Biomes BDS : `.reference/bds/server/behavior_packs/vanilla/biomes/` (71 fichiers)
- Feature Rules : `.reference/bds/server/definitions/feature_rules/` (120 fichiers)
- Features : `.reference/bds/server/behavior_packs/vanilla_1.17.0/features/`
- BedrockProtocol : `.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/types/biome/`

### Projets open-source
- **BetterVanillaGenerator** (Java/Nukkit) — Le plus complet pour Bedrock terrain
  - https://github.com/wode490390/BetterVanillaGenerator
- **Cubiomes** (C) — Biome generation rapide (Java Edition, algorithmiquement proche)
  - https://github.com/Cubitect/cubiomes

### Documentation
- https://wiki.bedrock.dev/world-generation/heightmap-noise
- https://wiki.bedrock.dev/world-generation/biomes
- https://learn.microsoft.com/en-us/minecraft/creator/documents/biomes/biomeoverview

---

## 10. Plan d'implémentation proposé

### Phase A : Perlin Noise + 3 couches terrain
1. Implémenter Perlin noise (pas Simplex) avec multi-octave fBm
2. Créer les 3 noise maps (low, high, selector)
3. Formule de densité avec biome depth/scale
4. Sampling 5x33x5 + trilinéaire

### Phase B : Biomes Bedrock
1. Charger les 71 biomes depuis les JSON BDS
2. Sélection multi-noise 4D (ou climate-based pour commencer)
3. noise_type → depth/scale mapping
4. Gaussian averaging 5x5 pour transitions

### Phase C : Surface Builder
1. Surface materials par biome (top, mid, foundation, sea_floor)
2. Surface material adjustments (noise-based)
3. Sea level = 64

### Phase D : Features avancées
1. Caves (noise-based, si possible)
2. Arbres par biome (depuis feature rules)
3. Minerais avec distribution BDS
