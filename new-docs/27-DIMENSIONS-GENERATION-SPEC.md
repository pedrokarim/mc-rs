# Spec de génération — Nether & End (référence canonique)

> Source autoritaire : données vanilla **data-driven** dans
> `.reference/mcmeta/data/minecraft/worldgen/` (biome / configured_feature /
> placed_feature / noise_settings), recoupées avec minecraft.wiki et bedrock.dev.
> **Toute valeur ici est tirée des fichiers vanilla, pas devinée.** Les générateurs
> mc-rs doivent reproduire ces nombres exactement.
>
> ⚠️ Édition Bedrock : les 5 biomes de l'End sont **fusionnés en `the_end` (id 9)**
> côté client, mais la *distribution des features* suit la classification Java.

---

## 0. Conventions

- **Plages de hauteur** (Nether : Y 0..128, top=128, bottom=0) :
  - `above_bottom: X` → Y = `0 + X`
  - `below_top: X` → Y = `128 − X`
  - `absolute: X` → Y = `X`
- **`count`** = nombre de placements (veines/features) **par chunk**.
- **`count_on_every_layer: N`** = N tentatives par tranche de 16 blocs de hauteur,
  chacune posée sur un sol valide (→ densité élevée sur toute la colonne).
- **`rarity_filter 1/n`** = 1 chance sur n que la feature soit placée dans le chunk.
- **Feature `ore`** (vein) : remplit une veine ellipsoïdale d'au plus `size` blocs le
  long d'un segment aléatoire ; `discard_chance_on_air_exposure` = proba de retirer
  un bloc de la veine s'il touche l'air.
