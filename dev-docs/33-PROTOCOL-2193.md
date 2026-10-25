# 33 – Protocole 2193 (Bedrock 1.26.50 / 1.26.51)

Montée de version réalisée le 2026-10-25 : protocole **2169 (1.26.45) → 2193 (1.26.51)**. 1.26.51 est un
hotfix de 1.26.50 (« r/26_u5 ») : les schémas Mojang des tags `v1.26.50` et `v1.26.51` ne diffèrent que
par leurs métadonnées. `MINECRAFT_VERSION` vaut donc `1.26.51`, la dernière release.

La procédure et le contexte (fin de support de PocketMine-MP, lecture des schémas JSON Mojang, méthode face
à un écran « Block ») restent ceux de [`31-PROTOCOL-2169.md`](31-PROTOCOL-2169.md).

## 1. Références

| Référence | Emplacement | Rôle |
|---|---|---|
| Schémas JSON Mojang | `.reference/bedrock-protocol-docs`, `git diff v1.26.45 v1.26.51 -- json/` | Format final. **Pas de changelog** `legacy_changelogs/changelog_2193_*` |
| gophertunnel | `git diff v1.61.0 v1.62.0` (v1.61.0 = 2169, v1.62.0 = 2193) | Implémentation Go (dragonfly) |
| CloudburstProtocol | `bedrock-codec/.../codec/v2193/` (`Bedrock_v2193.java`, 13 serializers mis à jour, 2 paquets) | Implémentation Java |
| dragonfly | `v0.11.5` (commit `4c7b5074`, « Support 1.26.50 ») | Données de jeu |
| df-mc/worldupgrader | `v1.0.22`, `blockupgrader/schemas/0351_1.26.40_to_1.26.50.27.json` | Propriétés de blocs ajoutées |
| pmmp/BedrockItemUpgradeSchema | `id_meta_upgrade_schema/0271_*`, `0281_1.26.40_to_1.26.50.json` | Renommages d'items |

Les trois implémentations concordent sur tous les paquets ci-dessous.

## 2. Changements réseau

### 2.1 Appliqués

| Paquet | Changement | Fichier |
|---|---|---|
| PlayerAuthInput | Les cinq optionnels « doubles » de 2168 deviennent **simples** (un seul booléen de présence) ; plus de booléen de présence devant la liste des drapeaux | `mc-rs-proto/src/packets/player.rs`, `mc-rs-sim/src/bedrock.rs` |
| InventoryTransaction | Plus de booléens de présence (imposés à vrai en 2168) devant le type et les actions ; identifiant de fenêtre et drapeaux de source en optionnels simples | `player.rs` |
| UseItem (InventoryTransaction et PlayerAuthInput) | `Hand` (`u8`, 0 main principale, 1 main secondaire) après le slot de barre rapide ; actions en liste simple dans PlayerAuthInput | `player.rs` (`ItemInteractionData::hand`) |
| ItemStackResponse | Conteneurs et net id de slot en optionnels simples | `mc-rs-proto/src/packets/world.rs` |
| BossEvent | Identifiant du joueur retiré (il suivait celui du boss) | `mc-rs-server/src/visuals.rs` |
| PlaySound | `BypassListenerRangeCheck` (booléen) avant le handle, puis `PlaybackPositionSeconds` (`f32` optionnel) | `world.rs` |
| StartGame | Liste `Block Properties` remplie avec les blocs vanilla data-driven (voir § 3) | `world.rs`, `world/data_driven_blocks.rs` |
| SetPlayerFurnaceOptions (351, nouveau) | Ignoré, comme dragonfly | `connection/mod.rs` |

### 2.2 Sans effet sur mc-rs (paquets non envoyés ni lus)

- CameraPresets : `ApplyInheritedStartingRotation` (booléen) puis `StartingRotation` (Vec2 optionnel).
- ClientboundAttributeLayerSync : `NoiseAlignment` (`u8` + VarUInt32) en fin d'attribut d'environnement.
- DebugDrawer : `LineGapHeight` (`f32`) après la couleur de fond des textes.
- DimensionData : hauteur en `MinimumY` + `HeightRange` (étendue, plus maximum) et `DefaultBiome` (chaîne).
- MoveActorDelta : `Ticks` (VarUInt64) en fin de paquet.
- SubChunk : chaque ligne de height map précédée de son nombre de cases (VarUInt32 = 16).
- ServerboundDiagnostics : position (Vec3) et dimension (chaîne) par entité.
- ServerboundPackSettingChange : variante 3 = liste de chaînes.
- RecordStarted (352, nouveau, S→C).
- Énumérations étendues en fin de liste, sans décalage : `DisconnectFailReason` (`Kicked` = 55 et
  `Shutdown` = 68 inchangés), `MapDecoration::Type`. `BuildPlatform` : `Nx` renommé `Nintendo`, même valeur.

## 3. Données de jeu 1.26.50

Toutes viennent de **dragonfly v0.11.5**, identiques au commit `4c7b5074` :

