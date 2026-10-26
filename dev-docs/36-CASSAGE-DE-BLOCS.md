# Cassage de blocs

Port du cassage de blocs de PocketMine-MP, avec les données de BDS là où PMMP s'arrête ou diverge du jeu.
Commencé le 2026-10-25.

## 1. Ce qui est porté

| PMMP 5.43.1 | mc-rs |
|---|---|
| `InGamePacketHandler::handlePlayerActionFromData` (START_BREAK, CONTINUE_DESTROY_BLOCK, ABORT_BREAK, STOP_BREAK, CRACK_BREAK, PREDICT_DESTROY_BLOCK, `validateFacing`) | `connection/block_breaking.rs`, `handle_block_break_action` |
| `InGamePacketHandler::syncBlocksNearby` (renvoi des blocs après un refus) | `sync_blocks_nearby` |
| `Player::attackBlock` (portée de 100 blocs, spectateur, geste, feu éteint sur la face visée) | `attack_block` |
| `Player::continueBreakBlock`, `stopBreakBlock` | CRACK_BREAK, `stop_break_block` |
| `Player::breakBlock`, `canInteract` (portée 7 en survie, 13 en créatif, cible devant le joueur) | `break_block`, `can_interact` |
| `SurvivalBlockBreakHandler` (vitesse, BLOCK_START_BREAK / BLOCK_BREAK_SPEED / BLOCK_STOP_BREAK, particule et son de coup tous les 5 ticks, arrêt au-delà de 16 blocs) | `BlockBreakHandler`, `tick_block_break` (appelé à chaque tick de jeu) |
| `World::useBreakOn` (air, incassable en survie, spectateur, aventure sans `CanDestroy`, drops et XP seulement avec ressources finies, BlockBreakEvent) | `use_break_on` |
| `BlockBreakInfo::isToolCompatible`, `getBreakTime` | `block_break.rs` |
| `Tool::getMiningEfficiency`, `Sword` (×1,5), Efficacité (niveau² + 1) | `HeldTool::mining_efficiency` |
| `Block::getDrops`, `getXpDropForTool` (Toucher de soie, outil incompatible) | `BreakProfile::drops`, `xp_drop` |
| `Item::onDestroyBlock` (usure 1, épée 2, cisailles toujours 1), `Durable::getUnbreakingDamageReduction` | `destroy_damage`, `unbreaking_reduction` |
| `World::dropExperience`, `ExperienceOrb::splitIntoOrbSizes` | `main.rs`, `passive_entities::split_into_orb_sizes` |
| `Entity::isUnderwater`, `Liquid::getFluidHeightPercent` | `is_underwater`, `liquids::water_height_percent` |
| `ItemStackExtraData::read` (NBT, CanPlaceOn, CanDestroy), `Item::getEnchantmentLevel` | `item_extra_data.rs` |

Vitesse par tick : `1 / (temps × 20)`, ×1/5 en l'air (hors vol), ×1/5 les yeux sous l'eau sans Affinité
aquatique, ×(1 + 0,2 L) × 1,2^L avec Célérité, ×0,21^L avec Fatigue de minage.

Comme PMMP, le serveur ne contrôle pas la durée du cassage : `PREDICT_DESTROY_BLOCK` est accepté dès que
la cible est à portée et cassable dans le mode de jeu du joueur.

## 2. Données : `data/block_break.json`

Produit par [`scripts/pmmp/export_block_break_data.php`](../scripts/pmmp/export_block_break_data.php), qui
exécute PMMP lui-même :

```bash
.reference/PocketMine-MP/bin/php/php.exe -d memory_limit=4G scripts/pmmp/export_block_break_data.php
```

Le PHP fourni avec PMMP (`bin/php`) est requis : il porte l'extension `encoding` dont PMMP 5.43 a besoin,
absente du PHP 8.5 installé par winget. Durée : une dizaine de secondes.

Pour chaque état de `canonical_block_states.nbt` (indice = runtime ID), le script :