- **Feature `scattered_ore`** : comme `ore` mais blocs dispersés (utilisé pour
  l'ancient debris) ; cible un *tag* `base_stone_nether` (netherrack/basalt/blackstone).

---

## 1. NETHER — terrain (✅ déjà fidèle)

- Bornes **Y 0..128** (8 sub-chunks), `dimension_id = 1`.
- `final_density` vanilla (`noise_settings/nether.json`) : `old_blended_noise`
  (`nether/base_3d_noise`) + gradients Y en cloche → voûtes de netherrack.
- Cellules 4 (horiz.) × 8 (vert.), interpolation trilinéaire.
- Bloc plein = **netherrack** ; **lave** dans tout vide à Y ≤ 32.
- **Bedrock** : gradient dégressif sur 5 couches au plancher (Y0-4) et au plafond
  (Y123-127) — `vertical_gradient` `above_bottom 0..5` / `below_top 0..5`.

## 2. NETHER — biomes (multi-noise, ✅ mapping fait)

Points climat (`biome_parameters/nether.json`, valeurs `MultiNoiseBiomeSource.NETHER`) :

| Biome | temperature | humidity | weirdness |
|---|---|---|---|
| nether_wastes | 0 | 0 | 0 |
| soul_sand_valley | 0 | −0.5 | 0 |
| crimson_forest | 0.4 | 0 | 0 |
| warped_forest | 0 | 0.5 | 0.375 |
| basalt_deltas | −0.5 | 0 | 0.175 |

Sélection par `temperature` + `vegetation(=humidity)` (les autres paramètres = 0).

## 3. NETHER — surface rule (par biome)

Tirée de `noise_settings/nether.json → surface_rule`. Ordre = priorité.
`depth(ceiling)` = faces tournées vers le bas (plafonds) ; `depth(floor)` = sols.

| Biome | Sol (floor) | Plafond (ceiling) | Variante |
|---|---|---|---|
| (tous) bord monde | bedrock (5 couches gradient bas+haut) | — | |
| (défaut, Y > top−5) | netherrack | — | |
| **basalt_deltas** | basalt ; **gravel** si Y > 30 | basalt | |
| **soul_sand_valley** | soul_sand ; **gravel** si Y < 30 | soul_sand | |
| **crimson_forest** | nether_wart_block (sous nylium) | — | nylium en top layer via features |
| **warped_forest** | warped_wart_block (sous nylium) | — | nylium en top layer via features |
| **nether_wastes** | netherrack ; patches soul_sand / gravel ~Y30-32 | — | |

> Note : le **nylium** (crimson/warped) de surface est posé par la couche de
> végétation (les fungi/roots poussent dessus) ; la surface rule pose le wart block
> sous la surface. À reproduire : top = nylium dans les forêts.

## 4. NETHER — minerais (veines, `count`/chunk)

| Feature | Biomes | count/chunk | size (vein) | Plage Y | discard_air | Bloc |
|---|---|---|---|---|---|---|
| ore_quartz_nether | wastes, crimson, warped, soul | **16** | 14 | 10 → 118 | 0.0 | nether_quartz_ore |
| ore_quartz_deltas | basalt_deltas | **32** | 14 | 10 → 118 | 0.0 | nether_quartz_ore |
| ore_gold_nether | wastes, crimson, warped, soul | **10** | 10 | 10 → 118 | 0.0 | nether_gold_ore |
| ore_gold_deltas | basalt_deltas | **20** | 10 | 10 → 118 | 0.0 | nether_gold_ore |
| ore_magma | tous | **4** | 33 | 27 → 36 | 0.0 | magma_block |
| ore_gravel_nether | tous sauf deltas | **2** | 33 | 5 → 41 | 0.0 | gravel |
| ore_blackstone | tous sauf deltas | **2** | 33 | 5 → 31 | 0.0 | blackstone |
| ore_soul_sand | soul_sand_valley | **12** | 12 | 0 → 31 | 0.0 | soul_sand |
| ore_ancient_debris_large | tous | **1** | 3 (scattered) | 8 → 24 (trapèze) | 1.0 | ancient_debris |
| ore_debris_small | tous | **1** | 2 (scattered) | 8 → 120 (uniforme) | 1.0 | ancient_debris |

## 5. NETHER — décorations / features (par biome)

### Communes (toutes ou presque)
| Feature | count/chunk | Plage Y | Détail |
|---|---|---|---|
| glowstone | **10** (Java) | 0 → 128 | blob de glowstone (`glowstone_blob`) accroché au plafond. ⚠️ Java=10 rend bien trop dense visuellement ; **Bedrock ≈ 2-3/chunk** → on génère 2-3 |
| glowstone_extra | 0..9 (biais bas) | 4 → 124 | blobs supplémentaires |
| patch_fire | 0..5 | 4 → 124 | feu sur netherrack |
| patch_soul_fire | 0..5 | 4 → 124 | soul fire (sauf crimson) |
| spring_open (nether_open) | **8** | — | source de lave « ouverte » |
| spring_closed (nether_closed) | **16** | — | source de lave « fermée » |
| spring_lava | **20** | — | (étape végétation) |
| brown/red_mushroom_nether | 1/2 | — | champignon au sol |

### crimson_forest (étape veg)
- `weeping_vines` : count **10** (lianes pleureuses au plafond)
- `crimson_fungi` : `huge_fungus`, **every_layer 8** (gros fungi crimson)
- `crimson_forest_vegetation` : `nether_forest_vegetation`, **every_layer 6**,
  spread 8×4, blocs pondérés : crimson_roots **87**, crimson_fungus **11**, warped_fungus **1**

### warped_forest (étape veg)
- `warped_fungi` : `huge_fungus`, **every_layer 8**
- `warped_forest_vegetation` : every_layer **5**, pondéré : warped_roots **85**,
  warped_fungus **13**, crimson_roots **1**, crimson_fungus **1**
- `nether_sprouts` : every_layer **4**
- `twisting_vines` : count **10** (max_height 8, spread 8×4) — lianes montantes

### soul_sand_valley
- `basalt_pillar` (étape local_mod) : count **10** — piliers de basalte coiffés de magma
- `patch_crimson_roots` : `random_patch` (tries 96, xz_spread 7, y_spread 3)
- (+ ore_soul_sand cf. §4)

### basalt_deltas (étape surface_structures)
- `delta` : `delta_feature`, **every_layer 40** — lacs de lave, bord magma, taille 3-7, rim 0-2
- `small_basalt_columns` : every_layer **4** (hauteur 1-4, reach 1)
- `large_basalt_columns` : every_layer **2** (hauteur 5-10, reach 2-3)
- `basalt_blobs` : count **75** — remplace netherrack par basalte (rayon 3-7)
- `blackstone_blobs` : count **25** — rayon 3-7
- `spring_closed_double` : count **32** ; `spring_delta` : count **16**

### huge_fungus (crimson_fungus / warped_fungus)
- `HugeFungusFeature` (code vanilla) : tige (`crimson_stem`/`warped_stem`) hauteur
  **4..13**, chapeau de `nether_wart_block`/`warped_wart_block` (rayon ~2, dôme), décor
  `shroomlight` semé dans le chapeau. `planted: false` (généré spontanément).

---

## 6. END — terrain (✅ déjà fidèle)

- Bornes **Y 0..256** (16 sub-chunks), `dimension_id = 2`. Pas de fluide.
- `final_density` = `end_islands + base_3d_noise` puis gradients Y.
- Nœud **`end_islands`** implémenté → **île centrale** dômée (`clamp(100 − dist·8)`)
  + **îles externes** au-delà de **~1024 blocs**, séparées par l'**anneau de vide**.
  Structure vérifiée par test (centre solide / anneau == 0 / externes > 0).

## 7. END — biomes (classification Java, fusionnés en `the_end` côté Bedrock)

`TheEndBiomeSource` :
- `dist² ≤ 4096` (≈ 64 blocs) → **the_end** (île centrale)
- sinon, selon une valeur de bruit `f` (hauteur d'île, même simplex que `end_islands`) :
  - `f > 0.25` → **end_highlands** (grandes îles externes ; chorus + end city)
  - `f ≥ −0.0625` → **end_midlands** (intérieur, nu)
  - `f > −0.21875` → **end_barrens** (bords abrupts, nu)
  - sinon → **small_end_islands** (petites îles éparses dans le vide)

## 8. END — features (par biome)

| Biome | Étape | Feature | Détail |
|---|---|---|---|
| the_end | raw_gen | **end_spike** | les 10 piliers (cf. §9), `crystal_invulnerable:false` |
| the_end | top_layer | **end_platform** | plateforme d'obsidienne d'arrivée (réservée) |
| end_highlands | raw_gen | **end_gateway_return** | gateway de retour, exit `[100,50,0]`, `exact:true` |
| end_highlands | veg | **chorus_plant** | chorus trees sur end_stone (cf. §10) |
| end_midlands | — | (aucune) | nu |
| end_barrens | — | (aucune) | nu |
| small_end_islands | raw_gen | **end_island_decorated** | petites îles (blob end_stone) |

## 9. END — end spikes (piliers d'obsidienne) ✅ fait

Réf wiki/w/Obsidian_pillar + `SpikeFeature`. **10 piliers, positions FIXES** (cercle
r42 autour de (0,0)) : (-42,-1) (-34,-25) (-34,24) (-13,-40) (-13,39) (12,-40) (12,39)
(33,-25) (33,24) (42,0). Specs attribuées par mélange seed :

| rang | rayon | hauteur (top) | cage de fer |
|---|---|---|---|
| 0 | 3 | 76 | non |
| 1 | 3 | 79 | **oui** |
| 2 | 3 | 82 | **oui** |
| 3 | 4 | 85 | non |
| 4 | 4 | 88 | non |
| 5 | 4 | 91 | non |
| 6 | 5 | 94 | non |
| 7 | 5 | 97 | non |
| 8 | 5 | 100 | non |
| 9 | 6 | 103 | non |

- Obsidienne Y0 → hauteur, **bedrock** au sommet, **end crystal** (entité) à `height+1`.
- Cage : **73 barreaux de fer** (5×5, parois |dx|=2 ou |dz|=2 + couche du dessus) sur
  les 2 piliers gardés (rangs 1 & 2). Un bloc de feu sous le cristal (vanilla).

## 10. END — chorus plants & gateways

- **chorus_plant** (`ChorusPlantFeature`) : pousse sur **end_stone** à l'air libre.
  Arbre corail récursif : tronc `chorus_plant` qui ramifie horizontalement puis
  verticalement, `chorus_flower` (age 0..5) aux extrémités. Densité sur end_highlands.
- **end_gateway** : après la mort du dragon, jusqu'à **20 gateways** en cercle (r ~96)
  autour du portail de sortie ; téléporte vers les îles externes (et retour).
  `end_gateway_return` : sortie fixe `[100,50,0]`.
- **exit portal / fountain** : centre `(0,64,0)`, généré à la mort du dragon (déjà géré
  par `dragon_fight_manager`), œuf de dragon au 1er kill.

---

## 11. État de l'implémentation (mis à jour 2026-06-09)

Décorateurs `nether_decorate.rs` / `end_decorate.rs` + moteur `world/feature.rs`.

- ✅ **Moteur de features** : `world/feature.rs` — `OreFeature` (veine ellipsoïde
  fidèle + scattered), `Region`, `HeightRange` (uniform/trapezoid + ancrages),
  `Count`, RNG par chunk+salt. Réutilisable.
- ✅ **Minerais** : vraies veines `count`/chunk + `size` + plages Y (§4). Fini la
  proba/bloc. Probe : quartz ~220/chunk, debris rare.
- ✅ **Springs** : lave open (8) + closed (16).
- ✅ **basalt_deltas** : blobs basalt (75) / blackstone (25), deltas de lave
  (every_layer 40, rim magma), colonnes small (ev4) + large (ev2).
- ✅ **Végétation** : `count_on_every_layer` pondéré (listes vanilla) + huge fungus
  (tige 4-13 + dôme wart block + shroomlight).
- ✅ **Surface** : surface rule par biome (nylium, soul sand/gravel, basalt/gravel,
  patches wastes).
- ✅ **glowstone** : count 10 + extra, blobs accrochés aux plafonds.
- ✅ **feu** : patch_fire / patch_soul_fire (0-5/chunk).
- ✅ **End chorus** : croissance récursive fidèle (`ChorusPlantFeature`).
- ✅ **End spikes** : §9 (positions/hauteurs/cages exactes).

### Reste (suivis, hors « génération de terrain » pure)
- **end_gateway** : sortie post-dragon vers les îles externes → mécanique de TP.
- **Structures** : nether_fortress, bastion, end_city (loot data déjà présente).
- **Parité de seed Java** : les RNG sont déterministes par chunk mais pas
  bit-identiques à Java (positions différentes, statistiques identiques).
