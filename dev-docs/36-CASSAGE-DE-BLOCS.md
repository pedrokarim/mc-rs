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
- **Drops aléatoires** (33 classes PMMP : minerais avec Fortune, cultures, feuilles, gravier, herbes,
  glowstone, melon…) : repli sur l'ancienne table `inventory::block_drop`, soumise à la compatibilité de
  l'outil. À porter classe par classe.
- **Drops lus dans la tuile** (bannières, lits, pots de fleurs, cadres, pupitre, boîtes de Shulker) : même
  repli. Le contenu des conteneurs (`Tile::onBlockDestroyed`) n'est rendu que pour les fourneaux.
- **261 blocs inconnus de PMMP** (4 739 états : étagères, ruches, pistons, observateurs, distributeurs,
  sculk, dalles et escaliers data-driven, bois de peuplier, soufre, cinabre…) : dureté et outil de BDS,
  drops par repli. Sources possibles : Allay (implémentations par bloc), dragonfly.
- **Réactions au clic** (`Block::onAttack` : bloc musical, œuf de dragon, minerai de redstone) : non portées.
- **Usure** : `durability::apply_damage` range les dégâts dans la méta de l'item, alors que PMMP les écrit
  dans la NBT (`Damage`, méta réseau 0). Chantier à part, à vérifier avec le vrai client.
- **Au sol** : mc-rs ne calcule pas les collisions du joueur ; l'état vient du drapeau `VerticalCollision`
  de PlayerAuthInput (PMMP : `Entity::checkGroundState`).
- **Explosions** : `explosion.rs` ignore la résistance ; seule l'incassabilité est lue dans les profils.
- **Vérification dans BDS** (Ghidra) : à faire une fois l'analyse du binaire terminée.

## 5. Tests

`block_break::tests` (temps de cassage, outils, Efficacité, épée, soie, XP des minerais, décisions BDS,
usure), `item_extra_data::tests`, `liquids::tests::water_states_follow_liquid_depth` (ordre des états
d'eau vérifié dans la palette), `passive_entities::tests::xp_splits_like_pmmp`.
