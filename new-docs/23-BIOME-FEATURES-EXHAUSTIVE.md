# 23 - Contenu exhaustif de chaque biome Bedrock Edition

Source : fichiers feature_rules du Bedrock Dedicated Server (`.reference/bds/server/definitions/feature_rules/`)
et biome JSON (`.reference/bds/server/behavior_packs/vanilla/biomes/`).

Les nombres sont en **itérations par chunk** (16x16 blocs).

---

## Éléments globaux (tous biomes overworld)

| Feature | Iterations/chunk | Notes |
|---|---|---|
| Reeds (canne à sucre) | 10 | Près de l'eau, sur sable/terre |
| Pumpkin (citrouille) | 1 | Très rare |
| Brown mushroom | 1 | Rare, zones sombres |
| Red mushroom | 1 | Rare, zones sombres |
| Sand patches | 3 | Sauf swamp |
| Gravel patches | 1 | Sauf swamp |
| Clay patches | 1 | Sous l'eau |
| Springs (sources) | 1 | Flancs de collines |
| Coal ore | 20 | Underground |
| Iron ore | 20 | Underground |
| Gold ore | 2 | Underground (Y < 32) |
| Diamond ore | 1 | Underground (Y < 16) |
| Redstone ore | 8 | Underground (Y < 16) |
| Lapis ore | 1 | Underground (Y < 32) |
| Dirt pockets | 10 | Underground |
| Gravel pockets | 8 + 80 (extra) | Underground |
| Granite | 10 | Underground |
| Diorite | 10 | Underground |
| Andesite | 10 | Underground |

---

## Plains (plaines)

| Feature | Iterations | Notes |
|---|---|---|
| Tall grass | 10 (ou 5 si flower patch) | Herbe courte |
| Double plant grass | 7 | Herbes hautes à 2 blocs |
| Flowers | 4 (ou 15 si flower patch) | Patchs de fleurs aléatoires |
| Trees | 1 | Chêne, rare |
| **Total végétation** | **~22/chunk** | |

**Flower patches** : noise-based, ~20% des chunks ont un cluster dense de 15 fleurs.

**Surface** : grass_block, dirt
**Structures** : villages, pillager outposts

---

## Sunflower Plains

Comme Plains + :

| Feature | Iterations | Notes |
|---|---|---|
| Sunflowers (double plant) | 10 | Tournesols |

---

## Desert (désert)