| Fichier (`crates/mc-rs-server/data/`) | Contenu |
|---|---|
| `canonical_block_states.nbt` | 22 091 états, 1 477 blocs (17 499 / 1 379 en 1.26.40), triés par FNV-1 64 du nom comme le client |
| `vanilla_items.nbt` | 2 076 items (1 976) ; 259 runtime IDs ont changé |
| `creative_items.nbt` | 124 groupes, 1 980 entrées |
| `data_driven_blocks.nbt` (nouveau) | 98 définitions de blocs data-driven |

Recettes : bedrock-samples `v1.26.50.4` (`python scripts/import_recipes.py v1.26.50.4`). Schémas d'items :
`python scripts/import_item_upgrade_schema.py` (29 schémas, jusqu'à 0281).

### 3.1 Blocs vanilla data-driven

Les 98 blocs ajoutés (dalles, doubles dalles et escaliers de béton et de laine des 16 couleurs, `red_shrub`,
`shelf_mushroom`) sont définis par composants, comme des blocs personnalisés. Le client ne les ajoute à sa
palette que si le serveur envoie leurs définitions dans la liste `Block Properties` de StartGame (schéma
`ServerBlockProperty.json` : nom, puis définition NBT réseau ; dragonfly `Server.makeBlockEntries`).
Sans elles, la palette du client compte 98 blocs de moins et tous les runtime IDs séquentiels qui suivent le
premier de ces blocs sont décalés. `world/data_driven_blocks.rs` lit le fichier (NBT little-endian
`{blocks: [{name, components}]}`) et prépare la liste une fois, dans l'ordre du fichier.

### 3.2 Propriétés de blocs ajoutées et migration

- 58 blocs (clôtures, vitres, barreaux) gagnent `minecraft:connection_north/east/south/west` (octet 0/1).
- 65 escaliers gagnent `minecraft:corner` (`none`, `inner_left`, `inner_right`, `outer_left`, `outer_right`).

Ces états dépendent des blocs voisins. Le serveur ne les calcule pas encore : il place la valeur par défaut
(0 et `none`), et le client affiche donc des clôtures non connectées et des escaliers droits tant que ce
calcul (dragonfly `StateDeriver`, `world/state_upgrade.go`) n'est pas porté.

`generate_data_migrations` a été réécrit pour la migration :

- une migration **directe** vers la palette courante par palette héritée (`--palette 1.26.20=…`,
  `--palette 1.26.40=…`), au lieu d'écraser la précédente ;
- `--block-schema` applique les `addedProperties` d'un schéma worldupgrader / BedrockBlockUpgradeSchema
  (valeur par défaut ajoutée à l'état hérité qui ne porte pas la propriété) avant l'appariement. Toute autre
  opération du schéma fait échouer la génération ;
- une table d'items par version de sauvegarde joueur (`--items <version>,<libellé>,<premier schéma>=…`).

Résultat : 1.26.40 → 17 499 / 17 499 états appariés exactement (593 complétés par le schéma) ;
1.26.20 → 16 898 / 16 899 (`potent_sulfur` replié sur son premier état, comme avant).

Commande utilisée (sources : BedrockData tags `bedrock-1.26.20` et `bedrock-1.26.0`, dragonfly
`v0.11.4`, worldupgrader `v1.0.22`) :

```text
cargo run -p mc-rs-server --bin generate_data_migrations -- \
  --palette 1.26.20=<BedrockData bedrock-1.26.20 canonical_block_states.nbt> \
  --palette 1.26.40=<dragonfly v0.11.4 server/world/block_states.nbt> \
  --block-schema <worldupgrader v1.0.22 0351_1.26.40_to_1.26.50.27.json> \
  --items 0,1.26.0,261=<BedrockData bedrock-1.26.0 required_item_list.json> \
  --items 1,1.26.40,281=<dragonfly v0.11.4 server/world/vanilla_items.nbt>
```

Sauvegardes joueurs : `CURRENT_PLAYER_DATA_VERSION` = 2. Une sauvegarde v1 (tables 1.26.40) est convertie
par nom d'item puis par les schémas d'upgrade d'items postérieurs à sa table (`item_upgrade::upgrade_since`,
comme PMMP), et ses runtime IDs de blocs par la migration 1.26.40.

Le monde `world_noise` a été converti au premier démarrage (14 280 sous-chunks). Copie d'avant conversion :
`worlds/_backup-pre-2193-20261025/` (monde et `players/`).

## 4. Vérifications

- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` (1 398 tests).
  La toolchain 1.99 apportait trois nouveaux lints sur du code existant (`mc-rs-raknet`, `mc-rs-webui`,
  `main.rs`), corrigés au passage ; `async-trait` passe en 0.1.92 (`double_must_use` dans sa macro).
- `mc-rs-sim --players 3 --behavior active` : 3/3 en jeu, 0 échec ; `--mode probe` : le serveur survit.

## 5. Points à confirmer sur un vrai client 1.26.51

1. **Blocs data-driven** : le client doit accepter les 98 définitions et afficher les bons blocs partout
   (un décalage de palette rendrait faux la plupart des blocs du terrain, pas seulement les nouveaux).
   Placer une dalle de laine depuis le créatif valide aussi l'item et le bloc.
2. **Clôtures et escaliers** : non connectés / droits tant que le voisinage n'est pas calculé (§ 3.2).
3. Les écarts de 2169 encore ouverts ([`31-PROTOCOL-2169.md`](31-PROTOCOL-2169.md) § 4).