1. désérialise l'état avec PMMP (`GlobalBlockStateHandlers::getDeserializer()`) ;
2. relève `BlockBreakInfo` et appelle `Block::getDrops()` avec chaque famille d'outil : main nue, outils en
   netherite (le meilleur rang satisfait tout niveau de récolte), cisailles, Toucher de soie, puis
   `getDropsForIncompatibleTool` avec et sans Toucher de soie ;
3. classe chaque résultat : liste fixe, `"random"` (deux tirages diffèrent ; 400 tirages pour les classes
   dont le code appelle l'aléatoire) ou `"world"` (drops lus dans la tuile) ;
4. relève l'XP : constante, ou bornes de `mt_rand(min, max)` ;
5. croise avec le dump de BDS 1.26.50 d'Allay (`data/resources/block_states.json` et
   `block_types.json`, mêmes 22 091 états, même ordre : le script s'arrête sinon).

Les profils identiques sont fusionnés (1 341 profils). La table `tools` donne, pour les 36 outils de PMMP,
type, niveau de récolte, efficacité adaptée et non adaptée, durabilité.

**À relancer à chaque changement de palette**, comme `generate_block_registry` : le test
`every_palette_state_has_a_profile` et une assertion au chargement signalent un fichier périmé.

### Propriétés 1.26.50 inconnues de PMMP

`minecraft:corner` (escaliers) et `minecraft:connection_*` (clôtures, vitres, barreaux) sont apparues en
1.26.50 (worldupgrader v1.0.22, `0351_1.26.40_to_1.26.50.27.json`). Ce sont des états de voisinage, sans
effet sur la dureté ni les drops : le script les retire avant de désérialiser, et remet les valeurs par
défaut du schéma (`none`, 0) sur les items blocs rendus.

## 2 bis. Drops observés sur BDS : `data/block_drops_bds.json`

Depuis le 2026-10-26, les drops viennent d'abord de BDS 1.26.52.3 **en fonctionnement** : chaque état est posé
par commande, puis BDS calcule lui-même ses drops avec chaque outil (`loot spawn … mine <bloc> mainhand`, sa
logique de drops sans casse ni durée). PMMP ne sert plus qu'aux états absents de ce relevé. L'XP n'est pas
relevée (`/loot` ne la produit pas) : elle reste celle de PMMP.

Chaîne (scripts dans `scripts/dev/`, relevés dans `.reference/bds/drops/`, hors git) :

```bash
python scripts/dev/bds_palette.py          # vérifie les hashes réseau de la palette contre BDS
python scripts/dev/plan_bds_drops.py --out .reference/bds/drops/plan-states-v3.json
./scripts/dev/bds_capture/bds-capture.exe drops -bds "$(pwd)/.reference/bds/bedrock-server-1.26.52.3-win/bedrock_server.exe" \
    -plan .reference/bds/drops/plan-states-v3.json -out .reference/bds/drops/states-v2.jsonl
python scripts/dev/plan_bds_drops.py --out .reference/bds/drops/plan-random-v3.json --samples 256 \
    --only .reference/bds/drops/states-v2.jsonl
./scripts/dev/bds_capture/bds-capture.exe drops … -plan .reference/bds/drops/plan-random-v3.json -out .reference/bds/drops/random-v3.jsonl
python scripts/dev/build_bds_drops.py --groups .reference/bds/drops/plan-states-v3.groups.json \
    .reference/bds/drops/states-v2.jsonl .reference/bds/drops/random-v3.jsonl
```

1. **Plan** (`plan_bds_drops.py`) : les 22 091 états sont regroupés par nom et par propriétés qui peuvent
   changer les drops ; orientation, connexions, alimentation en redstone, phases d'animation sont ignorées
   (liste `IGNORED`). 2 049 états représentants, 16 outils : main nue, pioches des 5 rangs, hache, pelle, houe
   et épée en bois, masse, cisailles, puis Toucher de soie et Fortune I à III sur une pioche en netherite
   (compatible avec tout bloc). Les blocs techniques (air, liquides, feu, portails, pistons en mouvement)
   sont écartés.
2. **Relevé** (`bds-capture drops`, voir son README) : 16 tirages par état et par outil, soit 524 896 tirages
   en 9 minutes ; puis 256 tirages par outil pour les 148 états des 59 blocs qui se sont montrés aléatoires.
   Un tirage ne compte que si l'état était en place au tick du tirage.
3. **Données** (`build_bds_drops.py`) : pour chaque état, la table de son représentant, outil par outil :
   résultats observés et nombre de tirages. Les items blocs sont rattachés à leur runtime ID par le hash
   réseau de BDS (FNV-1a du NBT de l'état, `bds_palette.py`, vérifié identique pour les 22 091 états).

Côté serveur, `block_break::drop_stacks` choisit l'outil relevé qui représente l'outil tenu
(`bds_tool_key`) : pioche selon son rang, autre outil par sa famille, masse, sinon main nue. Toucher de soie et
Fortune ne comptent que si l'outil est compatible avec le bloc ; ils prennent alors les résultats de la
pioche en netherite enchantée. Un bloc aléatoire tire un résultat selon les fréquences observées.

Constats :

- BDS fait apparaître **une entité par item** quand un bloc rend plusieurs exemplaires (bougies, charbon avec
  Fortune) ; mc-rs fait de même.
- Écarts avec PMMP corrigés par le relevé, entre autres : plantes de deux blocs (tournesol, rosier, lilas,
  pivoine, plante-amphore) rendues par la moitié basse, dalle de chêne pétrifiée qui rend une dalle de chêne,
  nombre de faces de la touffe de résine et du lichen lumineux, herbes et fougères hautes aux cisailles (deux
  herbes ou fougères courtes), œuf de dragon, Fortune III sur le charbon (1 à 4 charbons à 37 / 23 / 20 / 20 %
  sur 272 tirages, conforme à la formule connue des minerais, 40 / 20 / 20 / 20 % ; ~55 % de 1 chez PMMP).
  Gravier : silex dans 11,4 % des cas (10 % attendus).
- Restent sur PMMP (pas de table) : les états que `setblock` refuse (doubles dalles data-driven, chimie,
  verres durcis, blocs obsolètes : 209 noms, listés dans `<relevé>.rejected.json`), ceux qui ne tiennent pas
  seuls (moitié haute des portes, cœur de grinçant, soufre puissant), et ceux dont PMMP lit les drops dans la
  tuile (lits, bannières, boîtes de Shulker, pot de fleurs, cadres, pupitre) : `/loot` ne voit que la tuile
  par défaut d'un bloc posé par commande.

**À refaire à chaque changement de palette ou de version de BDS** : le test `bds_survey_covers_the_palette` et
une assertion au chargement signalent un fichier périmé.

## 3. PMMP ou BDS : décisions

Le script consigne les écarts entre les deux sources dans `pmmp_bds_differences` (dureté, résistance, type
d'outil ; valeurs PMMP puis BDS) et ses choix de niveau de récolte dans `harvest_level_decisions`. Bilan :

| Donnée | Retenue | Écarts constatés |
|---|---|---|
| Dureté | BDS, arrondie à 4 décimales | aucun hors arrondi float32 (béton 1.799999, chemin 0.649999) ; liquides -1 chez PMMP, 100 chez BDS ; `pitcher_crop` -1 chez PMMP, 0 chez BDS |
| Résistance aux explosions | BDS × 5 (échelle de PMMP) | rapport 5 partout, sauf `mangrove_log` (BDS 0,4, PMMP 10) |
| Type d'outil | tags `is_*_item_destructible` de BDS, plus le bit cisailles de PMMP | rails et `big_dripleaf`, `hay_block`, `dried_kelp_block` : outil chez BDS seulement ; `dragon_egg`, `glowstone`, `frosted_ice` : pioche chez PMMP seulement ; laine, herbes, buisson mort : cisailles chez PMMP (BDS les traite par tags de matière, cf. Allay `ItemBaseComponentImpl::isCorrectToolFor`) |
| Outil requis (vitesse) | `requiresCorrectToolForDrops` de BDS | voir ci-dessous |
| Niveau de récolte (drops) | BDS quand il exige un outil (`stone/iron/diamond_tier_destructible`), sinon PMMP s'il modélise les drops | 18 décisions, listées dans `harvest_level_decisions` du fichier |

Niveau de récolte :

- **BDS l'emporte** : barreaux de fer et de cuivre (n'importe quelle pioche, PMMP exige la pierre), tailleur
  de pierre (pioche requise, PMMP non), œuf de dragon et blocs de chimie (récupérables à la main, PMMP exige
  une pioche) ;
- **PMMP est gardé** pour les classes qui modélisent leurs drops par « outil requis » et
  `getDropsForIncompatibleTool` (améthystes, herbes, fougères, buisson mort, racines suspendues, pousses du
  Nether, petite foliole) : BDS ne les déclare pas « outil requis » mais ne les rend pas non plus à la main.
  La vitesse suit alors BDS ; seule l'améthyste (dureté 1,5) en est affectée.

Tout nouvel écart fait échouer le script tant qu'il n'est pas tranché dans `BDS_HARVEST_OVERRIDES` ou
`PMMP_DROP_MODEL_CLASSES`.

## 4. Ce qui manque encore

- **Effets par joueur** : `EffectManager` n'est branché sur aucun joueur ; Célérité, Fatigue de minage et
  Puissance du conduit valent 0 dans le calcul (`MinerState`).
- **Drops aléatoires** : tirés selon les fréquences observées sur BDS (§ 2 bis), pas selon la formule de BDS.
  Les probabilités rares (pomme des feuilles de chêne, 0,5 %) sont mal estimées avec 256 tirages ; la
  formule exacte serait à lire dans le binaire.
- **Drops lus dans la tuile** (bannières, lits, pots de fleurs, cadres, pupitre, boîtes de Shulker) : repli
  sur l'ancienne table `inventory::block_drop`. Le contenu des conteneurs (`Tile::onBlockDestroyed`) n'est
  rendu que pour les fourneaux.
- **États sans relevé de BDS** (§ 2 bis) : doubles dalles data-driven (leur table de butin est dans le pack
  vanilla de BDS, `loot_tables/blocks/`, non encore lue), blocs de chimie, verres durcis, moitiés hautes de
  portes, cœur de grinçant, soufre puissant : données de PMMP ou repli.
- **XP** : celle de PMMP, `/loot` ne la produisant pas.
- **Réactions au clic** (`Block::onAttack` : bloc musical, œuf de dragon, minerai de redstone) : non portées.
- **Usure** : `durability::apply_damage` range les dégâts dans la méta de l'item, alors que PMMP les écrit
  dans la NBT (`Damage`, méta réseau 0). Chantier à part, à vérifier avec le vrai client.
- **Au sol** : mc-rs ne calcule pas les collisions du joueur ; l'état vient du drapeau `VerticalCollision`
  de PlayerAuthInput (PMMP : `Entity::checkGroundState`).
- **Explosions** : `explosion.rs` ignore la résistance ; seule l'incassabilité est lue dans les profils.
- **Vérification dans BDS** (Ghidra) : à faire une fois l'analyse du binaire terminée.

## 5. Tests

`block_break::tests` (temps de cassage, outils, Efficacité, épée, soie, XP des minerais, décisions BDS,
usure ; drops de BDS : couverture de la palette, items connus du registre, outil et rang, Fortune, silex du
gravier, nombre de bougies), `item_extra_data::tests`, `liquids::tests::water_states_follow_liquid_depth` (ordre des états
d'eau vérifié dans la palette), `passive_entities::tests::xp_splits_like_pmmp`.