| Feature | Iterations | Notes |
|---|---|---|
| Cactus | 10 | |
| Dead bush | 2 | Buissons secs |
| Reeds | 50 | Canne à sucre (près de l'eau) |
| Desert well | 1 (rare) | Structure |
| Fossils | 1 (très rare) | Sous-terrain |

**Surface** : sand (3 blocs), sandstone (8-12 blocs)
**Structures** : desert temple, desert well, villages

---

## Forest (forêt)

| Feature | Iterations | Notes |
|---|---|---|
| Trees (oak + birch mix) | tree_feature x1 | Dense |
| Foliage | 1 | Décoration au sol |
| Flowers | 2 | |

**Surface** : grass_block, dirt
**Structures** : — (pas de structures spécifiques)

---

## Birch Forest

| Feature | Iterations | Notes |
|---|---|---|
| Trees (birch only) | 1 | Bouleaux |
| Flowers | 2 | |

**Surface** : grass_block, dirt

---

## Flower Forest

| Feature | Iterations | Notes |
|---|---|---|
| Flowers | **100** | Toutes variétés ! |
| Trees | 1 | |
| Foliage | 1 | |

**Types de fleurs** : dandelion, poppy, allium, azure bluet, tulips, oxeye daisy, cornflower, lily of the valley
**Surface** : grass_block, dirt

---

## Roofed Forest (forêt sombre)

| Feature | Iterations | Notes |
|---|---|---|
| Dark oak trees | **16** | Très dense |
| Huge mushrooms | inclus | Champignons géants |

**Surface** : grass_block, dirt
**Structures** : woodland mansion (très rare)

---

## Taiga

| Feature | Iterations | Notes |
|---|---|---|
| Trees (spruce) | 1 | Sapins |
| Tall grass | 1 | Peu d'herbe |
| Mushrooms | 1 | |
| Double fern | 1 | Fougères doubles |
| Sweet berry bush | 1 | Baies sucrées |

**Surface** : grass_block, dirt
**Structures** : villages

---

## Cold Taiga (taïga enneigée)

| Feature | Iterations | Notes |
|---|---|---|
| Trees (spruce) | 1 | |
| Sweet berry bush | 1 | |
| Snow layer | surface | Couche de neige |

**Surface** : snow_layer, grass_block, dirt

---

## Mega Taiga (taïga géante)

| Feature | Iterations | Notes |
|---|---|---|
| Trees (mega spruce) | 1 | Sapins géants 2x2 |
| Tall grass | 7 | |
| Dead bush | 1 | |
| Mushrooms | 3 | |
| Forest rocks (mossy cobblestone) | 1 | Rochers moussus |

**Surface** : podzol, dirt

---

## Jungle

| Feature | Iterations | Notes |
|---|---|---|
| Trees (jungle, cocoa) | 1 | Arbres géants |
| Tall grass | 25 | Beaucoup d'herbe |
| Flowers | 4 | |
| Bamboo | 16 | After surface |
| Melon | 1 | Pastèques |
| Vines (lianes) | **50** | Très abondant |

**Surface** : grass_block, dirt
**Structures** : jungle temple

---

## Bamboo Jungle

| Feature | Iterations | Notes |
|---|---|---|
| Bamboo | **15-160** (noise-based) | Massif |
| Tall grass | **150** | Énorme densité |
| Trees | 1 | |

**Surface** : grass_block, podzol (sous bambou)

---

## Jungle Edge

| Feature | Iterations | Notes |
|---|---|---|
| Trees (jungle, smaller) | 1 | |
| Tall grass | 25 | |

---

## Savanna (savane)

| Feature | Iterations | Notes |
|---|---|---|
| Tall grass | **20** | Beaucoup |
| Flowers | 4 | |
| Trees (acacia) | 1 | Acacias |
| Double plant grass | 1 | |

**Surface** : grass_block, dirt (couleur plus jaune)
**Structures** : villages

---

## Savanna Mutated (savane escarpée)

| Feature | Iterations | Notes |
|---|---|---|
| Tall grass | 5 | |
| Trees | 1 | Terrain très escarpé |

---

## Savanna Plateau

Comme Savanna mais en altitude (highlands).

---

## Extreme Hills (montagnes)

| Feature | Iterations | Notes |
|---|---|---|
| Trees (spruce + oak) | 1 | Peu d'arbres |
| Emerald ore | 1 | Exclusif ! |
| Silverfish blocks | 7 | Infested stone |

**Surface** : grass_block, dirt (avec patches de stone exposée)
**Structures** : —

---

## Extreme Hills + Trees

Comme Extreme Hills + plus d'arbres (sapins).

---

## Swampland (marais)

| Feature | Iterations | Notes |
|---|---|---|
| Trees (oak avec vines) | swamp_foliage x1 | Chênes avec lianes |
| Tall grass | 5 | |
| Flowers (blue orchid) | 1 | Orchidées bleues |
| Mushrooms | **8** | Beaucoup de champis |
| Reeds | 10 | |
| Waterlily (nénuphars) | **4** | Sur l'eau |
| Dead bush | 1 | |
| Fossils | 1 (très rare) | |

**Surface** : grass_block, dirt (eau peu profonde)
**Structures** : witch hut

---

## Ocean

| Feature | Iterations | Notes |
|---|---|---|
| Seagrass | 12 | Herbe marine |

**Surface** : gravel (fond marin)
**Structures** : ocean monument (deep), shipwreck, ocean ruins

---

## Warm Ocean

| Feature | Iterations | Notes |
|---|---|---|
| Coral | 4 | |
| Coral crust | 1 | |
| Coral hang | 16 | Coraux suspendus |
| Sea pickle | 4 | |
| Sea anemone | **20** | |

**Surface** : sand (fond)

---

## River

| Feature | Iterations | Notes |
|---|---|---|
| Seagrass | 12 | |

**Surface** : dirt

---

## Beach (plage)

**Surface** : sand
Pas de végétation spécifique (juste les globaux : reeds, pumpkin).

---

## Cold Beach

Comme Beach + neige.

---

## Stone Beach (côte rocheuse)

**Surface** : stone
Pas de végétation.

---

## Mesa / Badlands

| Feature | Iterations | Notes |
|---|---|---|
| Dead bush | **20** | Beaucoup ! |
| Cactus | 5 | |
| Reeds | 3 | |
| Gold ore (extra) | **20** | Y=32-80, exclusif au mesa |

**Surface** : red_sand, hardened_clay (couches de terracotta colorée dans le vrai jeu)
**Structures** : mineshaft (en surface)

---

## Mesa Plateau / Mesa Plateau Stone

Comme Mesa en altitude (highlands).
Mesa Plateau Stone a des arbres (sapins sur le plateau).

---

## Mushroom Island

| Feature | Iterations | Notes |
|---|---|---|
| Huge mushroom | 1 | Champignons géants |
| Small mushrooms | 1 | |

**Surface** : mycelium, dirt
**Particularité** : aucun mob hostile ne spawn

---

## Ice Plains

| Feature | Iterations | Notes |
|---|---|---|
| Trees (spruce) | 1 | Rare |
| Snow layer | surface | |

**Surface** : snow_layer, grass_block
**Structures** : igloo

---

## Ice Plains Spikes

| Feature | Iterations | Notes |
|---|---|---|
| Ice spikes | **3** | Tours de packed ice |
| Ice patches | 2 | |

**Surface** : snow_block, dirt

---

## Deep Ocean (toutes variantes)

Comme Ocean mais plus profond.
**Structures** : ocean monument

---

## Frozen Ocean / Frozen River

Comme Ocean/River avec glace en surface.

---

## Structures par biome (serveur-side)

Les structures sont générées par le **serveur**, pas le client. Le serveur place les blocs.

| Structure | Biomes | Notes |
|---|---|---|
| Village | plains, desert, savanna, taiga, snowy | Type varie par biome |
| Desert Temple | desert | |
| Jungle Temple | jungle | |
| Witch Hut | swamp | |
| Ocean Monument | deep_ocean | |
| Woodland Mansion | roofed_forest | Très rare |
| Pillager Outpost | plains, desert, savanna, taiga, snowy | |
| Igloo | ice_plains, cold_taiga | |
| Shipwreck | ocean (toutes variantes), beach | |
| Ocean Ruins | ocean (toutes variantes) | |
| Mineshaft | tous biomes (underground) | Mesa : en surface |
| Stronghold | tous biomes (underground) | |
| Dungeon | tous biomes (underground) | |
| Buried Treasure | beach | |
| Ruined Portal | tous biomes | |
| Desert Well | desert | |
| Fossil | desert, swamp | Underground |
| Trail Ruins | jungle, taiga | 1.20+ |

**Note** : Les structures ne sont pas encore implémentées dans mc-rs. Elles nécessitent des fichiers de structure (NBT) et un système de placement.

---

## Résumé : ce qui manque dans notre implémentation

### Végétation manquante
- [ ] Cactus (desert, mesa)
- [ ] Dead bush (desert, mesa, mega_taiga, swamp)
- [ ] Vines / lianes (jungle, swamp)
- [ ] Bamboo (jungle, bamboo_jungle)
- [ ] Reeds / canne à sucre (tous biomes, près de l'eau)
- [ ] Nénuphars / waterlily (swamp)
- [ ] Mushrooms / champignons (swamp, mega_taiga, taiga)
- [ ] Huge mushrooms (mushroom_island, roofed_forest)
- [ ] Melon (jungle)
- [ ] Pumpkin (très rare, tous biomes)
- [ ] Sweet berry bush (taiga, cold_taiga)
- [ ] Forest rocks / mossy cobblestone (mega_taiga)
- [ ] Coral (warm_ocean)
- [ ] Seagrass (ocean, river)
- [ ] Fallen logs (bois tombé) — existe dans Bedrock, très rare

### Arbres manquants (types)
- [ ] Birch tree (birch_forest)
- [ ] Spruce tree (taiga, mega_taiga, extreme_hills)
- [ ] Jungle tree (jungle) — géant 2x2
- [ ] Acacia tree (savanna) — forme en V
- [ ] Dark oak tree (roofed_forest) — 2x2
- [ ] Mega spruce (mega_taiga) — 2x2

### Structures (non implémentées)
- [ ] Toutes les structures ci-dessus

### Surface manquante
- [ ] Terracotta colorée en couches (mesa)
- [ ] Glace en surface (frozen_ocean, frozen_river)
- [ ] Patches de stone exposée (extreme_hills)
